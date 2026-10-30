use crate::*;

fn lex(s: &str) -> Result<Vec<Token>> {
    if s.len() > 262144 {
        return err("TI_LIMIT", "source exceeds 256 KiB");
    }
    let cs: Vec<char> = s.chars().collect();
    let (mut i, mut line, mut col) = (0, 1, 1);
    let mut out = vec![];
    while i < cs.len() {
        let c = cs[i];
        if c == '\r' {
            if cs.get(i + 1) != Some(&'\n') {
                return err("TI_SYNTAX", "lone CR");
            }
            i += 1;
            continue;
        }
        if c == '\n' {
            line += 1;
            col = 1;
            i += 1;
            continue;
        }
        if c == ' ' || c == '\t' {
            i += 1;
            col += 1;
            continue;
        }
        if c == '/' && cs.get(i + 1) == Some(&'/') {
            while i < cs.len() && cs[i] != '\n' && cs[i] != '\r' {
                i += 1;
                col += 1
            }
            continue;
        }
        let start = i;
        let column = col;
        if c == '"' {
            i += 1;
            let mut escaped = false;
            while i < cs.len() {
                let d = cs[i];
                if d < ' ' {
                    return err("TI_SYNTAX", "control character in string");
                }
                i += 1;
                if d == '"' && !escaped {
                    break;
                }
                escaped = d == '\\' && !escaped;
            }
            let raw: String = cs[start..i].iter().collect();
            let decoded: String = serde_json::from_str(&raw).map_err(|_| Diagnostic {
                code: "TI_SYNTAX".into(),
                message: "invalid string".into(),
                line,
                column,
            })?;
            if decoded.len() > 16384 {
                return err("TI_LIMIT", "string exceeds 16 KiB");
            }
        } else if c.is_ascii_alphabetic() || c == '_' {
            i += 1;
            while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '_') {
                i += 1
            }
            if i - start > 63 {
                return err("TI_LIMIT", "identifier too long");
            }
        } else if c.is_ascii_digit() || c == '-' {
            i += 1;
            while i < cs.len() && cs[i].is_ascii_digit() {
                i += 1
            }
            let n: String = cs[start..i].iter().collect();
            if n == "-0"
                || n.starts_with("00")
                || n.starts_with("-0")
                || (n.starts_with('0') && n.len() > 1)
                || n.parse::<i64>().is_err()
            {
                return err("TI_SYNTAX", "noncanonical or overflowing integer");
            }
        } else if "{}():;,?.=<>!".contains(c) {
            i += 1;
            if "=<>!".contains(c) && cs.get(i) == Some(&'=') {
                i += 1
            }
        } else {
            return Err(Diagnostic {
                code: "TI_SYNTAX".into(),
                message: format!("unexpected character {c:?}"),
                line,
                column,
            });
        }
        let text = cs[start..i].iter().collect();
        out.push(Token { text, line, column });
        col += i - start;
    }
    out.push(Token {
        text: "<eof>".into(),
        line,
        column: col,
    });
    Ok(out)
}
struct Parser {
    ts: Vec<Token>,
    i: usize,
    depth: usize,
}
impl Parser {
    fn peek(&self) -> &str {
        &self.ts[self.i].text
    }
    fn pop(&mut self) -> String {
        let v = self.peek().to_string();
        if self.i + 1 < self.ts.len() {
            self.i += 1
        }
        v
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.peek() == s {
            self.pop();
            true
        } else {
            false
        }
    }
    fn need(&mut self, s: &str) -> Result<()> {
        if self.eat(s) {
            Ok(())
        } else {
            err("TI_SYNTAX", format!("expected {s}, found {}", self.peek()))
        }
    }
    fn name(&mut self) -> Result<String> {
        let s = self.pop();
        if s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && !RESERVED.contains(&s.as_str())
            && !BUILTIN.contains(&s.as_str())
        {
            Ok(s)
        } else {
            err("TI_NAME", format!("invalid declaration name {s}"))
        }
    }
    fn ty(&mut self) -> Result<String> {
        let mut s = self.pop();
        if self.eat("?") {
            s.push('?')
        }
        Ok(s)
    }
    fn fields(&mut self) -> Result<Map<String, String>> {
        let mut m = Map::new();
        while self.eat("field") {
            let n = self.name()?;
            self.need(":")?;
            let t = self.ty()?;
            self.need(";")?;
            if m.insert(n, t).is_some() {
                return err("TI_NAME", "duplicate field");
            }
            if m.len() > 64 {
                return err("TI_LIMIT", "too many fields");
            }
        }
        Ok(m)
    }
    fn params(&mut self) -> Result<Map<String, String>> {
        let mut m = Map::new();
        self.need("(")?;
        if !self.eat(")") {
            loop {
                let n = self.name()?;
                self.need(":")?;
                let t = self.ty()?;
                if m.insert(n, t).is_some() {
                    return err("TI_NAME", "duplicate input");
                }
                if m.len() > 64 {
                    return err("TI_LIMIT", "too many inputs");
                }
                if self.eat(")") {
                    break;
                }
                self.need(",")?
            }
        }
        Ok(m)
    }
    fn expr(&mut self, min: u8) -> Result<Expr> {
        self.depth += 1;
        if self.depth > 64 {
            return err("TI_LIMIT", "expression nesting exceeds 64");
        }
        let mut e = if self.eat("not") {
            Expr::Not(Box::new(self.expr(4)?))
        } else if self.eat("(") {
            let x = self.expr(0)?;
            self.need(")")?;
            x
        } else {
            let s = self.pop();
            if s.starts_with('"') {
                Expr::Lit(Value::String(serde_json::from_str(&s).unwrap()))
            } else if s == "true" || s == "false" {
                Expr::Lit(Value::Bool(s == "true"))
            } else if s == "null" {
                Expr::Lit(Value::Null)
            } else if s.parse::<i64>().is_ok() {
                Expr::Lit(Value::Int64(s))
            } else {
                if !s
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                {
                    return err("TI_SYNTAX", "expected expression");
                }
                let mut n = s;
                if self.eat(".") {
                    let member = self.pop();
                    if !member
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    {
                        return err("TI_SYNTAX", "expected member");
                    }
                    n.push('.');
                    n.push_str(&member)
                }
                if self.eat("(") {
                    let mut args = vec![];
                    if !self.eat(")") {
                        loop {
                            args.push(self.expr(0)?);
                            if args.len() > 64 {
                                return err("TI_LIMIT", "too many arguments");
                            }
                            if self.eat(")") {
                                break;
                            }
                            self.need(",")?
                        }
                    }
                    Expr::Call(n, args)
                } else {
                    Expr::Ref(n)
                }
            }
        };
        let mut compared = false;
        loop {
            let op = self.peek();
            let prec = match op {
                "or" => 1,
                "and" => 2,
                "==" | "!=" | "<" | "<=" | ">" | ">=" => 3,
                _ => 0,
            };
            if prec == 0 || prec < min {
                break;
            }
            if prec == 3 && compared {
                return err("TI_SYNTAX", "chained comparison");
            }
            compared |= prec == 3;
            let op = self.pop();
            let rhs = self.expr(prec + 1)?;
            e = Expr::Binary(op, Box::new(e), Box::new(rhs));
        }
        let mut pending = vec![(&e, 1usize)];
        while let Some((node, depth)) = pending.pop() {
            if depth > 64 {
                return err("TI_LIMIT", "expression AST nesting exceeds 64");
            }
            match node {
                Expr::Binary(_, a, b) => {
                    pending.push((a, depth + 1));
                    pending.push((b, depth + 1));
                }
                Expr::Not(a) => pending.push((a, depth + 1)),
                Expr::Call(_, args) => {
                    for arg in args {
                        pending.push((arg, depth + 1));
                    }
                }
                _ => (),
            }
        }
        self.depth -= 1;
        Ok(e)
    }
    fn operation(&mut self, create: bool) -> Result<Operation> {
        let (name, entity, inputs) = if create {
            let e = self.name()?;
            let p = self.params()?;
            ("Create".into(), e, p)
        } else {
            let n = self.name()?;
            if n == "Create" {
                return err("TI_NAME", "reserved action Create");
            }
            let p = if self.peek() == "(" {
                self.params()?
            } else {
                Map::new()
            };
            self.need("on")?;
            (n, self.name()?, p)
        };
        self.need("{")?;
        let (from, to) = if create {
            (None, None)
        } else {
            self.need("from")?;
            let a = self.name()?;
            self.need("to")?;
            let b = self.name()?;
            self.need(";")?;
            (Some(a), Some(b))
        };
        self.need("allow")?;
        let allow = self.expr(0)?;
        self.need(";")?;
        let mut requires = vec![];
        let mut sets = vec![];
        let mut emits = vec![];
        while self.eat("require") {
            let e = self.expr(0)?;
            self.need("else")?;
            let label = self.pop();
            let label: String = serde_json::from_str(&label).map_err(|_| Diagnostic {
                code: "TI_SYNTAX".into(),
                message: "require label must be string".into(),
                line: 0,
                column: 0,
            })?;
            self.need(";")?;
            requires.push((label, e));
        }
        while self.eat("set") {
            let n = self.name()?;
            self.need("=")?;
            let e = self.expr(0)?;
            self.need(";")?;
            sets.push((n, e));
        }
        while !create && self.eat("emit") {
            let n = self.name()?;
            self.need("{")?;
            let mut values = vec![];
            while !self.eat("}") {
                let f = self.name()?;
                self.need("=")?;
                let e = self.expr(0)?;
                self.need(";")?;
                values.push((f, e));
            }
            emits.push((n, values));
        }
        self.need("}")?;
        if requires.len() > 64 || emits.len() > 16 {
            return err("TI_LIMIT", "operation too large");
        }
        Ok(Operation {
            entity,
            name,
            inputs,
            from,
            to,
            allow,
            requires,
            sets,
            emits,
        })
    }
    fn model(&mut self) -> Result<Model> {
        self.need("module")?;
        let module = self.name()?;
        self.need("version")?;
        let version = self.pop().parse::<i64>().unwrap_or(0);
        if !(1..=2147483647).contains(&version) {
            return err("TI_MODEL", "invalid module version");
        }
        self.need(";")?;
        let mut m = Model {
            module,
            version,
            entities: Map::new(),
            events: Map::new(),
            policies: Map::new(),
            operations: vec![],
        };
        let mut names = Set::new();
        let mut count = 0;
        while self.peek() != "<eof>" {
            count += 1;
            if count > 256 {
                return err("TI_LIMIT", "too many declarations");
            }
            match self.pop().as_str() {
                "entity" => {
                    let n = self.name()?;
                    if !names.insert(n.clone()) {
                        return err("TI_NAME", "duplicate declaration");
                    }
                    self.need("{")?;
                    let fields = self.fields()?;
                    self.need("state")?;
                    let state = self.name()?;
                    self.need("{")?;
                    let mut states = Map::new();
                    while !self.eat("}") {
                        let s = self.name()?;
                        let mark = if self.peek() == "initial" || self.peek() == "final" {
                            self.pop()
                        } else {
                            String::new()
                        };
                        self.need(";")?;
                        if states.insert(s, mark).is_some() {
                            return err("TI_NAME", "duplicate state");
                        }
                        if states.len() > 64 {
                            return err("TI_LIMIT", "too many states");
                        }
                    }
                    let mut invariants = vec![];
                    while self.eat("invariant") {
                        let n = self.name()?;
                        self.need(":")?;
                        let e = self.expr(0)?;
                        self.need(";")?;
                        invariants.push((n, e));
                        if invariants.len() > 64 {
                            return err("TI_LIMIT", "too many invariants");
                        }
                    }
                    self.need("}")?;
                    m.entities.insert(
                        n,
                        Entity {
                            fields,
                            state,
                            states,
                            invariants,
                        },
                    );
                }
                "event" => {
                    let n = self.name()?;
                    if !names.insert(n.clone()) {
                        return err("TI_NAME", "duplicate declaration");
                    }
                    self.need("{")?;
                    let f = self.fields()?;
                    self.need("}")?;
                    m.events.insert(n, f);
                }
                "policy" => {
                    let n = self.name()?;
                    if !names.insert(n.clone()) {
                        return err("TI_NAME", "duplicate declaration");
                    }
                    self.need("(")?;
                    let param = self.name()?;
                    self.need(":")?;
                    let entity = self.name()?;
                    self.need(")")?;
                    self.need("=")?;
                    let expr = self.expr(0)?;
                    self.need(";")?;
                    m.policies.insert(
                        n,
                        Policy {
                            param,
                            entity,
                            expr,
                        },
                    );
                }
                "create" => m.operations.push(self.operation(true)?),
                "action" => m.operations.push(self.operation(false)?),
                _ => return err("TI_SYNTAX", "expected declaration"),
            }
        }
        Ok(m)
    }
}

pub(crate) fn parse(source: &str) -> Result<Model> {
    let ts = lex(source)?;
    let mut p = Parser { ts, i: 0, depth: 0 };
    p.model().map_err(|mut e| {
        let t = &p.ts[p.i];
        e.line = t.line;
        e.column = t.column;
        e
    })
}
