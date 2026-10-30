#![cfg(feature = "host-qualification")]
mod support;
use serde_json::json;
use support::*;
use ti_lang::{mensoria::Command, Value};
fn command(f: &Fixture, key: &str, action: &str, id: &str, revision: Option<u64>) -> Command {
    Command {
        key: key.into(),
        incarnation: 1,
        artifact: f.artifact.identity().into(),
        entity: "LeaveRequest".into(),
        action: action.into(),
        record_id: id.into(),
        expected_revision: revision,
        inputs: if action == "Create" {
            [
                ("reason".into(), Value::String("test".into())),
                ("days".into(), Value::Int64("2".into())),
            ]
            .into()
        } else {
            Default::default()
        },
    }
}
#[tokio::test]
async fn native_smoke() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let r = a
        .execute(&command(&f, "create", "Create", "one", None))
        .await;
    assert!(r.is_ok(), "{r:?}");
}
async fn pending(f: &Fixture, id: &str) {
    let a = f.host("alice").await;
    a.execute(&command(f, &format!("{id}-create"), "Create", id, None))
        .await
        .unwrap();
    a.execute(&command(f, &format!("{id}-submit"), "Submit", id, Some(1)))
        .await
        .unwrap();
}
async fn counts(f: &Fixture) -> serde_json::Value {
    sql(&f.admin,"RETURN {records:array::len((SELECT * FROM ti_record)),operations:array::len((SELECT * FROM operation WHERE receipt_class='ti_action')),audits:array::len((SELECT * FROM ti_audit)),outbox:array::len((SELECT * FROM outbox WHERE receipt_class='ti_action'))};",json!({})).await
}
#[tokio::test]
async fn tc_022() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let a = f.host("alice").await;
    let before = counts(&f).await;
    assert!(a
        .execute(&command(&f, "self", "Approve", "one", Some(2)))
        .await
        .is_err());
    assert_eq!(counts(&f).await, before);
}
#[tokio::test]
async fn tc_023() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let b = f.host("bob").await;
    sql(
        &f.admin,
        "UPDATE ti_head SET roles.write=['member'];",
        json!({}),
    )
    .await;
    assert_eq!(
        b.execute(&command(&f, "denied", "Approve", "one", Some(2)))
            .await
            .unwrap_err()
            .code,
        "TI_ACCESS_DENIED"
    );
}
#[tokio::test]
async fn tc_024() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let e = f.host("eve").await;
    assert_eq!(
        e.execute(&command(&f, "hidden", "Submit", "one", Some(2)))
            .await
            .unwrap_err()
            .code,
        "TI_ACCESS_DENIED"
    );
    let mut wrong = command(&f, "inc", "Create", "two", None);
    wrong.incarnation = 2;
    assert_eq!(
        e.execute(&wrong).await.unwrap_err().code,
        "TI_ACCESS_DENIED"
    );
}
#[tokio::test]
async fn tc_025() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let a = f.host("alice").await;
    let b = f.host("bob").await;
    let c1 = command(&f, "withdraw", "Withdraw", "one", Some(2));
    let c2 = command(&f, "approve", "Approve", "one", Some(2));
    let (r1, r2) = tokio::join!(a.execute(&c1), b.execute(&c2));
    assert_eq!(usize::from(r1.is_ok()) + usize::from(r2.is_ok()), 1);
    assert_eq!(counts(&f).await["operations"], 3);
}
#[tokio::test]
async fn tc_026() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    a.execute(&command(&f, "new", "Create", "one", None))
        .await
        .unwrap();
    let mut edit = command(&f, "edit", "EditDraft", "one", Some(1));
    edit.inputs = command(&f, "_", "Create", "one", None).inputs;
    let r = a.execute(&edit).await.unwrap();
    assert_eq!(r.revision, 2);
    assert_eq!(r.candidate.record.state, "Draft");
}
#[tokio::test]
async fn tc_027() {
    let f = Fixture::new().await;
    let db = surrealdb::Surreal::new::<surrealdb::engine::remote::http::Http>(f.endpoint.as_str())
        .await
        .unwrap();
    db.use_ns("baas_p0").use_db("ti_fixture").await.unwrap();
    db.signin(surrealdb::opt::auth::Record {
        namespace: "baas_p0".into(),
        database: "ti_fixture".into(),
        access: "account".into(),
        params: json!({"email":"alice","pass":"actor-fixture"}),
    })
    .await
    .unwrap();
    for text in [
        "CREATE ti_record:evil SET revision=1;",
        "INSERT INTO ti_record {id:ti_record:evil,revision:1};",
        "BEGIN; CREATE ti_record:evil SET revision=1; COMMIT;",
        "UPDATE ti_record SET revision=999;",
        "DELETE ti_record;",
    ] {
        let _ = db.query(text).await;
    }
    assert_eq!(counts(&f).await["records"], 0);
}
#[tokio::test]
async fn tc_028() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    for fault in 1..=3 {
        let before = counts(&f).await;
        let r = a
            .execute_probe(
                &command(
                    &f,
                    &format!("fault-{fault}"),
                    "Create",
                    &format!("record-{fault}"),
                    None,
                ),
                fault,
            )
            .await;
        assert!(r.is_err());
        assert_eq!(counts(&f).await, before);
    }
}
#[tokio::test]
async fn tc_029() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "once", "Create", "one", None);
    let r = a.execute(&c).await.unwrap();
    let before = counts(&f).await;
    let again = a.execute(&c).await.unwrap();
    assert_eq!(
        serde_json::to_value(r).unwrap(),
        serde_json::to_value(again).unwrap()
    );
    assert_eq!(counts(&f).await, before);
}
#[tokio::test]
async fn tc_030() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "once", "Create", "one", None);
    a.execute(&c).await.unwrap();
    for field in 0..4 {
        let mut other = c.clone();
        match field {
            0 => {
                other.inputs.insert("days".into(), Value::Int64("3".into()));
            }
            1 => other.artifact = "other".into(),
            2 => other.entity = "Other".into(),
            _ => other.record_id = "another".into(),
        };
        assert_eq!(
            a.execute(&other).await.unwrap_err().code,
            "TI_IDEMPOTENCY_CONFLICT"
        );
    }
    let submit = command(&f, "submit-once", "Submit", "one", Some(1));
    a.execute(&submit).await.unwrap();
    let mut changed = submit;
    changed.expected_revision = Some(2);
    assert_eq!(
        a.execute(&changed).await.unwrap_err().code,
        "TI_IDEMPOTENCY_CONFLICT"
    );
}

