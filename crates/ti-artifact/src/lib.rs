//! Immutable model artifacts. Publication/activation persistence belongs to the host.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap as Map;
use std::sync::Arc;
use ti_language::{compile, Compiled, Diagnostic, Result};
#[derive(Debug, Clone, Serialize)]
pub struct Manifest {
    pub edition: String,
    pub profile: String,
    pub module: String,
    pub version: i64,
    pub source_sha256: String,
    pub ir_sha256: String,
    pub compiler_fingerprint: String,
    pub dependencies: Map<String, String>,
    pub limits: Map<String, u64>,
    pub required_capabilities: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct Artifact {
    compiled: Arc<Compiled>,
    manifest: Manifest,
    identity: String,
}
impl Artifact {
    pub fn freeze(source: &str, dependencies: Map<String, String>) -> Result<Self> {
        let compiled = compile(source)?;
        if dependencies
            .iter()
            .any(|(k, v)| k.is_empty() || v.is_empty())
        {
            return err("TI_MODEL", "empty dependency binding");
        }
        let mut compiler = Sha256::new();
        compiler.update(b"ti.compiler/0.1\0");
        for source in ti_language::implementation_sources()
            .iter()
            .copied()
            .chain([include_str!("lib.rs"), include_str!("../../../Cargo.lock")])
        {
            compiler.update((source.len() as u64).to_be_bytes());
            compiler.update(source.as_bytes());
        }
        let ir = compiled.ir_bytes()?;
        let manifest = Manifest {
            edition: compiled.edition.into(),
            profile: compiled.profile.into(),
            module: compiled.module_name().into(),
            version: compiled.model_version(),
            source_sha256: compiled.source_sha256.clone(),
            ir_sha256: format!("{:x}", Sha256::digest(&ir)),
            compiler_fingerprint: format!("{:x}", compiler.finalize()),
            dependencies,
            limits: [
                ("source_bytes", 262144),
                ("identifier_bytes", 63),
                ("string_bytes", 16384),
                ("opaque_id_bytes", 256),
                ("expression_depth", 64),
                ("declarations", 256),
                ("fields", 64),
                ("inputs", 64),
                ("states", 64),
                ("requirements", 64),
                ("invariants", 64),
                ("emissions", 16),
                ("evaluation_nodes", 100000),
                ("input_bytes", 131072),
                ("event_bytes", 131072),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
            required_capabilities: vec![
                "trusted-actor".into(),
                "revision-cas".into(),
                "atomic-record-operation-audit-outbox".into(),
                "original-key-reconciliation".into(),
                "managed-write-fence".into(),
            ],
        };
        let bytes = serde_json::to_vec(&manifest).map_err(|_| Diagnostic {
            code: "TI_MODEL".into(),
            message: "manifest serialization failed".into(),
            line: 0,
            column: 0,
        })?;
        let mut h = Sha256::new();
        h.update(b"ti.artifact/0.1\0");
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
        Ok(Self {
            compiled: Arc::new(compiled),
            manifest,
            identity: format!("{:x}", h.finalize()),
        })
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn compiled(&self) -> &Compiled {
        &self.compiled
    }
}
#[derive(Default)]
pub struct Registry {
    artifacts: Map<(String, i64), Artifact>,
}
impl Registry {
    pub fn publish(&mut self, artifact: Artifact) -> Result<&Artifact> {
        let key = (artifact.manifest.module.clone(), artifact.manifest.version);
        if self
            .artifacts
            .get(&key)
            .is_some_and(|a| a.identity != artifact.identity)
        {
            return err("TI_MODEL", "published model version is immutable");
        };
        Ok(self.artifacts.entry(key).or_insert(artifact))
    }
    pub fn get(&self, module: &str, version: i64) -> Option<&Artifact> {
        self.artifacts.get(&(module.into(), version))
    }
}

fn err<T>(code: &str, message: &str) -> Result<T> {
    Err(Diagnostic {
        code: code.into(),
        message: message.into(),
        line: 0,
        column: 0,
    })
}
