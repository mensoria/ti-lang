//! Ti compiler, semantic checks, deterministic evaluator and normative codec.
mod check;
mod evaluator;
mod model;
pub mod project;
mod syntax;
pub mod wire;
use check::{assign, BUILTIN, RESERVED};
pub use evaluator::{Actor, Candidate, Event, Record};
pub use model::Model;
use model::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap as Map, BTreeSet as Set};
pub type Result<T> = std::result::Result<T, Diagnostic>;
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
}
fn err<T>(code: &str, msg: impl Into<String>) -> Result<T> {
    Err(Diagnostic {
        code: code.into(),
        message: msg.into(),
        line: 0,
        column: 0,
    })
}
#[derive(Debug, Clone, Serialize)]
struct Token {
    text: String,
    line: usize,
    column: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    Bool(bool),
    String(String),
    Int64(String),
    UserId(String),
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "id")]
    Id(IdValue),
    #[serde(rename = "state")]
    State(String),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdValue {
    pub entity: String,
    pub id: String,
}
impl Value {
    fn ty(&self) -> String {
        match self {
            Self::Bool(_) => "Bool".into(),
            Self::String(_) => "String".into(),
            Self::Int64(_) => "Int64".into(),
            Self::UserId(_) => "UserId".into(),
            Self::Null => "null".into(),
            Self::Id(v) => format!("{}Id", v.entity),
            Self::State(_) => "state".into(),
        }
    }
}
/// A validated model; construction is only possible through `compile`.
#[derive(Debug, Serialize)]
pub struct Compiled {
    model: Model,
    pub source_sha256: String,
    pub edition: &'static str,
    pub profile: &'static str,
}
pub fn compile(source: &str) -> Result<Compiled> {
    let model = syntax::parse(source)?;
    model.validate()?;
    Ok(Compiled {
        model,
        source_sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
        edition: "ti/0.1",
        profile: "approval-core/0.1",
    })
}

/// Decode source bytes without accepting invalid UTF-8 or a BOM.
pub fn compile_bytes(bytes: &[u8]) -> Result<Compiled> {
    if bytes.len() > 262144 {
        return err("TI_LIMIT", "source exceeds 256 KiB");
    }
    let source = std::str::from_utf8(bytes).map_err(|_| Diagnostic {
        code: "TI_SYNTAX".into(),
        message: "source is not valid UTF-8".into(),
        line: 1,
        column: 1,
    })?;
    compile(source)
}

impl Compiled {
    pub fn module_name(&self) -> &str {
        &self.model.module
    }
    pub fn model_version(&self) -> i64 {
        self.model.version
    }
    pub fn ir_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self.model).map_err(|_| Diagnostic {
            code: "TI_MODEL".into(),
            message: "IR serialization failed".into(),
            line: 0,
            column: 0,
        })
    }
}
/// Exact source inputs to the current language implementation fingerprint.
pub fn implementation_sources() -> &'static [&'static str] {
    &[
        include_str!("lib.rs"),
        include_str!("model.rs"),
        include_str!("project.rs"),
        include_str!("check.rs"),
        include_str!("syntax.rs"),
        include_str!("evaluator.rs"),
        include_str!("wire.rs"),
    ]
}
