use crate::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub roles: Set<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub fields: Map<String, Value>,
    pub state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub record: Record,
    pub events: Vec<Event>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub fields: Map<String, Value>,
}
struct Eval<'a> {
    model: &'a Model,
    actor: &'a Actor,
    left: usize,
}
fn truth(v: Value) -> Result<bool> {
    if let Value::Bool(b) = v {
        Ok(b)
    } else {
        err("TI_TYPE", "expected Boolean")
    }
}
fn trim(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\r', '\n'])
}
impl Eval<'_> {
    /// Explicit work stack: policy expansion consumes the shared node budget
    /// without adding an implementation-only native-stack recursion limit.
    fn expr(
        &mut self,
        e: &Expr,
        env: &Map<String, Value>,
        records: &Map<String, Record>,
    ) -> Result<Value> {
        #[derive(Clone, Copy)]
        enum Scope<'a> {
            Root(&'a Map<String, Value>, &'a Map<String, Record>),
            Policy(&'a Policy, &'a Record),
        }
        enum Task<'a> {
            Node(&'a Expr, Scope<'a>),
            Not,
            Left(&'a str, &'a Expr, Scope<'a>),
            Right(&'a str, Value),
            Call(&'a str),
        }
        let mut tasks = vec![Task::Node(e, Scope::Root(env, records))];
        let mut stack: Vec<Value> = vec![];
        while let Some(task) = tasks.pop() {
            match task {
                Task::Node(e, scope) => {
                    if self.left == 0 {
                        return err("TI_LIMIT", "evaluation budget exhausted");
                    }
                    self.left -= 1;
                    match e {
                        Expr::Lit(v) => stack.push(v.clone()),
                        Expr::Ref(n) => {
                            let value = match scope {
                                Scope::Root(env, _) => env.get(n).cloned(),
                                Scope::Policy(p, r) => {
                                    let entity = &self.model.entities[&p.entity];
                                    if n == "actor.id" {
                                        Some(Value::UserId(self.actor.id.clone()))
                                    } else if n == &format!("{}.id", p.param) {
                                        Some(Value::Id(IdValue {
                                            entity: p.entity.clone(),
                                            id: r.id.clone(),
                                        }))
                                    } else if n == &format!("{}.{}", p.param, entity.state) {
                                        Some(Value::State(r.state.clone()))
                                    } else if entity.states.contains_key(n) {
                                        Some(Value::State(n.clone()))
                                    } else {
                                        n.strip_prefix(&format!("{}.", p.param))
                                            .and_then(|f| r.fields.get(f))
                                            .cloned()
                                    }
                                }
                            };
                            stack.push(value.ok_or_else(|| Diagnostic {
                                code: "TI_INPUT".into(),
                                message: "missing expression binding".into(),
                                line: 0,
                                column: 0,
                            })?);
                        }
                        Expr::Not(x) => {
                            tasks.push(Task::Not);
                            tasks.push(Task::Node(x, scope));
                        }
                        Expr::Binary(op, a, b) => {
                            tasks.push(Task::Left(op, b, scope));
                            tasks.push(Task::Node(a, scope));
                        }
                        Expr::Call(n, args) => {
                            if let Some(p) = self.model.policies.get(n) {
                                let Expr::Ref(arg) = &args[0] else {
                                    return err("TI_TYPE", "entity argument required");
                                };
                                let record = match scope {
                                    Scope::Root(_, records) => records.get(arg),
                                    Scope::Policy(parent, record) => {
                                        if arg == &parent.param {
                                            Some(record)
                                        } else {
                                            None
                                        }
                                    }
                                }
                                .ok_or_else(|| Diagnostic {
                                    code: "TI_INPUT".into(),
                                    message: "missing policy entity".into(),
                                    line: 0,
                                    column: 0,
                                })?;
                                tasks.push(Task::Node(&p.expr, Scope::Policy(p, record)));
                            } else {
                                tasks.push(Task::Call(n));
                                tasks.push(Task::Node(&args[0], scope));
                            }
                        }
                    }
                }
                Task::Not => {
                    let v = stack.pop().expect("validated expression stack");
                    stack.push(Value::Bool(!truth(v)?));
                }
                Task::Left(op, b, scope) => {
                    let a = stack.pop().expect("validated expression stack");
                    if (op == "and" && !truth(a.clone())?) || (op == "or" && truth(a.clone())?) {
                        stack.push(a);
                    } else {
                        tasks.push(Task::Right(op, a));
                        tasks.push(Task::Node(b, scope));
                    }
                }
                Task::Right(op, a) => {
                    let b = stack.pop().expect("validated expression stack");
                    let result = match op {
                        "and" | "or" => truth(b)?,
                        "==" => a == b,
                        "!=" => a != b,
                        _ => {
                            let (Value::Int64(a), Value::Int64(b)) = (a, b) else {
                                return err("TI_TYPE", "integer required");
                            };
                            let a: i64 = a.parse().map_err(|_| Diagnostic {
                                code: "TI_INPUT".into(),
                                message: "invalid integer".into(),
                                line: 0,
                                column: 0,
                            })?;
                            let b: i64 = b.parse().map_err(|_| Diagnostic {
                                code: "TI_INPUT".into(),
                                message: "invalid integer".into(),
                                line: 0,
                                column: 0,
                            })?;
                            match op {
                                "<" => a < b,
                                "<=" => a <= b,
                                ">" => a > b,
                                _ => a >= b,
                            }
                        }
                    };
                    stack.push(Value::Bool(result));
                }
                Task::Call(n) => {
                    let v = stack.pop().expect("validated expression stack");
                    stack.push(match (n, v) {
                        ("length", Value::String(s)) => Value::Int64(s.chars().count().to_string()),
                        ("trim", Value::String(s)) => Value::String(trim(&s).into()),
                        ("nonempty", Value::String(s)) => Value::Bool(!trim(&s).is_empty()),
                        ("nonempty", Value::Null) => Value::Bool(false),
                        ("actor.hasRole", Value::String(s)) => {
                            Value::Bool(self.actor.roles.contains(&s))
                        }
                        _ => return err("TI_TYPE", "invalid call"),
                    });
                }
            }
        }
        Ok(stack.pop().expect("validated expression result"))
    }
}

