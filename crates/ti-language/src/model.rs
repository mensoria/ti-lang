use crate::*;

#[derive(Debug, Clone, Serialize)]
pub(crate) enum Expr {
    Lit(Value),
    Ref(String),
    Call(String, Vec<Expr>),
    Not(Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Entity {
    pub(crate) fields: Map<String, String>,
    pub(crate) state: String,
    pub(crate) states: Map<String, String>,
    pub(crate) invariants: Vec<(String, Expr)>,
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Operation {
    pub(crate) entity: String,
    pub(crate) name: String,
    pub(crate) inputs: Map<String, String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) allow: Expr,
    pub(crate) requires: Vec<(String, Expr)>,
    pub(crate) sets: Vec<(String, Expr)>,
    pub(crate) emits: Vec<(String, Vec<(String, Expr)>)>,
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Policy {
    pub(crate) param: String,
    pub(crate) entity: String,
    pub(crate) expr: Expr,
}
#[derive(Debug, Clone, Serialize)]
pub struct Model {
    pub module: String,
    pub version: i64,
    pub(crate) entities: Map<String, Entity>,
    pub(crate) policies: Map<String, Policy>,
    pub(crate) events: Map<String, Map<String, String>>,
    pub(crate) operations: Vec<Operation>,
}
