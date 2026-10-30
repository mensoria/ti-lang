use serde_json::{json, Value as Json};
use std::{
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use surrealdb::{
    engine::remote::http::{Client, Http},
    opt::auth::{Record, Root},
    Surreal,
};
use ti_lang::{
    artifact::Artifact,
    mensoria::{self, MensoriaHost, Roles, SessionBinding},
};
pub struct Fixture {
    child: Child,
    pub admin: Surreal<Client>,
    pub issuer: Surreal<Client>,
    pub endpoint: String,
    pub artifact: Artifact,
    pub roles: Roles,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub async fn sql(db: &Surreal<Client>, text: &str, params: Json) -> Json {
    let mut r = db.query(text).bind(params).await.unwrap();
    let errors = r.take_errors();
    assert!(errors.is_empty(), "{errors:?}");
    for i in (0..r.num_statements()).rev() {
        let v = r
            .take::<surrealdb::types::Value>(i)
            .unwrap()
            .into_json_value();
        if !v.is_null() {
            return v;
        }
    }
    Json::Null
}
impl Fixture {
    pub async fn new() -> Self {
        let source = std::env::var_os("MENSORIA_SOURCE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mensoria")
            });
        let binary = std::env::var_os("TI_SURREAL_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../mensoria/prototypes/surrealdb-rust-ai-baas/.p0/bin/surreal")
            });
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let password = format!(
            "fixture-{}-{port}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let mut child = Command::new(binary)
            .args([
                "start",
                "--bind",
                &format!("127.0.0.1:{port}"),
                "--user",
                "ti_fixture",
                "--pass",
                &password,
                "memory",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("native Surreal 3.3.0 required");
        let endpoint = format!("127.0.0.1:{port}");
        let admin = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match Surreal::new::<Http>(endpoint.as_str()).await {
                    Ok(db) => break db,
                    Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                }
            }
        })
        .await
        .unwrap_or_else(|_| {
            let _ = child.kill();
            let _ = child.wait();
            panic!("native server startup deadline");
        });
        let artifact = Artifact::freeze(
            include_str!("../../examples/approval.ti"),
            Default::default(),
        )
        .unwrap();
        let roles = Roles {
            aliases: [
                ("member".into(), vec!["leave.applicant".into()]),
                ("reviewer".into(), vec!["leave.reviewer".into()]),
            ]
            .into(),
            read: vec!["reviewer".into()],
            write: vec!["member".into(), "reviewer".into()],
            events: vec!["reviewer".into()],
        };
        let f = Self {
            child,
            admin,
            issuer: Surreal::new::<Http>(endpoint.as_str()).await.unwrap(),
            endpoint,
            artifact,
            roles,
        };
        for i in 0..100 {
            if f.admin
                .signin(Root {
                    username: "ti_fixture".into(),
                    password: password.clone(),
                })
                .await
                .is_ok()
            {
                break;
            }
            assert!(i < 99, "server failed to start");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        sql(
            &f.admin,
            "DEFINE NAMESPACE baas_p0; USE NS baas_p0; DEFINE DATABASE ti_fixture;",
            json!({}),
        )
        .await;
        f.admin
            .use_ns("baas_p0")
            .use_db("ti_fixture")
            .await
            .unwrap();
        assert_eq!(f.admin.version().await.unwrap().to_string(), "3.3.0");
        for file in [
            "001-p0.surql",
            "003-command-bridge.surql",
            "004-data.surql",
            "005-data-concurrency.surql",
            "006-auth-lifecycle.surql",
        ] {
            let path = if source.join("migrations").is_dir() {
                source.join("migrations").join(file)
            } else {
                source.join(file)
            };
            let bytes = std::fs::read(path).unwrap();
            let manifest: Json =
                serde_json::from_str(include_str!("../fixtures/mensoria/manifest.json")).unwrap();
            use sha2::{Digest, Sha256};
            assert_eq!(
                format!("{:x}", Sha256::digest(&bytes)),
                manifest["sha256"][file],
                "unqualified migration source drift"
            );
            sql(&f.admin, std::str::from_utf8(&bytes).unwrap(), json!({})).await;
        }
        sql(&f.admin,"UPDATE environment_guard:current SET state='CLOSED'; CREATE data_issuer:ti SET environment='ti_fixture',pass=crypto::argon2::generate('issuer-fixture'); DEFINE FIELD session_epoch ON user TYPE int DEFAULT 0; CREATE user:alice SET email='alice',pass=crypto::argon2::generate('actor-fixture'),role='member'; CREATE user:bob SET email='bob',pass=crypto::argon2::generate('actor-fixture'),role='reviewer'; CREATE user:eve SET email='eve',pass=crypto::argon2::generate('actor-fixture'),role='member';",json!({})).await;

        mensoria::install(&f.admin).await.unwrap();
        mensoria::activate_closed(&f.admin, &f.artifact, &f.roles, 0)
            .await
            .unwrap();
        sql(
            &f.admin,
            "UPDATE environment_guard:current SET state='OPEN';",
            json!({}),
        )
        .await;
        f.issuer
            .use_ns("baas_p0")
            .use_db("ti_fixture")
            .await
            .unwrap();
        f.issuer
            .signin(Record {
                namespace: "baas_p0".into(),
                database: "ti_fixture".into(),
                access: "data_issuer_access".into(),
                params: json!({"environment":"ti_fixture","password":"issuer-fixture"}),
            })
            .await
            .unwrap();
        f
    }
    pub async fn host(&self, who: &str) -> MensoriaHost {
        let db = Surreal::new::<Http>(self.endpoint.as_str()).await.unwrap();
        db.use_ns("baas_p0").use_db("ti_fixture").await.unwrap();
        db.signin(Record {
            namespace: "baas_p0".into(),
            database: "ti_fixture".into(),
            access: "account".into(),
            params: json!({"email":who,"pass":"actor-fixture"}),
        })
        .await
        .unwrap();
        sql(&self.admin,"UPSERT type::record('session_family',$who) SET issued_by=data_issuer:ti,active=true,generation=1,revision=0,refresh_hash='fixture',expires_at=time::now()+1h,session={};",json!({"who":who})).await;
        MensoriaHost::bind(
            self.issuer.clone(),
            db,
            SessionBinding {
                scope: "project/environment".into(),
                database: "ti_fixture".into(),
                incarnation: 1,
                security_seq: 0,
                family: who.into(),
                family_generation: 1,
                session_epoch: 0,
                access_expires: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3600,
            },
            self.artifact.clone(),
        )
        .await
        .unwrap()
    }
}
pub mod proxy;
impl Fixture {
    pub async fn proxied_host(&self, endpoint: &str) -> MensoriaHost {
        let original = self.host("alice").await;
        let issuer = Surreal::new::<Http>(endpoint).await.unwrap();
        issuer.use_ns("baas_p0").use_db("ti_fixture").await.unwrap();
        issuer
            .signin(Record {
                namespace: "baas_p0".into(),
                database: "ti_fixture".into(),
                access: "data_issuer_access".into(),
                params: json!({"environment":"ti_fixture","password":"issuer-fixture"}),
            })
            .await
            .unwrap();
        // Trusted fixture sessions are created by host(); only the issuer transport changes.
        let db = Surreal::new::<Http>(self.endpoint.as_str()).await.unwrap();
        db.use_ns("baas_p0").use_db("ti_fixture").await.unwrap();
        db.signin(Record {
            namespace: "baas_p0".into(),
            database: "ti_fixture".into(),
            access: "account".into(),
            params: json!({"email":"alice","pass":"actor-fixture"}),
        })
        .await
        .unwrap();
        drop(original);
        MensoriaHost::bind(
            issuer,
            db,
            SessionBinding {
                scope: "project/environment".into(),
                database: "ti_fixture".into(),
                incarnation: 1,
                security_seq: 0,
                family: "alice".into(),
                family_generation: 1,
                session_epoch: 0,
                access_expires: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3600,
            },
            self.artifact.clone(),
        )
        .await
        .unwrap()
    }
}
