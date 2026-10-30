//! Pure multi-module compilation. The caller supplies immutable source snapshots;
//! this module never opens a file or grants host authority.
use crate::*;
use serde::Serialize;

pub(crate) fn error(code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        message: message.into(),
        line: 0,
        column: 0,
    }
}
#[derive(Clone, Debug)]
pub struct PackageSource {
    pub name: String,
    pub version: String,
    pub root: String,
    pub kind: String,
    pub model: Option<(String, i64)>,
    pub dependencies: Map<String, String>,
    /// Expected dotted module path -> exact UTF-8 bytes represented as a String.
    pub sources: Map<String, String>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Symbol {
    pub package: String,
    pub version: String,
    pub module: String,
    pub kind: String,
    pub name: String,
    pub handle: String,
}
#[derive(Debug, Serialize)]
pub struct ProjectCompiled {
    edition: &'static str,
    profile: &'static str,
    graph_sha256: String,
    exports: Map<String, Symbol>,
    symbols: Map<String, Symbol>,
    ir_format: &'static str,
    checked: Compiled,
}
impl ProjectCompiled {
    pub fn edition(&self) -> &str {
        self.edition
    }
    pub fn profile(&self) -> &str {
        self.profile
    }
    pub fn graph_sha256(&self) -> &str {
        &self.graph_sha256
    }
    pub fn exports(&self) -> &Map<String, Symbol> {
        &self.exports
    }
    pub fn symbols(&self) -> &Map<String, Symbol> {
        &self.symbols
    }

