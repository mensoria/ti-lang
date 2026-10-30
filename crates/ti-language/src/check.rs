use crate::*;

pub(crate) const RESERVED: &[&str] = &[
    "module",
    "version",
    "entity",
    "field",
    "state",
    "initial",
    "final",
    "invariant",
    "policy",
    "create",
    "action",
    "on",
    "from",
    "to",
    "allow",
    "require",
    "else",
    "set",
    "emit",
    "event",
    "true",
    "false",
    "null",
    "and",
    "or",
    "not",
    "actor",
    "input",
    "self",
];
pub(crate) const BUILTIN: &[&str] = &[
    "Bool", "String", "Int64", "UserId", "length", "trim", "nonempty",
];
pub(crate) fn assign(a: &str, b: &str) -> bool {
    a == b || (b.ends_with('?') && (a == b.trim_end_matches('?') || a == "null"))
}
fn bool_type(t: String) -> Result<()> {
    if t == "Bool" {
        Ok(())
    } else {
        err("TI_TYPE", "predicate must be Bool")
    }
}
impl Model {
    fn expr_type(
        &self,
        e: &Expr,
        env: &Map<String, String>,
        _stack: &mut Vec<String>,
    ) -> Result<String> {
        match e {
            Expr::Lit(v) => Ok(v.ty()),
            Expr::Ref(n) => env.get(n).cloned().ok_or_else(|| Diagnostic {
                code: "TI_NAME".into(),
                message: format!("unavailable reference {n}"),
                line: 0,
                column: 0,
            }),
            Expr::Not(x) => {
                bool_type(self.expr_type(x, env, _stack)?)?;
                Ok("Bool".into())
            }
            Expr::Binary(op, a, b) => {
                let bare_state = |x: &Expr| matches!(x, Expr::Ref(n) if !n.contains('.') && env.get(n).is_some_and(|t| t.starts_with("state:")) && env.get("#state-field").is_none_or(|field| field != n));
                if bare_state(a) && bare_state(b) {
                    return err("TI_TYPE", "state comparison requires a typed state field");
                }
                let a = self.expr_type(a, env, _stack)?;
                let b = self.expr_type(b, env, _stack)?;
                if op == "and" || op == "or" {
                    bool_type(a)?;
                    bool_type(b)?;
                } else if op == "==" || op == "!=" {
                    if !((a == b && !a.ends_with('?') && a != "null" && !a.starts_with("entity:"))
                        || (a.ends_with('?') && b == "null")
                        || (b.ends_with('?') && a == "null"))
                    {
                        return err("TI_TYPE", "incompatible equality");
                    }
                } else if a != "Int64" || b != "Int64" {
                    return err("TI_TYPE", "ordered comparison requires Int64");
                }
                Ok("Bool".into())
            }
            Expr::Call(n, args) => {
                if args.len() != 1 {
                    return err("TI_TYPE", "call requires one argument");
                }
                let t = self.expr_type(&args[0], env, _stack)?;
                match n.as_str() {
                    "length" | "trim" => {
                        if t != "String" {
                            return err("TI_TYPE", "String required");
                        }
                        Ok(if n == "length" { "Int64" } else { "String" }.into())
                    }
                    "nonempty" => {
                        if t != "String" && t != "String?" && t != "null" {
                            return err("TI_TYPE", "nullable String required");
                        }
                        Ok("Bool".into())
                    }
                    "actor.hasRole" => {
                        if !env.contains_key("actor.id") {
                            return err("TI_EFFECT", "actor unavailable");
                        }
                        match &args[0] {
                            Expr::Lit(Value::String(s))
                                if !s.is_empty() && s.len() <= 128 && s.is_ascii() => {}
                            _ => return err("TI_TYPE", "literal ASCII role required"),
                        };
                        Ok("Bool".into())
                    }
                    _ => {
                        let p = self.policies.get(n).ok_or_else(|| Diagnostic {
                            code: "TI_EFFECT".into(),
                            message: format!("unsupported call {n}"),
                            line: 0,
                            column: 0,
                        })?;
                        if t != format!("entity:{}", p.entity) {
                            return err("TI_TYPE", "policy entity mismatch");
                        }
                        if !env.contains_key("actor.id") {
                            return err("TI_EFFECT", "policy unavailable in this context");
                        }
                        Ok("Bool".into())
                    }
                }
            }
        }
    }
    fn entity_env(&self, n: &str, prefix: &str) -> Result<Map<String, String>> {
        let e = self.entities.get(n).ok_or_else(|| Diagnostic {
            code: "TI_NAME".into(),
            message: format!("unknown entity {n}"),
            line: 0,
            column: 0,
        })?;
        let mut env = Map::new();
        for (f, t) in &e.fields {
            env.insert(format!("{prefix}.{f}"), t.clone());
        }
        env.insert(prefix.into(), format!("entity:{n}"));
        env.insert(format!("{prefix}.id"), format!("{n}Id"));
        env.insert(format!("{prefix}.{}", e.state), format!("state:{n}"));
        for s in e.states.keys() {
            env.insert(s.clone(), format!("state:{n}"));
        }
        Ok(env)
    }
    pub(crate) fn validate(&self) -> Result<()> {
        fn calls(e: &Expr, out: &mut Set<String>) {
            match e {
                Expr::Call(n, args) => {
                    out.insert(n.clone());
                    for a in args {
                        calls(a, out);
                    }
                }
                Expr::Binary(_, a, b) => {
                    calls(a, out);
                    calls(b, out);
                }
                Expr::Not(a) => calls(a, out),
                _ => (),
            }
        }
        let mut graph: Map<String, Set<String>> = self
            .policies
            .iter()
            .map(|(n, p)| {
                let mut deps = Set::new();
                calls(&p.expr, &mut deps);
                deps.retain(|n| self.policies.contains_key(n));
                (n.clone(), deps)
            })
            .collect();
        while !graph.is_empty() {
            let ready: Set<String> = graph
                .iter()
                .filter(|(_, v)| v.is_empty())
                .map(|(n, _)| n.clone())
                .collect();
            if ready.is_empty() {
                return err("TI_EFFECT", "recursive policy");
            }
            graph.retain(|n, _| !ready.contains(n));
            for deps in graph.values_mut() {
                deps.retain(|n| !ready.contains(n));
            }
        }

        let valid = |t: &str| {
            let t = t.strip_suffix('?').unwrap_or(t);
            ["Bool", "String", "Int64", "UserId"].contains(&t)
                || self.entities.keys().any(|e| t == format!("{e}Id"))
        };
        for n in self.entities.keys() {
            let generated = format!("{n}Id");
            if self.entities.contains_key(&generated)
                || self.events.contains_key(&generated)
                || self.policies.contains_key(&generated)
            {
                return err("TI_NAME", "generated ID collision");
            }
        }
        for (e_name, e) in &self.entities {
            if e.states
                .values()
                .filter(|x| x.as_str() == "initial")
                .count()
                != 1
                || e.fields.contains_key(&e.state)
            {
                return err(
                    "TI_MODEL",
                    "exactly one initial state and distinct state field required",
                );
            }
            for f in e.fields.keys().chain(std::iter::once(&e.state)) {
                if [
                    "id",
                    "revision",
                    "created_at",
                    "updated_at",
                    "scope",
                    "model_version",
                ]
                .contains(&f.as_str())
                {
                    return err("TI_NAME", "platform field cannot be declared");
                }
            }
            for s in e.states.keys() {
                if e.fields.contains_key(s) || s == &e.state || BUILTIN.contains(&s.as_str()) {
                    return err("TI_NAME", "state name collision");
                }
            }
            if !e.fields.values().all(|t| valid(t)) {
                return err("TI_TYPE", "unknown field type");
            }
            let env = self.invariant_env(e_name)?;
            let mut seen = Set::new();
            for (n, x) in &e.invariants {
                if !seen.insert(n) {
                    return err("TI_NAME", "duplicate invariant");
                }
                bool_type(self.expr_type(x, &env, &mut vec![])?)?;
            }
            if self
                .operations
                .iter()
                .filter(|o| o.entity == *e_name && o.name == "Create")
                .count()
                != 1
            {
                return err("TI_MODEL", "entity requires exactly one creation");
            }
        }
        for fields in self.events.values() {
            if !fields.values().all(|t| valid(t)) {
                return err("TI_TYPE", "unknown event type");
            }
        }
        for (n, p) in &self.policies {
            let mut env = self.entity_env(&p.entity, &p.param)?;
            env.insert("actor.id".into(), "UserId".into());
            bool_type(self.expr_type(&p.expr, &env, &mut vec![n.clone()])?)?;
        }
        let mut names = Set::new();
        for o in &self.operations {
            if !names.insert((&o.entity, &o.name)) {
                return err("TI_NAME", "duplicate operation");
            }
            let ent = self.entities.get(&o.entity).ok_or_else(|| Diagnostic {
                code: "TI_NAME".into(),
                message: "unknown operation entity".into(),
                line: 0,
                column: 0,
            })?;
            let mut env = if o.from.is_some() {
                self.entity_env(&o.entity, "self")?
            } else {
                Map::new()
            };
            env.insert("actor.id".into(), "UserId".into());
            for (n, t) in &o.inputs {
                if !valid(t) {
                    return err("TI_TYPE", "unknown input type");
                }
                env.insert(format!("input.{n}"), t.clone());
            }
            if let Some(f) = &o.from {
                if !ent.states.contains_key(f)
                    || o.to.as_ref().is_none_or(|t| !ent.states.contains_key(t))
                    || ent.states[f] == "final"
                {
                    return err("TI_MODEL", "invalid transition");
                }
            }
            bool_type(self.expr_type(&o.allow, &env, &mut vec![])?)?;
            let mut labels = Set::new();
            for (n, x) in &o.requires {
                if n.is_empty()
                    || n.len() > 63
                    || !n.as_bytes()[0].is_ascii_lowercase()
                    || !n
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    || !labels.insert(n)
                {
                    return err("TI_MODEL", "invalid requirement label");
                }
                bool_type(self.expr_type(x, &env, &mut vec![])?)?;
            }
            let mut assigned = Set::new();
            for (n, x) in &o.sets {
                let t = ent.fields.get(n).ok_or_else(|| Diagnostic {
                    code: "TI_NAME".into(),
                    message: format!("not an assignable field: {n}"),
                    line: 0,
                    column: 0,
                })?;
                if !assigned.insert(n) {
                    return err("TI_NAME", "duplicate assignment");
                }
                if !assign(&self.expr_type(x, &env, &mut vec![])?, t) {
                    return err("TI_TYPE", "assignment type mismatch");
                }
            }
            if o.from.is_none()
                && ent
                    .fields
                    .iter()
                    .any(|(n, t)| !t.ends_with('?') && !assigned.contains(n))
            {
                return err("TI_MODEL", "required field not initialized");
            }
            env.remove("actor.id");
            for (n, values) in &o.emits {
                let fields = self.events.get(n).ok_or_else(|| Diagnostic {
                    code: "TI_NAME".into(),
                    message: "unknown event".into(),
                    line: 0,
                    column: 0,
                })?;
                let mut seen = Set::new();
                for (f, x) in values {
                    if !seen.insert(f)
                        || !fields.get(f).is_some_and(|t| {
                            self.expr_type(x, &env, &mut vec![])
                                .is_ok_and(|a| assign(&a, t))
                        })
                    {
                        return err("TI_TYPE", "invalid event field");
                    }
                }
                if seen.len() != fields.len() {
                    return err("TI_TYPE", "missing event field");
                }
            }
        }
        Ok(())
    }
    fn invariant_env(&self, n: &str) -> Result<Map<String, String>> {
        let e = &self.entities[n];
        let mut env = e.fields.clone();
        env.insert("#state-field".into(), e.state.clone());
        env.insert(e.state.clone(), format!("state:{n}"));
        for s in e.states.keys() {
            env.insert(s.clone(), format!("state:{n}"));
        }
        Ok(env)
    }
}