#[tokio::test]
async fn tc_031_client_observation_loss() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "lost", "Create", "one", None);
    assert_eq!(
        a.execute_probe(&c, 4).await.unwrap_err().code,
        "TI_OUTCOME_UNKNOWN"
    );
    assert_eq!(a.execute(&c).await.unwrap().revision, 1);
    assert_eq!(counts(&f).await["operations"], 1);
}
#[tokio::test]
async fn tc_032() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "expired", "Create", "one", None);
    a.execute(&c).await.unwrap();
    a.expire_receipt(&c).await.unwrap();
    assert_eq!(a.execute(&c).await.unwrap_err().code, "TI_RESULT_EXPIRED");
    assert_eq!(counts(&f).await["operations"], 1);
}
#[tokio::test]
async fn tc_033() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "revoke", "Create", "one", None);
    a.execute(&c).await.unwrap();
    sql(
        &f.admin,
        "UPDATE session_family:alice SET active=false;",
        json!({}),
    )
    .await;
    assert_eq!(a.execute(&c).await.unwrap_err().code, "TI_ACCESS_DENIED");
}
#[tokio::test]
async fn tc_034() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let b = f.host("bob").await;
    let r = b
        .execute(&command(&f, "approve", "Approve", "one", Some(2)))
        .await
        .unwrap();
    let events = b.begin_delivery(&r.operation_id, "attempt1").await.unwrap();
    assert_eq!(events.as_array().unwrap().len(), 1);
    b.finish_delivery(
        &r.operation_id,
        "attempt1",
        ti_lang::mensoria::DeliveryOutcome::RetrySafe,
    )
    .await
    .unwrap();
    assert_eq!(
        b.begin_delivery(&r.operation_id, "attempt2").await.unwrap(),
        events
    );
    b.finish_delivery(
        &r.operation_id,
        "attempt2",
        ti_lang::mensoria::DeliveryOutcome::Delivered,
    )
    .await
    .unwrap();
    assert!(b.begin_delivery(&r.operation_id, "attempt3").await.is_err());
    assert_eq!(counts(&f).await["operations"], 3);
}
#[tokio::test]
async fn tc_035() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let b = f.host("bob").await;
    let a = f.host("alice").await;
    let r = b
        .execute(&command(&f, "approve", "Approve", "one", Some(2)))
        .await
        .unwrap();
    assert!(a.begin_delivery(&r.operation_id, "bad").await.is_err());
    b.begin_delivery(&r.operation_id, "unknown").await.unwrap();
    b.finish_delivery(
        &r.operation_id,
        "unknown",
        ti_lang::mensoria::DeliveryOutcome::Unknown,
    )
    .await
    .unwrap();
    assert!(b
        .begin_delivery(&r.operation_id, "blind-replay")
        .await
        .is_err());
}
#[tokio::test]
async fn tc_036_host() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let mut c = command(&f, "stale", "Create", "one", None);
    c.artifact = "old-model".into();
    assert_eq!(a.execute(&c).await.unwrap_err().code, "TI_MODEL_STALE");
}
#[tokio::test]
async fn tc_037() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let incompatible = ti_lang::artifact::Artifact::freeze(
        &include_str!("../examples/approval.ti")
            .replace("version 1", "version 2")
            .replace("days > 0", "days > 100"),
        Default::default(),
    )
    .unwrap();
    sql(
        &f.admin,
        "UPDATE environment_guard:current SET state='CLOSED';",
        json!({}),
    )
    .await;
    assert!(
        ti_lang::mensoria::activate_closed(&f.admin, &incompatible, &f.roles, 1)
            .await
            .is_err()
    );
    let head = sql(
        &f.admin,
        "RETURN (SELECT * FROM ONLY ti_head:leave);",
        json!({}),
    )
    .await;
    assert_eq!(head["artifact"], f.artifact.identity());
    assert_eq!(
        sql(
            &f.admin,
            "RETURN (SELECT VALUE state FROM ONLY environment_guard:current);",
            json!({})
        )
        .await,
        "CLOSED"
    );
}
#[tokio::test]
async fn tc_038() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "old-incarnation", "Create", "one", None);
    a.execute(&c).await.unwrap();
    sql(
        &f.admin,
        "UPDATE environment_guard:current SET incarnation=2;",
        json!({}),
    )
    .await;
    assert_eq!(a.execute(&c).await.unwrap_err().code, "TI_ACCESS_DENIED");
}
#[tokio::test]
async fn tc_039_host() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    a.execute(&command(&f, &"k".repeat(128), "Create", "one", None))
        .await
        .unwrap();
    assert_eq!(
        a.execute(&command(&f, &"k".repeat(129), "Create", "two", None))
            .await
            .unwrap_err()
            .code,
        "TI_INPUT"
    );
}
#[tokio::test]
async fn tc_043() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let e = f.host("eve").await;
    let hidden = e
        .execute(&command(&f, "hidden", "Approve", "one", Some(999)))
        .await
        .unwrap_err();
    let absent = e
        .execute(&command(&f, "absent", "Approve", "absent", Some(999)))
        .await
        .unwrap_err();
    assert_eq!(hidden.code, "TI_ACCESS_DENIED");
    assert_eq!(hidden.code, absent.code);
}
#[tokio::test]
async fn tc_044() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "original", "Create", "one", None);
    let original = a.execute(&c).await.unwrap();
    let next = ti_lang::artifact::Artifact::freeze(
        &include_str!("../examples/approval.ti").replace("version 1", "version 2"),
        Default::default(),
    )
    .unwrap();
    sql(
        &f.admin,
        "UPDATE environment_guard:current SET state='CLOSED';",
        json!({}),
    )
    .await;
    ti_lang::mensoria::activate_closed(&f.admin, &next, &f.roles, 1)
        .await
        .unwrap();
    sql(
        &f.admin,
        "UPDATE environment_guard:current SET state='OPEN';",
        json!({}),
    )
    .await;
    assert_eq!(a.execute(&c).await.unwrap().artifact, original.artifact);
    assert_eq!(counts(&f).await["operations"], 1);
    assert_eq!(
        a.execute(&command(&f, "fresh-old", "Create", "two", None))
            .await
            .unwrap_err()
            .code,
        "TI_MODEL_STALE"
    );
}
#[tokio::test]
async fn tc_031_socket_response_loss_and_cancellation() {
    use std::sync::atomic::Ordering;
    let f = Fixture::new().await;
    let proxy = support::proxy::Proxy::start(f.endpoint.clone());
    let a = f.proxied_host(&proxy.endpoint).await;
    proxy.mode.store(1, Ordering::SeqCst);
    let c = command(&f, "socket-drop", "Create", "one", None);
    assert_eq!(a.execute(&c).await.unwrap_err().code, "TI_OUTCOME_UNKNOWN");
    assert!(proxy.observed.load(Ordering::SeqCst));
    assert_eq!(a.execute(&c).await.unwrap().revision, 1);
    proxy.observed.store(false, Ordering::SeqCst);
    proxy.mode.store(2, Ordering::SeqCst);
    let c = command(&f, "cancelled", "Create", "two", None);
    let (task_host, task_command) = (a.clone(), c.clone());
    let task = tokio::spawn(async move { task_host.execute(&task_command).await });
    for i in 0..200 {
        if proxy.observed.load(Ordering::SeqCst) {
            break;
        }
        assert!(i < 199, "proxy never observed commit response");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    proxy.mode.store(0, Ordering::SeqCst);
    assert_eq!(a.execute(&c).await.unwrap().revision, 1);
    assert_eq!(counts(&f).await["operations"], 2);
}
#[tokio::test]
async fn tc_041_wire_boundary() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "wire", "Create", "one", None);
    let raw = serde_json::to_string(&c).unwrap();
    // A public command accepts standard tagged scalar inputs only.
    let decoded = Command::decode(raw.as_bytes()).unwrap();
    a.execute(&decoded).await.unwrap();
    for invalid in [
        raw.replace(
            "\"days\":",
            "\"days\":{\"type\":\"Int64\",\"value\":\"1\"},\"days\":",
        ),
        raw.replace("\"value\":\"2\"", "\"value\":9007199254740993"),
        raw.replace("\"type\":\"Int64\"", "\"type\":\"UserId\""),
        raw.replacen('{', "{\"actor\":\"bob\",", 1),
    ] {
        match Command::decode(invalid.as_bytes()) {
            Err(_) => {}
            Ok(other) => assert!(a.execute(&other).await.is_err()),
        }
    }
    assert_eq!(counts(&f).await["operations"], 1);
}
#[tokio::test]
async fn tc_042_native_key_order() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "ordered", "Create", "one", None);
    let first = a.execute(&c).await.unwrap();
    let raw = format!(
        r#"{{"key":"ordered","incarnation":1,"artifact":"{}","entity":"LeaveRequest","action":"Create","record_id":"one","expected_revision":null,"inputs":{{"reason":{{"type":"String","value":"test"}},"days":{{"type":"Int64","value":"2"}}}}}}"#,
        f.artifact.identity()
    );
    let second = a
        .execute(&Command::decode(raw.as_bytes()).unwrap())
        .await
        .unwrap();
    assert_eq!(first.operation_id, second.operation_id);
    assert_eq!(counts(&f).await["operations"], 1);
}
#[tokio::test]
async fn tc_028_native_write_failure() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    for table in ["ti_audit", "operation", "outbox"] {
        sql(
            &f.admin,
            &format!("DEFINE FIELD reject_write ON {table} TYPE int VALUE 1 ASSERT $value=2;"),
            json!({}),
        )
        .await;
        let before = counts(&f).await;
        assert!(a
            .execute(&command(
                &f,
                &format!("reject-{table}"),
                "Create",
                table,
                None
            ))
            .await
            .is_err());
        assert_eq!(counts(&f).await, before);
        sql(
            &f.admin,
            &format!("REMOVE FIELD reject_write ON {table};"),
            json!({}),
        )
        .await;
    }
}
#[tokio::test]
async fn tc_033_current_authority() {
    let f = Fixture::new().await;
    let a = f.host("alice").await;
    let c = command(&f, "current", "Create", "one", None);
    a.execute(&c).await.unwrap();
    for (deny, restore) in [
        (
            "UPDATE user:alice SET active=false;",
            "UPDATE user:alice SET active=true;",
        ),
        (
            "UPDATE user:alice SET role='guest';",
            "UPDATE user:alice SET role='member';",
        ),
        (
            "UPDATE user:alice SET session_epoch=1;",
            "UPDATE user:alice SET session_epoch=0;",
        ),
        (
            "UPDATE session_family:alice SET generation=2;",
            "UPDATE session_family:alice SET generation=1;",
        ),
        (
            "UPDATE session_family:alice SET expires_at=time::now()-1s;",
            "UPDATE session_family:alice SET expires_at=time::now()+1h;",
        ),
        (
            "UPDATE environment_guard:current SET applied_security_seq=1;",
            "UPDATE environment_guard:current SET applied_security_seq=0;",
        ),
    ] {
        sql(&f.admin, deny, json!({})).await;
        assert!(a.execute(&c).await.is_err(), "{deny}");
        sql(&f.admin, restore, json!({})).await;
        assert_eq!(a.execute(&c).await.unwrap().revision, 1);
    }
}
#[tokio::test]
async fn tc_024_wrong_native_binding() {
    use ti_lang::mensoria::{MensoriaHost, SessionBinding};
    let f = Fixture::new().await;
    for database in ["wrong_environment", "ti_fixture"] {
        let binding = SessionBinding {
            scope: "project/environment".into(),
            database: database.into(),
            incarnation: 1,
            security_seq: 0,
            family: "alice".into(),
            family_generation: 1,
            session_epoch: 0,
            access_expires: u64::MAX / 2,
        };
        assert!(MensoriaHost::bind(
            f.issuer.clone(),
            f.issuer.clone(),
            binding,
            f.artifact.clone()
        )
        .await
        .is_err());
    }
}
#[tokio::test]
async fn tc_033_result_read_revocation() {
    let f = Fixture::new().await;
    pending(&f, "one").await;
    let b = f.host("bob").await;
    let c = command(&f, "reviewed", "Approve", "one", Some(2));
    b.execute(&c).await.unwrap();
    sql(&f.admin, "UPDATE ti_head SET roles.read=[];", json!({})).await;
    assert_eq!(b.execute(&c).await.unwrap_err().code, "TI_ACCESS_DENIED");
}
