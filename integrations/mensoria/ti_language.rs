//! Fixed Ti action port. ApiSession, identity and ControlGate remain owned by
//! the existing DataFacade; public requests cannot supply actor/tenant/roles.
use super::*;
use ti_lang::{
    artifact::Artifact,
    mensoria::{Command, MensoriaHost, Receipt, SessionBinding},
};

impl DataFacade {
    /// Execute against an artifact selected by the server's deployed registry.
    /// No public router is registered by this optional integration feature.
    pub async fn execute_ti_action(
        &self,
        project: &str,
        environment: &str,
        access_token: &str,
        artifact: Artifact,
        request: &[u8],
    ) -> Result<Receipt> {
        let command = Command::decode(request).map_err(|e| anyhow::anyhow!(e.code))?;
        let session = self.session(project, environment, access_token).await?;
        ensure!(!session.admin, "TI_ACCESS_DENIED");
        let (actor_db, env, snapshot, state, _) = identity(&self.settings, &session).await?;
        ensure!(state == "READY", "ENV_NOT_READY");
        let gate = ControlGate::connect(&self.settings.endpoint, &self.settings.gate_password, env)
            .await?;
        let permit = gate.acquire_read(5000).await?;
        let result = async {
            ensure!(permit.snapshot == snapshot, "ENV_NOT_READY");
            let issuer = issuer(&self.settings, &snapshot.physical_database).await?;
            let binding = SessionBinding {
                scope: serde_json::to_string(&(project, environment, &session.target.database))?,
                database: snapshot.physical_database.clone(),
                incarnation: snapshot.incarnation,
                security_seq: snapshot.security_seq,
                family: session.family.clone(),
                family_generation: session.generation,
                session_epoch: session.session_epoch,
                access_expires: session.expires.min(session.family_expires),
            };
            let host = MensoriaHost::bind(issuer, actor_db, binding, artifact)
                .await
                .map_err(|e| anyhow::anyhow!(e.code))?;
            let receipt = host
                .execute(&command)
                .await
                .map_err(|e| anyhow::anyhow!(e.code))?;
            // A publication failure can occur after commit; never report rollback.
            self.finish_query(&session, &gate, &permit)
                .await
                .map_err(|e| command_barrier::unknown(e, &receipt.operation_id))?;
            Ok(receipt)
        }
        .await;
        if result.is_err() {
            let _ = gate.abandon_read(&permit).await;
        }
        result
    }
}
