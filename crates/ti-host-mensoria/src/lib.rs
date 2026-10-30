//! Mensoria environment-DB transaction host. Runtime connections are record-bound
//! actor and environment-scoped data_issuer sessions; never root credentials.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use surrealdb::{engine::remote::http::Client, Surreal};
use ti_artifact::Artifact;
use ti_language::{wire, Actor, Candidate, Diagnostic, Record, Result, Value};

fn failure(code: &str) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        message: code.into(),
        line: 0,
        column: 0,
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Pins the Rust host, fixed transactions and exact dependency resolution.
pub fn implementation_fingerprint() -> String {
    let mut bytes = vec![];
    for source in [
        include_str!("lib.rs"),
        include_str!("mensoria_transaction.surql"),
        include_str!("mensoria_delivery.surql"),
        include_str!("mensoria_delivery_finish.surql"),
        include_str!("../../../Cargo.lock"),
    ] {
        bytes.extend(wire::frame(b'h', source.as_bytes()).expect("bounded host source"));
    }
    hash(&bytes)
}
pub(crate) async fn query(db: &Surreal<Client>, sql: &str, params: Json) -> Result<Json> {
    let mut response = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        db.query(sql).bind(params),
    )
    .await
    .map_err(|_| failure("TI_OUTCOME_UNKNOWN"))?
    .map_err(|_| failure("TI_OUTCOME_UNKNOWN"))?;
    let errors: Vec<_> = response.take_errors().into_iter().collect();
    if !errors.is_empty() {
        // Only a complete fixed-transaction abort can establish a rejection.
        let complete = sql.trim().starts_with("BEGIN;")
            && sql.trim().ends_with("COMMIT;")
            && errors.len() == response.num_statements().saturating_sub(1);
        let thrown = errors.iter().find(|(_, e)| e.is_thrown());
        if complete {
            if let Some((_, e)) = thrown {
                let message = e.to_string();
                for code in [
                    "TI_ACCESS_DENIED",
                    "TI_REVISION_CONFLICT",
                    "TI_MODEL_STALE",
                    "TI_IDEMPOTENCY_CONFLICT",
                    "TI_LIMIT",
                    "TI_RESULT_EXPIRED",
                    "TI_TEST_ABORT",
                ] {
                    if message.contains(code) {
                        return Err(failure(code));
                    }
                }
            }
        }
        return Err(failure("TI_OUTCOME_UNKNOWN"));
    }
    for i in (0..response.num_statements()).rev() {
        let value = response
            .take::<surrealdb::types::Value>(i)
            .map_err(|_| failure("TI_OUTCOME_UNKNOWN"))?
            .into_json_value();
        if !value.is_null() {
            return Ok(value);
        }
    }
    Ok(Json::Null)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub key: String,
    pub incarnation: u64,
    pub artifact: String,
    pub entity: String,
    pub action: String,
    pub record_id: String,
    pub expected_revision: Option<u64>,
    pub inputs: BTreeMap<String, Value>,
}
impl Command {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut json = wire::decode_command_json(bytes)?;
        let inputs = wire::decode_inputs(
            &serde_json::to_vec(&json["inputs"]).map_err(|_| failure("TI_INPUT"))?,
        )?;
        json["inputs"] = json!({});
        let mut command: Self = serde_json::from_value(json).map_err(|_| failure("TI_INPUT"))?;
        command.inputs = inputs;
        Ok(command)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub operation_id: String,
    pub artifact: String,
    pub revision: u64,
    pub host_fingerprint: String,
    pub candidate: Candidate,
}
/// Construct only from the existing verified ApiSession/SafetySnapshot bridge.
/// These are host bindings, never fields decoded from a public action request.
#[derive(Clone)]
pub struct SessionBinding {
    pub scope: String,
    pub database: String,
    pub incarnation: u64,
    pub security_seq: u64,
    pub family: String,
    pub family_generation: u64,
    pub session_epoch: u64,
    pub access_expires: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Roles {
    pub aliases: BTreeMap<String, Vec<String>>,
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub events: Vec<String>,
}
#[derive(Clone)]
pub struct MensoriaHost {
    issuer: Surreal<Client>,
    actor_db: Surreal<Client>,
    binding: SessionBinding,
    artifact: Artifact,
}
struct Context {
    actor: Actor,
    head: Json,
    role: String,
}
impl MensoriaHost {
    pub async fn bind(
        issuer: Surreal<Client>,
        actor_db: Surreal<Client>,
        binding: SessionBinding,
        artifact: Artifact,
    ) -> Result<Self> {
        let h = Self {
            issuer,
            actor_db,
            binding,
            artifact,
        };
        h.context().await?;
        Ok(h)
    }
    async fn context(&self) -> Result<Context> {
        let issuer=query(&self.issuer,"RETURN {kind:$auth.kind, environment:$auth.environment, db:session::db(), ns:session::ns()};",json!({})).await?;
        if issuer["kind"] != "data_issuer"
            || issuer["environment"] != self.binding.database
            || issuer["db"] != self.binding.database
            || issuer["ns"] != "baas_p0"
        {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        let actor=query(&self.actor_db,"RETURN {id:record::id($auth.id),table:record::tb($auth.id),active:$auth.active,role:$auth.role,epoch:$auth.session_epoch,db:session::db(),ns:session::ns()};",json!({})).await?;
        if actor["table"] != "user"
            || actor["active"] != true
            || actor["db"] != self.binding.database
            || actor["ns"] != "baas_p0"
            || actor["epoch"].as_u64().unwrap_or(0) != self.binding.session_epoch
        {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        let a = actor["id"]
            .as_str()
            .ok_or_else(|| failure("TI_ACCESS_DENIED"))?;
        let role = actor["role"]
            .as_str()
            .ok_or_else(|| failure("TI_ACCESS_DENIED"))?
            .to_owned();
        let state=query(&self.issuer,"RETURN {guard:(SELECT * FROM ONLY environment_guard:current),family:(SELECT * FROM ONLY type::record('session_family',$family)),unexpired:time::now()<time::from_secs($expiry),family_unexpired:(SELECT VALUE expires_at>time::now() FROM ONLY type::record('session_family',$family)),head:(SELECT * FROM ONLY type::record('ti_head',$module))};",json!({"family":self.binding.family,"expiry":self.binding.access_expires,"module":self.artifact.manifest().module})).await?;
        let g = &state["guard"];
        let f = &state["family"];
        if g["state"] != "OPEN"
            || g["incarnation"] != self.binding.incarnation
            || g["applied_security_seq"] != self.binding.security_seq
            || state["unexpired"] != true
            || state["family_unexpired"] != true
            || f["active"] != true
            || f["generation"] != self.binding.family_generation
        {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        let roles: Roles = serde_json::from_value(state["head"]["roles"].clone())
            .map_err(|_| failure("TI_MODEL_STALE"))?;
        let aliases: BTreeSet<_> = roles
            .aliases
            .get(&role)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        Ok(Context {
            actor: Actor {
                id: a.into(),
                roles: aliases,
            },
            head: state["head"].clone(),
            role,
        })
    }
    fn params(&self, context: &Context) -> Json {
        json!({"actor":context.actor.id,"expiry":self.binding.access_expires,"role":context.role,"family":self.binding.family,"generation":self.binding.family_generation,"epoch":self.binding.session_epoch,"inc":self.binding.incarnation,"seq":self.binding.security_seq,"module":self.artifact.manifest().module})
    }
    fn operation(&self, c: &Command, actor: &str) -> Result<String> {
        Ok(hash(
            &serde_json::to_vec(&json!([
                "ti.operation/0.1",
                "mensoria.end_user/1",
                self.binding.scope,
                self.binding.incarnation,
                actor,
                c.key
            ]))
            .map_err(|_| failure("TI_INPUT"))?,
        ))
    }
    fn record_key(&self, c: &Command) -> String {
        hash(
            &serde_json::to_vec(&(
                self.artifact.manifest().module.as_str(),
                &c.entity,
                &c.record_id,
            ))
            .expect("string tuple"),
        )
    }
    pub async fn execute(&self, c: &Command) -> Result<Receipt> {
        self.execute_inner(c, 0).await
    }
    #[cfg(feature = "host-qualification")]
    pub async fn execute_probe(&self, c: &Command, fault: u8) -> Result<Receipt> {
        self.execute_inner(c, fault).await
    }
    async fn execute_inner(&self, c: &Command, fault: u8) -> Result<Receipt> {
        if c.inputs.iter().any(|(name, value)| {
            name.len() > 63
                || !name.is_ascii()
                || match value {
                    Value::String(s) => s.len() > 16384,
                    Value::UserId(s) => s.is_empty() || s.len() > 256,
                    Value::Id(v) => v.id.is_empty() || v.id.len() > 256 || v.entity.len() > 63,
                    Value::Int64(s) => s.len() > 20,
                    _ => false,
                }
        }) {
            return Err(failure("TI_INPUT"));
        }
        if c.inputs.len() > 64 {
            return Err(failure("TI_LIMIT"));
        }
        let encoded: std::result::Result<BTreeMap<_, _>, _> = c
            .inputs
            .iter()
            .map(|(k, v)| wire::encode_value(v).map(|v| (k, v)))
            .collect();
        wire::decode_inputs(&serde_json::to_vec(&encoded?).map_err(|_| failure("TI_INPUT"))?)?;
        if c.key.is_empty()
            || c.key.len() > 128
            || !c
                .key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            || c.record_id.is_empty()
            || c.record_id.len() > 256
        {
            return Err(failure("TI_INPUT"));
        }
        let context = self.context().await?;
        if c.incarnation != self.binding.incarnation {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        let roles: Roles = serde_json::from_value(context.head["roles"].clone())
            .map_err(|_| failure("TI_MODEL_STALE"))?;
        if !roles.write.contains(&context.role) {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        let op = self.operation(c, &context.actor.id)?;
        let revision = c.expected_revision.map(|x| x.to_string());
        let record = if c.action == "Create" {
            None
        } else {
            Some(c.record_id.as_str())
        };
        if c.expected_revision
            .is_some_and(|n| n == 0 || n > i64::MAX as u64)
        {
            return Err(failure("TI_INPUT"));
        }
        let fingerprint = wire::fingerprint(
            &wire::FingerprintContext {
                scope: &self.binding.scope,
                incarnation: &self.binding.incarnation.to_string(),
                identity_domain: "mensoria.end_user/1",
                principal: &context.actor.id,
                module: &self.artifact.manifest().module,
                version: self.artifact.manifest().version as u64,
                artifact: &c.artifact,
                entity: &c.entity,
                action: &c.action,
                record,
                revision: revision.as_deref(),
            },
            &c.inputs,
        )?;
        let digest = fingerprint;
        let mut params = self.params(&context);
        params["op"] = op.clone().into();
        params["digest"] = digest.clone().into();
        params["rid"] = self.record_key(c).into();
        let prior = query(
            &self.issuer,
            "RETURN (SELECT * FROM ONLY type::record('operation',$op));",
            params.clone(),
        )
        .await?;
        if !prior.is_null() {
            return self.prior(&context, &prior, &digest, &c.record_id).await;
        }
        if context.head["host_fingerprint"] != implementation_fingerprint()
            || context.head["artifact"] != c.artifact
            || c.artifact != self.artifact.identity()
        {
            return Err(failure("TI_MODEL_STALE"));
        }
        let old = query(
            &self.issuer,
            "RETURN (SELECT * FROM ONLY type::record('ti_record',$rid));",
            params.clone(),
        )
        .await?;
        if !old.is_null() && !roles.read.contains(&context.role) && old["owner"] != context.actor.id
        {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        if c.action != "Create" && old.is_null() {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        if c.action == "Create" && !old.is_null() {
            return Err(failure("TI_REVISION_CONFLICT"));
        }
        if c.action != "Create" && old["revision"].as_u64() != c.expected_revision {
            return Err(failure("TI_REVISION_CONFLICT"));
        }
        let old_record = if old.is_null() {
            None
        } else {
            Some(
                serde_json::from_value::<Record>(old["record"].clone())
                    .map_err(|_| failure("TI_RECORD_INVALID"))?,
            )
        };
        let candidate = self.artifact.compiled().evaluate(
            &c.entity,
            &c.action,
            &context.actor,
            &c.inputs,
            old_record.as_ref(),
            &c.record_id,
        )?;
        let rev = c
            .expected_revision
            .unwrap_or(0)
            .checked_add(1)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or_else(|| failure("TI_LIMIT"))?;
        let receipt = Receipt {
            operation_id: op.clone(),
            artifact: c.artifact.clone(),
            revision: rev,
            host_fingerprint: implementation_fingerprint(),
            candidate,
        };
        params["artifact"] = c.artifact.clone().into();
        params["head_revision"] = context.head["revision"].clone();
        params["expected"] = c.expected_revision.into();
        params["receipt"] = serde_json::to_value(&receipt).map_err(|_| failure("TI_INPUT"))?;
        params["owner"] = if old.is_null() {
            context.actor.id.clone().into()
        } else {
            old["owner"].clone()
        };
        params["fault"] = fault.into();
        params["entity"] = c.entity.clone().into();
        params["scope"] = self.binding.scope.clone().into();
        params["action"] = c.action.clone().into();
        params["logical_id"] = c.record_id.clone().into();
        params["envelopes"] = Json::Array(receipt.candidate.events.iter().enumerate().map(|(ordinal,event)|json!({"event_id":hash(format!("{op}:{ordinal}").as_bytes()),"operation_id":op,"ordinal":ordinal,"scope":self.binding.scope,"incarnation":self.binding.incarnation,"identity_domain":"mensoria.end_user/1","target_id":c.record_id,"event_type":event.name,"artifact":c.artifact,"revision":rev,"event":wire::encode_event(event).expect("validated event")})).collect());
        let result = query(
            &self.issuer,
            include_str!("mensoria_transaction.surql"),
            params,
        )
        .await;
        if fault == 4 && result.is_ok() {
            return Err(failure("TI_OUTCOME_UNKNOWN"));
        }
        result?;
        let after = self.context().await?;
        let prior = query(
            &self.issuer,
            "RETURN (SELECT * FROM ONLY type::record('operation',$op));",
            json!({"op":op}),
        )
        .await?;
        self.prior(&after, &prior, &digest, &c.record_id).await
    }
    async fn prior(
        &self,
        _context: &Context,
        prior: &Json,
        digest: &str,
        target: &str,
    ) -> Result<Receipt> {
        let context = self.context().await?;
        if prior["receipt_class"] != "ti_action" || prior["actor"] != context.actor.id {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        let roles: Roles = serde_json::from_value(context.head["roles"].clone())
            .map_err(|_| failure("TI_ACCESS_DENIED"))?;
        if !roles.write.contains(&context.role)
            || (!roles.read.contains(&context.role) && prior["owner"] != context.actor.id)
        {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        if prior["input_digest"] != digest || prior["target_id"] != target {
            return Err(failure("TI_IDEMPOTENCY_CONFLICT"));
        }
        if prior["expired"] == true {
            return Err(failure("TI_RESULT_EXPIRED"));
        }
        serde_json::from_value(prior["receipt"].clone()).map_err(|_| failure("TI_OUTCOME_UNKNOWN"))
    }
}

/// Event adapters must declare whether replay with the stable event IDs is safe.
#[derive(Clone, Copy)]
pub enum DeliveryOutcome {
    Delivered,
    RetrySafe,
    Unknown,
}
impl MensoriaHost {
    /// Begin a durable delivery attempt. An unfinished attempt is UNKNOWN and
    /// cannot be taken again automatically. Payloads are disclosed only to the
    /// currently authorized native event role.
    pub async fn begin_delivery(&self, operation: &str, attempt: &str) -> Result<Json> {
        let ctx = self.context().await?;
        let roles: Roles = serde_json::from_value(ctx.head["roles"].clone())
            .map_err(|_| failure("TI_ACCESS_DENIED"))?;
        if !roles.events.contains(&ctx.role) || attempt.is_empty() || attempt.len() > 128 {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        let mut params = self.params(&ctx);
        params["op"] = operation.into();
        params["attempt"] = attempt.into();
        query(
            &self.issuer,
            include_str!("mensoria_delivery.surql"),
            params,
        )
        .await?;
        let row = query(
            &self.issuer,
            "RETURN (SELECT * FROM ONLY type::record('outbox',$op));",
            json!({"op":operation}),
        )
        .await?;
        let current = self.context().await?;
        let roles: Roles = serde_json::from_value(current.head["roles"].clone())
            .map_err(|_| failure("TI_ACCESS_DENIED"))?;
        if !roles.events.contains(&current.role) {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        Ok(row["envelopes"].clone())
    }
    pub async fn finish_delivery(
        &self,
        operation: &str,
        attempt: &str,
        outcome: DeliveryOutcome,
    ) -> Result<()> {
        let ctx = self.context().await?;
        let roles: Roles = serde_json::from_value(ctx.head["roles"].clone())
            .map_err(|_| failure("TI_ACCESS_DENIED"))?;
        if !roles.events.contains(&ctx.role) {
            return Err(failure("TI_ACCESS_DENIED"));
        }
        let state = match outcome {
            DeliveryOutcome::Delivered => "DELIVERED",
            DeliveryOutcome::RetrySafe => "PENDING",
            DeliveryOutcome::Unknown => "UNKNOWN",
        };
        let mut params = self.params(&ctx);
        params["op"] = operation.into();
        params["attempt"] = attempt.into();
        params["state"] = state.into();
        query(
            &self.issuer,
            include_str!("mensoria_delivery_finish.surql"),
            params,
        )
        .await?;
        Ok(())
    }
    pub async fn expire_receipt(&self, command: &Command) -> Result<()> {
        let ctx = self.context().await?;
        let op = self.operation(command, &ctx.actor.id)?;
        // The operation tombstone and fingerprint survive payload retention.
        query(&self.issuer,"BEGIN; LET $o=SELECT * FROM ONLY type::record('operation',$op); IF $o.receipt_class!='ti_action' OR $o.actor!=$actor {THROW 'TI_ACCESS_DENIED';}; UPDATE type::record('operation',$op) SET expired=true,receipt=NONE; COMMIT;",json!({"op":op,"actor":ctx.actor.id})).await?;
        Ok(())
    }
}

/// Explicit maintenance boundary. The caller must hold the existing Mensoria
/// maintenance lease and closed admission; this never runs on an Action API path.
pub async fn install(admin: &Surreal<Client>) -> Result<()> {
    let guard = query(
        admin,
        "RETURN (SELECT * FROM ONLY environment_guard:current);",
        json!({}),
    )
    .await?;
    if guard["state"] != "CLOSED" {
        return Err(failure("TI_ACCESS_DENIED"));
    }
    query(admin,"DEFINE FUNCTION OVERWRITE fn::ti_service() {RETURN $auth.kind='data_issuer' AND $auth.environment=session::db();} PERMISSIONS FULL;",json!({})).await?;
    let info = query(admin, "INFO FOR DB STRUCTURE;", json!({})).await?;
    let tables = info["tables"]
        .as_array()
        .ok_or_else(|| failure("TI_MODEL"))?;
    for name in ["operation", "outbox", "environment_guard", "user"] {
        let table = tables
            .iter()
            .find(|t| t["name"] == name)
            .ok_or_else(|| failure("TI_MODEL"))?;
        if !(table["kind"]["kind"] == "ANY" || table["kind"]["kind"] == "NORMAL")
            || table["drop"] != false
        {
            return Err(failure("TI_MODEL"));
        }
        let permissions = &table["permissions"];
        let expression = |key: &str| -> Result<String> {
            let p = &permissions[key];
            if let Some(b) = p.as_bool() {
                Ok(b.to_string())
            } else {
                p.as_str()
                    .map(String::from)
                    .ok_or_else(|| failure("TI_MODEL"))
            }
        };
        let mut clauses = vec![];
        for action in ["select", "create", "update", "delete"] {
            let old = expression(action)?;
            let add = match (name, action) {
                ("user", "select") => "fn::ti_service()",
                ("environment_guard", "update") => "fn::ti_service() AND state='OPEN'",
                ("operation", "select" | "create" | "update")
                | ("outbox", "select" | "create" | "update") => {
                    "fn::ti_service() AND receipt_class='ti_action'"
                }
                _ => "false",
            };
            clauses.push(format!("FOR {action} WHERE ({old}) OR ({add})"));
        }
        let kind = if table["schemafull"] == true {
            "SCHEMAFULL"
        } else {
            "SCHEMALESS"
        };
        // Names and predicates are trusted database metadata, never source/request text.
        query(
            admin,
            &format!(
                "DEFINE TABLE OVERWRITE {name} TYPE {} {kind} PERMISSIONS {};",
                if table["kind"]["kind"] == "NORMAL" {
                    "NORMAL"
                } else {
                    "ANY"
                },
                clauses.join(" ")
            ),
            json!({}),
        )
        .await?;
    }
    query(admin,"DEFINE TABLE ti_record SCHEMALESS PERMISSIONS FOR select,create,update WHERE fn::ti_service(); DEFINE TABLE ti_audit SCHEMALESS PERMISSIONS FOR select,create WHERE fn::ti_service(); DEFINE TABLE ti_artifact SCHEMALESS PERMISSIONS FOR select WHERE fn::ti_service(); DEFINE TABLE ti_head SCHEMALESS PERMISSIONS FOR select WHERE fn::ti_service();",json!({})).await?;
    Ok(())
}
/// Maintain the frozen artifact under a closed environment guard. On failure the
/// caller must keep admission closed; reopening is owned by SafetyManager.
pub async fn activate_closed(
    admin: &Surreal<Client>,
    artifact: &Artifact,
    roles: &Roles,
    expected_head_revision: u64,
) -> Result<()> {
    if roles.aliases.len() > 256
        || roles.aliases.iter().any(|(native, aliases)| {
            native.is_empty()
                || native.len() > 128
                || aliases.len() > 256
                || aliases
                    .iter()
                    .any(|a| a.is_empty() || a.len() > 128 || !a.is_ascii())
        })
        || [&roles.read, &roles.write, &roles.events]
            .iter()
            .any(|r| r.len() > 256)
    {
        return Err(failure("TI_LIMIT"));
    }
    let module = &artifact.manifest().module;
    let guard = query(
        admin,
        "RETURN (SELECT * FROM ONLY environment_guard:current);",
        json!({}),
    )
    .await?;
    if guard["state"] != "CLOSED" {
        return Err(failure("TI_ACCESS_DENIED"));
    }
    // Bounded maintenance sweep: at most 1,000 managed records, one record per
    // page. Larger environments require a separate qualified migration runner.
    for offset in 0..=1000 {
        let stored=query(admin,"RETURN (SELECT * FROM ti_record WHERE module=$module ORDER BY id LIMIT 1 START $offset);",json!({"module":module,"offset":offset})).await?;
        let rows = stored
            .as_array()
            .ok_or_else(|| failure("TI_RECORD_INVALID"))?;
        if rows.is_empty() {
            break;
        }
        if offset == 1000 {
            return Err(failure("TI_LIMIT"));
        }
        for row in rows {
            let entity = row["entity"]
                .as_str()
                .ok_or_else(|| failure("TI_RECORD_INVALID"))?;
            let record: Record = serde_json::from_value(row["record"].clone())
                .map_err(|_| failure("TI_RECORD_INVALID"))?;
            artifact.compiled().validate_record(entity, &record)?;
        }
    }
    let key = hash(format!("{}:{}", module, artifact.manifest().version).as_bytes());
    let sql="BEGIN; LET $g=SELECT * FROM ONLY environment_guard:current; IF $g.state!='CLOSED' OR $g.revision!=$guard_revision {THROW 'TI_ACCESS_DENIED';}; LET $h=SELECT * FROM ONLY type::record('ti_head',$module); IF ($h.revision??0)!=$expected {THROW 'TI_MODEL_STALE';}; LET $old=SELECT * FROM ONLY type::record('ti_artifact',$key); IF $old!=NONE AND ($old.artifact!=$artifact OR $old.roles!=$roles) {THROW 'TI_MODEL_STALE';}; IF $old=NONE {CREATE type::record('ti_artifact',$key) SET artifact=$artifact,manifest=$manifest,roles=$roles;}; UPSERT type::record('ti_head',$module) SET artifact=$artifact,revision=$expected+1,roles=$roles,host_fingerprint=$host_fingerprint; UPDATE environment_guard:current SET revision+=1; COMMIT;";
    query(admin,sql,json!({"module":module,"key":key,"artifact":artifact.identity(),"manifest":artifact.manifest(),"roles":roles,"host_fingerprint":implementation_fingerprint(),"expected":expected_head_revision,"guard_revision":guard["revision"]})).await?;
    Ok(())
}