    pub fn evaluate(
        &self,
        entity: &str,
        action: &str,
        actor: &Actor,
        inputs: &Map<String, Value>,
        old: Option<&Record>,
        new_id: &str,
    ) -> Result<Candidate> {
        let symbol = self
            .exports
            .get(entity)
            .filter(|s| s.kind == "entity")
            .ok_or_else(|| error("TI_ACCESS_DENIED", "entity is not a root model export"))?;
        self.checked
            .evaluate(&symbol.handle, action, actor, inputs, old, new_id)
    }
    pub fn module_name(&self) -> &str {
        self.checked.module_name()
    }
    pub fn model_version(&self) -> i64 {
        self.checked.model_version()
    }
}
#[derive(Clone)]
struct Resolved {
    bindings: Map<String, Symbol>,
    public: Map<String, Symbol>,
}
fn key(package: &str, module: &str) -> String {
    format!("{package}/{module}")
}
fn symbol(p: &PackageSource, module: &str, kind: &str, name: &str) -> Symbol {
    let tuple = serde_json::to_vec(&("ti.symbol/0.2", &p.name, &p.version, module, kind, name))
        .expect("string tuple");
    let hash = format!("{:x}", Sha256::digest(tuple));
    // Internal lowering handles are not a public 0.2 transport codec. A full
    // defining tuple is retained and collisions are checked before compilation.
    let prefix = match kind {
        "entity" => "E",
        "policy" => "P",
        _ => "V",
    };
    Symbol {
        package: p.name.clone(),
        version: p.version.clone(),
        module: module.into(),
        kind: kind.into(),
        name: name.into(),
        handle: format!("{prefix}{}", &hash[..56]),
    }
}
fn target(p: &PackageSource, path: &str) -> Result<String> {
    let (first, tail) = path.split_once('.').unwrap_or((path, ""));
    if first == p.root {
        return Ok(key(&p.name, path));
    }
    let dep = p
        .dependencies
        .get(first)
        .ok_or_else(|| error("TI_IMPORT", "unknown dependency alias"))?;
    Ok(format!("{dep}/@{}", tail)) // dependency root is substituted after lookup
}
fn binding<'a>(scope: &'a Map<String, Symbol>, name: &str, kind: &str) -> Result<&'a Symbol> {
    scope.get(name).filter(|s| s.kind == kind).ok_or_else(|| {
        error(
            "TI_IMPORT",
            "unknown name or incompatible imported declaration kind",
        )
    })
}
fn ty(t: &str, scope: &Map<String, Symbol>) -> Result<String> {
    let (base, nullable) = t.strip_suffix('?').map_or((t, false), |v| (v, true));
    let out = if BUILTIN[..4].contains(&base) {
        base.to_owned()
    } else {
        binding(scope, base, "id")?.handle.clone()
    };
    Ok(if nullable { format!("{out}?") } else { out })
}
fn rewrite(e: &mut Expr, scope: &Map<String, Symbol>) -> Result<()> {
    match e {
        Expr::Call(name, args) => {
            if !["length", "trim", "nonempty", "actor.hasRole"].contains(&name.as_str()) {
                *name = binding(scope, name, "policy")?.handle.clone();
            }
            for arg in args {
                rewrite(arg, scope)?;
            }
        }
        Expr::Binary(_, a, b) => {
            rewrite(a, scope)?;
            rewrite(b, scope)?;
        }
        Expr::Not(x) => rewrite(x, scope)?,
        _ => (),
    }
    Ok(())
}
fn topo(mut graph: Map<String, Set<String>>) -> Result<Vec<String>> {
    let mut out = vec![];
    while !graph.is_empty() {
        let ready: Vec<_> = graph
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(n, _)| n.clone())
            .collect();
        if ready.is_empty() {
            return Err(error(
                "TI_DEPENDENCY_CYCLE",
                "cyclic package or module imports",
            ));
        }
        for name in ready {
            graph.remove(&name);
            for deps in graph.values_mut() {
                deps.remove(&name);
            }
            out.push(name);
        }
    }
    Ok(out)
}
/// Compile only after the caller has verified its lock. No untrusted JSON IR
/// constructor is exposed; checked handles are issued after all linked checks.
pub fn compile_packages(
    packages: &[PackageSource],
    root_package: &str,
    graph_sha256: &str,
) -> Result<ProjectCompiled> {
    if packages.is_empty() || packages.len() > 32 {
        return Err(error("TI_LIMIT", "package count exceeds 32"));
    }
    let mut package_map = Map::new();
    let mut package_graph: Map<String, Set<String>> = Map::new();
    let mut total_bytes = 0usize;
    let mut total_declarations = 0;
    for p in packages {
        if package_map.insert(p.name.clone(), p).is_some() {
            return Err(error("TI_PACKAGE", "duplicate package identity"));
        }
        if p.sources.len() > 128 || p.dependencies.len() > 16 {
            return Err(error("TI_LIMIT", "package file or dependency limit"));
        }
        package_graph.insert(p.name.clone(), p.dependencies.values().cloned().collect());
    }
    for deps in package_graph.values() {
        if deps.iter().any(|d| !package_map.contains_key(d)) {
            return Err(error("TI_PACKAGE", "missing dependency package"));
        }
    }
    let order = topo(package_graph.clone())?;
    let mut depths: Map<String, usize> = Map::new();
    for name in order {
        let depth = 1 + package_graph[&name]
            .iter()
            .map(|d| depths[d])
            .max()
            .unwrap_or(0);
        if depth > 16 {
            return Err(error("TI_LIMIT", "package dependency depth exceeds 16"));
        }
        depths.insert(name, depth);
    }

    let root = *package_map
        .get(root_package)
        .ok_or_else(|| error("TI_PACKAGE", "missing root package"))?;
    let mut modules = Map::new();
    let mut module_packages = Map::new();
    let mut own = Map::new();
    let mut all_symbols = Map::new();
    for p in packages {
        let mut declarations = 0;
        let mut bytes = 0;
        if !p.sources.contains_key(&p.root) {
            return Err(error("TI_PACKAGE", "root module source missing"));
        }
        for (path, source) in &p.sources {
            bytes += source.len();
            total_bytes += source.len();
            if bytes > 2097152 || total_bytes > 8388608 {
                return Err(error("TI_LIMIT", "project source byte budget"));
            }
            let m = syntax::parse_module(source).map_err(|mut e| {
                e.message = format!("{}/{path}: {}", p.name, e.message);
                e
            })?;
            if m.model.module != *path
                || !(path == &p.root || path.starts_with(&format!("{}.", p.root)))
            {
                return Err(error(
                    "TI_PACKAGE",
                    "module path does not match source location",
                ));
            }
            declarations += m.model.entities.len()
                + m.model.events.len()
                + m.model.policies.len()
                + m.model.operations.len();
            total_declarations += m.model.entities.len()
                + m.model.events.len()
                + m.model.policies.len()
                + m.model.operations.len();
            if declarations > 256 || total_declarations > 1024 {
                return Err(error("TI_LIMIT", "project declaration budget"));
            }
            let mut names = Map::new();
            for (kind, decls) in [
                ("entity", m.model.entities.keys().collect::<Vec<_>>()),
                ("event", m.model.events.keys().collect()),
                ("policy", m.model.policies.keys().collect()),
            ] {
                for name in decls {
                    let s = symbol(p, path, kind, name);
                    if all_symbols.insert(s.handle.clone(), s.clone()).is_some() {
                        return Err(error("TI_NAME", "lowered symbol collision"));
                    }
                    if names.insert(name.clone(), s.clone()).is_some() {
                        return Err(error("TI_NAME", "declaration collision"));
                    }
                    if kind == "entity" {
                        let mut id = s;
                        id.kind = "id".into();
                        id.name = format!("{name}Id");
                        id.handle.push_str("Id");
                        if names.insert(id.name.clone(), id).is_some() {
                            return Err(error("TI_NAME", "generated ID collision"));
                        }
                    }
                }
            }
            let k = key(&p.name, path);
            own.insert(k.clone(), names);
            module_packages.insert(k.clone(), p);
            modules.insert(k, m);
        }
    }
    let mut graph = Map::new();
    let resolve_target = |p: &PackageSource, path: &str| -> Result<String> {
        let t = target(p, path)?;
        if let Some((dep, tail)) = t.split_once("/@") {
            let dp = package_map
                .get(dep)
                .ok_or_else(|| error("TI_IMPORT", "dependency missing"))?;
            Ok(key(
                dep,
                &if tail.is_empty() {
                    dp.root.clone()
                } else {
                    format!("{}.{tail}", dp.root)
                },
            ))
        } else {
            Ok(t)
        }
    };
    for (k, m) in &modules {
        let p = module_packages[k];
        let mut deps = Set::new();
        for import in &m.imports {
            let t = resolve_target(p, &import.module)?;
            if !modules.contains_key(&t) {
                return Err(error("TI_IMPORT", "imported module does not exist"));
            }
            deps.insert(t);
        }
        graph.insert(k.clone(), deps);
    }
    let mut resolved: Map<String, Resolved> = Map::new();
    let mut package_imports: Map<String, usize> = Map::new();
    let mut package_exports: Map<String, usize> = Map::new();
    for k in topo(graph)? {
        let m = &modules[&k];
        let p = module_packages[&k];
        let mut bindings = own[&k].clone();
        let mut public = Map::new();
        for name in &m.exports {
            public.insert(
                name.clone(),
                bindings
                    .get(name)
                    .ok_or_else(|| error("TI_VISIBILITY", "invalid public declaration"))?
                    .clone(),
            );
        }
        for import in &m.imports {
            let target = resolve_target(p, &import.module)?;
            let exports = &resolved[&target].public;
            for (name, alias) in &import.items {
                let s = exports
                    .get(name)
                    .ok_or_else(|| error("TI_VISIBILITY", "import target is private or missing"))?
                    .clone();
                if bindings.insert(alias.clone(), s.clone()).is_some() {
                    return Err(error(
                        "TI_NAME",
                        "import alias shadows a declaration or another import",
                    ));
                }
                if import.public {
                    if public.insert(alias.clone(), s.clone()).is_some() {
                        return Err(error("TI_NAME", "duplicate re-export"));
                    }
                    if s.kind == "entity" {
                        let mut id = s.clone();
                        id.kind = "id".into();
                        id.name.push_str("Id");
                        id.handle.push_str("Id");
                        if public.insert(format!("{alias}Id"), id).is_some() {
                            return Err(error("TI_NAME", "re-exported ID collision"));
                        }
                    }
                }
            }
        }
        let imports = package_imports.entry(p.name.clone()).or_default();
        *imports += m.imports.iter().map(|i| i.items.len()).sum::<usize>();
        let exports = package_exports.entry(p.name.clone()).or_default();
        *exports += public.len();
        if *imports > 256 || *exports > 256 || bindings.len() > 256 || public.len() > 256 {
            return Err(error("TI_LIMIT", "import/export budget"));
        }
        resolved.insert(k, Resolved { bindings, public });
    }
    let (model_name, model_revision) = root.model.clone().unwrap_or((root.root.clone(), 1));
    let mut merged = Model {
        module: model_name,
        version: model_revision,
        entities: Map::new(),
        events: Map::new(),
        policies: Map::new(),
        operations: vec![],
    };
    let mut public_types = Set::new();
    for r in resolved.values() {
        for s in r.public.values() {
            public_types.insert(s.handle.clone());
        }
    }
    for (k, m) in modules {
        let scope = &resolved[&k].bindings;
        let p = module_packages[&k];
        for (name, mut e) in m.model.entities {
            let s = binding(scope, &name, "entity")?;
            for t in e.fields.values_mut() {
                *t = ty(t, scope)?;
                if public_types.contains(&s.handle)
                    && t.trim_end_matches('?').ends_with("Id")
                    && !BUILTIN.contains(&t.trim_end_matches('?'))
                    && !public_types.contains(t.trim_end_matches('?'))
                {
                    return Err(error(
                        "TI_VISIBILITY",
                        "public entity references private nominal type",
                    ));
                }
            }
            for (_, x) in &mut e.invariants {
                rewrite(x, scope)?;
            }
            merged.entities.insert(s.handle.clone(), e);
        }
        for (name, mut fields) in m.model.events {
            let s = binding(scope, &name, "event")?;
            for t in fields.values_mut() {
                *t = ty(t, scope)?;
                if public_types.contains(&s.handle)
                    && t.trim_end_matches('?').ends_with("Id")
                    && !BUILTIN.contains(&t.trim_end_matches('?'))
                    && !public_types.contains(t.trim_end_matches('?'))
                {
                    return Err(error(
                        "TI_VISIBILITY",
                        "public event references private nominal type",
                    ));
                }
            }
            merged.events.insert(s.handle.clone(), fields);
        }
        for (name, mut policy) in m.model.policies {
            let s = binding(scope, &name, "policy")?;
            policy.entity = binding(scope, &policy.entity, "entity")?.handle.clone();
            if public_types.contains(&s.handle) && !public_types.contains(&policy.entity) {
                return Err(error(
                    "TI_VISIBILITY",
                    "public policy references private entity",
                ));
            }
            rewrite(&mut policy.expr, scope)?;
            merged.policies.insert(s.handle.clone(), policy);
        }
        for mut op in m.model.operations {
            let entity = binding(scope, &op.entity, "entity")?;
            if entity.package != p.name {
                return Err(error(
                    "TI_EFFECT",
                    "foreign entity operations cannot be extended",
                ));
            }
            op.entity = entity.handle.clone();
            for t in op.inputs.values_mut() {
                *t = ty(t, scope)?;
            }
            rewrite(&mut op.allow, scope)?;
            for (_, x) in op.requires.iter_mut().chain(op.sets.iter_mut()) {
                rewrite(x, scope)?;
            }
            for (event, fields) in &mut op.emits {
                let s = binding(scope, event, "event")?;
                if public_types.contains(&entity.handle) && !public_types.contains(&s.handle) {
                    return Err(error(
                        "TI_VISIBILITY",
                        "public entity emits a private event",
                    ));
                }
                *event = s.handle.clone();
                for (_, x) in fields {
                    rewrite(x, scope)?;
                }
            }
            merged.operations.push(op);
        }
    }
    merged
        .operations
        .sort_by(|a, b| (&a.entity, &a.name).cmp(&(&b.entity, &b.name)));
    merged.validate()?;
    let mut exports = resolved[&key(&root.name, &root.root)].public.clone();
    for s in exports.values() {
        if s.kind == "entity" && s.package != root.name {
            return Err(error("TI_EFFECT", "root cannot activate a foreign entity"));
        }
    }
    if root.kind == "library" {
        exports.retain(|_, s| s.kind != "entity");
    }
    let checked = Compiled {
        model: merged,
        source_sha256: graph_sha256.into(),
        edition: "ti/0.2",
        profile: "approval-package/0.2",
    };
    Ok(ProjectCompiled {
        edition: "ti/0.2",
        profile: "approval-package/0.2",
        graph_sha256: graph_sha256.into(),
        exports,
        symbols: all_symbols,
        ir_format: "ti.project-ir/0.2-dev.1",
        checked,
    })
}