fn values(m: &Model, entity: &str, prefix: &str, r: &Record) -> Map<String, Value> {
    let mut env = Map::new();
    for (n, v) in &r.fields {
        env.insert(format!("{prefix}.{n}"), v.clone());
    }
    env.insert(
        format!("{prefix}.id"),
        Value::Id(IdValue {
            entity: entity.into(),
            id: r.id.clone(),
        }),
    );
    env.insert(
        format!("{prefix}.{}", m.entities[entity].state),
        Value::State(r.state.clone()),
    );
    for s in m.entities[entity].states.keys() {
        env.insert(s.clone(), Value::State(s.clone()));
    }
    env
}
fn valid_value(v: &Value, t: &str) -> bool {
    if let Value::Int64(s) = v {
        if s.parse::<i64>().is_err() || s.parse::<i64>().unwrap().to_string() != *s {
            return false;
        }
    }
    match v {
        Value::String(s) if s.len() > 16384 => return false,
        Value::UserId(s) if s.is_empty() || s.len() > 256 => return false,
        Value::Id(v) if v.id.is_empty() || v.id.len() > 256 => return false,
        _ => (),
    }
    assign(&v.ty(), t)
}
impl Compiled {
    /// Returns a candidate only. Actor/context must be authenticated by the caller.
    /// The host must atomically authorize, CAS, persist, audit and publish an outbox.
    pub fn evaluate(
        &self,
        entity: &str,
        action: &str,
        actor: &Actor,
        input: &Map<String, Value>,
        old: Option<&Record>,
        new_id: &str,
    ) -> Result<Candidate> {
        let m = &self.model;
        let o = m
            .operations
            .iter()
            .find(|o| o.entity == entity && o.name == action)
            .ok_or_else(|| Diagnostic {
                code: "TI_NAME".into(),
                message: "unknown action".into(),
                line: 0,
                column: 0,
            })?;
        let e = &m.entities[entity];
        if actor.id.is_empty()
            || actor.id.len() > 256
            || actor.roles.len() > 256
            || actor
                .roles
                .iter()
                .any(|r| r.is_empty() || r.len() > 128 || !r.is_ascii())
        {
            return err("TI_INPUT", "invalid actor snapshot");
        }
        if input.len() != o.inputs.len()
            || !o
                .inputs
                .iter()
                .all(|(n, t)| input.get(n).is_some_and(|v| valid_value(v, t)))
        {
            return err("TI_INPUT", "input shape or type mismatch");
        }
        let encoded: Result<Map<_, _>> = input
            .iter()
            .map(|(k, v)| Ok((k, crate::wire::encode_value(v)?)))
            .collect();
        if serde_json::to_vec(&encoded?).map_or(true, |v| v.len() > 131072) {
            return err("TI_LIMIT", "encoded public inputs exceed 128 KiB");
        }
        if o.from.is_some() != old.is_some() {
            return err("TI_INPUT", "action requires matching old-record presence");
        }
        let mut r = if let Some(r) = old {
            if r.id.is_empty()
                || r.id.len() > 256
                || r.fields.len() != e.fields.len()
                || !e
                    .fields
                    .iter()
                    .all(|(n, t)| r.fields.get(n).is_some_and(|v| valid_value(v, t)))
                || !e.states.contains_key(&r.state)
            {
                return err("TI_RECORD_INVALID", "invalid stored record");
            }
            r.clone()
        } else {
            if new_id.is_empty() || new_id.len() > 256 {
                return err("TI_INPUT", "stable logical ID required");
            }
            Record {
                id: new_id.into(),
                fields: e.fields.keys().map(|n| (n.clone(), Value::Null)).collect(),
                state: e
                    .states
                    .iter()
                    .find(|(_, v)| v.as_str() == "initial")
                    .unwrap()
                    .0
                    .clone(),
            }
        };
        let mut env = if old.is_some() {
            values(m, entity, "self", &r)
        } else {
            Map::new()
        };
        env.insert("actor.id".into(), Value::UserId(actor.id.clone()));
        for (n, v) in input {
            env.insert(format!("input.{n}"), v.clone());
        }
        let records = old
            .map(|r| Map::from([("self".into(), r.clone())]))
            .unwrap_or_default();
        let mut eval = Eval {
            model: m,
            actor,
            left: 100000,
        };
        if !truth(eval.expr(&o.allow, &env, &records)?)? {
            return err("TI_ACCESS_DENIED", "action denied");
        }
        if o.from.as_ref().is_some_and(|s| s != &r.state) {
            return err("TI_STATE", "invalid source state");
        }
        let check = |eval: &mut Eval, r: &Record, code: &str| -> Result<()> {
            let mut env = r.fields.clone();
            env.insert(e.state.clone(), Value::State(r.state.clone()));
            for s in e.states.keys() {
                env.insert(s.clone(), Value::State(s.clone()));
            }
            for (n, x) in &e.invariants {
                if !truth(eval.expr(x, &env, &Map::new())?)? {
                    return err(code, n.clone());
                }
            }
            Ok(())
        };
        if old.is_some() {
            check(&mut eval, &r, "TI_RECORD_INVALID")?
        }
        for (n, x) in &o.requires {
            if !truth(eval.expr(x, &env, &records)?)? {
                return err("TI_REQUIRE", n.clone());
            }
        }
        let mut changes = vec![];
        for (n, x) in &o.sets {
            changes.push((n.clone(), eval.expr(x, &env, &records)?));
        }
        for (n, v) in changes {
            r.fields.insert(n, v);
        }
        if let Some(to) = &o.to {
            r.state = to.clone()
        }
        check(&mut eval, &r, "TI_INVARIANT")?;
        let mut event_env = values(m, entity, "self", &r);
        for (n, v) in input {
            event_env.insert(format!("input.{n}"), v.clone());
        }
        let mut events = vec![];
        for (n, fields) in &o.emits {
            let mut out = Map::new();
            for (f, x) in fields {
                out.insert(f.clone(), eval.expr(x, &event_env, &Map::new())?);
            }
            events.push(Event {
                name: n.clone(),
                fields: out,
            });
        }
        let encoded: Result<Vec<_>> = events.iter().map(crate::wire::encode_event).collect();
        if serde_json::to_vec(&encoded?).map_or(true, |v| v.len() > 131072) {
            return err("TI_LIMIT", "encoded public events exceed 128 KiB");
        }
        Ok(Candidate { record: r, events })
    }
}

impl Compiled {
    pub fn validate_record(&self, entity: &str, record: &Record) -> Result<()> {
        let e = self.model.entities.get(entity).ok_or_else(|| Diagnostic {
            code: "TI_NAME".into(),
            message: "unknown entity".into(),
            line: 0,
            column: 0,
        })?;
        if record.fields.len() != e.fields.len()
            || !e.states.contains_key(&record.state)
            || !e
                .fields
                .iter()
                .all(|(n, t)| record.fields.get(n).is_some_and(|v| valid_value(v, t)))
        {
            return err("TI_RECORD_INVALID", "stored record shape mismatch");
        }
        let actor = Actor {
            id: "maintenance".into(),
            roles: Set::new(),
        };
        let mut eval = Eval {
            model: &self.model,
            actor: &actor,
            left: 100000,
        };
        let mut env = record.fields.clone();
        env.insert(e.state.clone(), Value::State(record.state.clone()));
        for state in e.states.keys() {
            env.insert(state.clone(), Value::State(state.clone()));
        }
        for (_, invariant) in &e.invariants {
            if !truth(eval.expr(invariant, &env, &Map::new())?)? {
                return err("TI_RECORD_INVALID", "stored invariant failed");
            }
        }
        Ok(())
    }
}
