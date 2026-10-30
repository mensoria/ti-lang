//! Bounded offline project loading and explicit dependency locking.
mod io;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap as Map,
    path::{Component, Path, PathBuf},
};
pub use ti_language::project::{ProjectCompiled, Symbol};
use ti_language::{project::PackageSource, Diagnostic, Result};

fn fail(code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        message: message.into(),
        line: 0,
        column: 0,
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub package: Package,
    pub model: Option<Model>,
    #[serde(default)]
    pub dependencies: Map<String, Dependency>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub profile: String,
    pub root: String,
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub name: String,
    pub revision: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub package: String,
    pub version: String,
    pub path: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageRef {
    pub name: String,
    pub version: String,
    pub content_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub profile: String,
    pub root: String,
    pub kind: String,
    pub content_sha256: String,
    pub dependencies: Map<String, PackageRef>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    pub schema: String,
    pub root_package: PackageRef,
    pub packages: Vec<LockedPackage>,
    pub graph_sha256: String,
}
#[derive(Clone)]
struct Loaded {
    manifest: Manifest,
    content: String,
    sources: Map<String, String>,
    deps: Map<String, String>,
}
pub struct Project {
    packages: Map<String, Loaded>,
    root: String,
    lock: Lock,
}
impl Project {
    pub fn lockfile(&self) -> &Lock {
        &self.lock
    }
    pub fn compile(&self) -> Result<ProjectCompiled> {
        let packages: Vec<_> = self
            .packages
            .values()
            .map(|p| PackageSource {
                name: p.manifest.package.name.clone(),
                version: p.manifest.package.version.clone(),
                root: p.manifest.package.root.clone(),
                kind: p.manifest.package.kind.clone(),
                model: p
                    .manifest
                    .model
                    .as_ref()
                    .map(|m| (m.name.clone(), m.revision)),
                dependencies: p.deps.clone(),
                sources: p.sources.clone(),
            })
            .collect();
        ti_language::project::compile_packages(&packages, &self.root, &self.lock.graph_sha256)
    }
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 63
        && (s.as_bytes()[0].is_ascii_lowercase() || s.as_bytes()[0] == b'_')
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && !['\0'].iter().any(|c| s.contains(*c))
        && ![
            "module",
            "pub",
            "use",
            "as",
            "entity",
            "policy",
            "event",
            "create",
            "action",
            "field",
            "state",
            "initial",
            "final",
            "invariant",
            "allow",
            "require",
            "set",
            "emit",
            "version",
            "true",
            "false",
            "null",
            "actor",
            "input",
            "self",
            "from",
            "to",
            "on",
            "else",
            "and",
            "or",
            "not",
        ]
        .contains(&s)
}
fn package_name(s: &str) -> bool {
    s.len() <= 191
        && s.split('.').count() >= 2
        && s.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && part.as_bytes()[0].is_ascii_lowercase()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}

fn version(s: &str) -> bool {
    let parts: Vec<_> = s.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            p.parse::<u32>()
                .is_ok_and(|v| v <= 2147483647 && v.to_string() == *p)
        })
}
fn manifest(bytes: &[u8]) -> Result<Manifest> {
    let source =
        std::str::from_utf8(bytes).map_err(|_| fail("TI_PROJECT", "manifest must be UTF-8"))?;
    let m: Manifest = toml::from_str(source).map_err(|_| {
        fail(
            "TI_PROJECT",
            "invalid, duplicate or unknown manifest fields",
        )
    })?;
    let p = &m.package;
    if m.schema != "ti.project/1"
        || !package_name(&p.name)
        || !version(&p.version)
        || !identifier(&p.root)
        || p.edition != "ti/0.2"
        || p.profile != "approval-package/0.2"
        || !["model", "library"].contains(&p.kind.as_str())
        || (p.kind == "model") != m.model.is_some()
    {
        return Err(fail(
            "TI_PACKAGE",
            "unsupported or invalid package identity, kind, edition or profile",
        ));
    }
    if m.model
        .as_ref()
        .is_some_and(|m| !identifier(&m.name) || !(1..=2147483647).contains(&m.revision))
    {
        return Err(fail("TI_PACKAGE", "invalid model name or revision"));
    }
    if m.dependencies.len() > 16 {
        return Err(fail("TI_LIMIT", "dependency edge limit"));
    }
    for (alias, dep) in &m.dependencies {
        if !identifier(alias)
            || alias == &p.root
            || !package_name(&dep.package)
            || !version(&dep.version)
            || dep.path.is_empty()
            || Path::new(&dep.path).is_absolute()
            || dep.path.contains('\\')
            || dep.path.contains(':')
        {
            return Err(fail(
                "TI_PACKAGE",
                "invalid dependency alias, identity or local path",
            ));
        }
    }
    Ok(m)
}
fn normalize(p: &Path) -> Result<PathBuf> {
    let path = if p.is_absolute() {
        p.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| fail("TI_PROJECT", "working directory unavailable"))?
            .join(p)
    };
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                out.push(part.as_os_str())
            }
            Component::CurDir => (),
            Component::ParentDir => {
                if !out.pop() {
                    return Err(fail("TI_PROJECT", "invalid path traversal"));
                }
            }
        }
    }
    Ok(out)
}
fn canonical(value: &serde_json::Value) -> Result<Vec<u8>> {
    fn sorted(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let ordered: Map<_, _> = map.iter().collect();
                let mut out = serde_json::Map::new();
                for (k, v) in ordered {
                    out.insert(k.clone(), sorted(v));
                }
                serde_json::Value::Object(out)
            }
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(sorted).collect())
            }
            _ => value.clone(),
        }
    }
    serde_json::to_vec(&sorted(value))
        .map_err(|_| fail("TI_PACKAGE", "canonical serialization failed"))
}

fn digest(domain: &str, tag: u8, bytes: &[u8]) -> Result<String> {
    let mut h = Sha256::new();
    h.update(ti_language::wire::frame(b'd', domain.as_bytes())?);
    h.update(ti_language::wire::frame(tag, bytes)?);
    Ok(format!("{:x}", h.finalize()))
}
fn content(m: &Manifest, sources: &Map<String, String>) -> Result<String> {
    let mut value =
        serde_json::to_value(m).map_err(|_| fail("TI_PROJECT", "manifest serialization failed"))?;
    if m.model.is_none() {
        value
            .as_object_mut()
            .expect("manifest object")
            .remove("model");
    }
    for dep in value["dependencies"]
        .as_object_mut()
        .expect("dependency object")
        .values_mut()
    {
        dep.as_object_mut().expect("dependency").remove("path");
    }
    let mut h = Sha256::new();
    h.update(ti_language::wire::frame(b'd', b"ti.package-source/1")?);
    h.update(ti_language::wire::frame(b'm', &canonical(&value)?)?);
    for (path, source) in sources {
        let relative = format!("src/{}.ti", path.replace('.', "/"));
        h.update(ti_language::wire::frame(b'p', relative.as_bytes())?);
        h.update(ti_language::wire::frame(b's', source.as_bytes())?);
    }
    Ok(format!("{:x}", h.finalize()))
}
struct Loader {
    roots: Vec<io::Root>,
    packages: Map<String, Loaded>,
    stack: Vec<String>,
    bytes: usize,
    paths: Map<PathBuf, String>,
    attempts: usize,
}
impl Loader {
    fn new(manifest: &Path, roots: &[PathBuf]) -> Result<Self> {
        let dirs = if roots.is_empty() {
            vec![manifest
                .parent()
                .ok_or_else(|| fail("TI_PROJECT", "manifest has no parent"))?
                .to_owned()]
        } else {
            roots.to_vec()
        };
        if dirs.len() > 32 {
            return Err(fail("TI_LIMIT", "workspace root count"));
        }
        let mut authorized = vec![];
        for root in dirs {
            let path = std::fs::canonicalize(root)
                .map_err(|_| fail("TI_PROJECT", "workspace root missing"))?;
            authorized.push(io::Root::new(path)?);
        }
        Ok(Self {
            roots: authorized,
            packages: Map::new(),
            stack: vec![],
            bytes: 0,
            paths: Map::new(),
            attempts: 0,
        })
    }
    fn root(&self, path: &Path) -> Result<&io::Root> {
        self.roots
            .iter()
            .filter(|r| path.starts_with(&r.path))
            .max_by_key(|r| r.path.components().count())
            .ok_or_else(|| {
                fail(
                    "TI_PROJECT",
                    "dependency is outside explicitly authorized workspace roots",
                )
            })
    }
    fn sources(&self, dir: &Path, root: &str) -> Result<Map<String, String>> {
        let src = dir.join("src");
        let mut pending = vec![(src.clone(), 0usize)];
        let mut files = Map::new();
        let mut bytes = 0;
        let mut entries = 0;
        while let Some((path, depth)) = pending.pop() {
            if depth > 32 {
                return Err(fail("TI_LIMIT", "source directory depth"));
            }
            for (name, directory) in self.root(&path)?.entries(&path)? {
                entries += 1;
                if entries > 4096 {
                    return Err(fail("TI_LIMIT", "source directory entry budget"));
                }
                if name == "Ti.toml" {
                    return Err(fail("TI_PROJECT", "nested manifest is forbidden"));
                }
                let child = path.join(&name);
                if directory {
                    if !identifier(&name) {
                        return Err(fail("TI_PACKAGE", "invalid module directory"));
                    }
                    pending.push((child, depth + 1));
                    continue;
                }
                if !name.ends_with(".ti") {
                    continue;
                }
                if files.len() >= 128 {
                    return Err(fail("TI_LIMIT", "source file count"));
                }
                let stem = name.strip_suffix(".ti").expect("suffix");
                if !identifier(stem) {
                    return Err(fail("TI_PACKAGE", "nonportable source filename"));
                }
                let relative = child
                    .strip_prefix(&src)
                    .map_err(|_| fail("TI_PROJECT", "source escaped source root"))?;
                let module = relative
                    .with_extension("")
                    .components()
                    .map(|c| c.as_os_str().to_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join(".");
                if !(module == root || module.starts_with(&format!("{root}."))) {
                    return Err(fail(
                        "TI_PACKAGE",
                        "source path outside declared root module",
                    ));
                }
                let raw = self.root(&child)?.read(&child, 262144)?;
                bytes += raw.len();
                if bytes > 2097152 {
                    return Err(fail("TI_LIMIT", "package source byte budget"));
                }
                let text = String::from_utf8(raw)
                    .map_err(|_| fail("TI_SYNTAX", "source must be UTF-8"))?;
                if files.insert(module, text).is_some() {
                    return Err(fail("TI_PACKAGE", "duplicate module source"));
                }
            }
        }
        if !files.contains_key(root) {
            return Err(fail("TI_PACKAGE", "root source file missing"));
        }
        Ok(files)
    }
    fn load(
        &mut self,
        path: &Path,
        expected: Option<(&str, &str)>,
        depth: usize,
    ) -> Result<String> {
        self.attempts += 1;
        if self.attempts > 512 {
            return Err(fail("TI_LIMIT", "dependency resolution attempt budget"));
        }
        if depth > 16 {
            return Err(fail("TI_LIMIT", "dependency depth exceeds 16"));
        }
        let path = normalize(path)?;
        if let Some(name) = self.paths.get(&path) {
            if self.stack.contains(name) {
                return Err(fail("TI_DEPENDENCY_CYCLE", "package dependency cycle"));
            }
            let prior = &self.packages[name];
            if expected.is_some_and(|(n, v)| n != name || v != prior.manifest.package.version) {
                return Err(fail(
                    "TI_PACKAGE",
                    "cached dependency does not match exact declared identity",
                ));
            }
            return Ok(name.clone());
        }
        let m = manifest(&self.root(&path)?.read(&path, 65536)?)?;
        let name = m.package.name.clone();
        if expected.is_some_and(|(n, v)| n != name || v != m.package.version) {
            return Err(fail(
                "TI_PACKAGE",
                "dependency manifest does not match exact declared name/version",
            ));
        }
        if self.stack.contains(&name) {
            return Err(fail("TI_DEPENDENCY_CYCLE", "package dependency cycle"));
        }
        let directory = path
            .parent()
            .ok_or_else(|| fail("TI_PROJECT", "manifest parent unavailable"))?;
        let sources = self.sources(directory, &m.package.root)?;
        let content = content(&m, &sources)?;
        if let Some(prior) = self.packages.get(&name) {
            if prior.manifest.package.version != m.package.version || prior.content != content {
                return Err(fail(
                    "TI_PACKAGE",
                    "multiple versions or contents for one package",
                ));
            }
        } else if self.packages.len() + self.stack.len() >= 32 {
            return Err(fail("TI_LIMIT", "package count exceeds 32"));
        }
        self.bytes += sources.values().map(|s| s.len()).sum::<usize>();
        if self.bytes > 8388608 {
            return Err(fail("TI_LIMIT", "graph source byte budget"));
        }
        self.stack.push(name.clone());
        let mut deps = Map::new();
        for (alias, dep) in &m.dependencies {
            let child = normalize(&directory.join(&dep.path).join("Ti.toml"))?;
            let found = self.load(&child, Some((&dep.package, &dep.version)), depth + 1)?;
            deps.insert(alias.clone(), found);
        }
        self.stack.pop();
        if let Some(prior) = self.packages.get(&name) {
            if prior.deps != deps {
                return Err(fail(
                    "TI_PACKAGE",
                    "inconsistent duplicate package dependency graph",
                ));
            }
        }
        self.paths.insert(path, name.clone());
        self.packages.insert(
            name.clone(),
            Loaded {
                manifest: m,
                content,
                sources,
                deps,
            },
        );
        Ok(name)
    }
    fn graph_depth(&self, root: &str) -> Result<()> {
        fn visit(loader: &Loader, name: &str, memo: &mut Map<String, usize>) -> Result<usize> {
            if let Some(v) = memo.get(name) {
                return Ok(*v);
            }
            let mut depth = 1;
            for child in loader.packages[name].deps.values() {
                depth = depth.max(1 + visit(loader, child, memo)?);
            }
            if depth > 16 {
                return Err(fail("TI_LIMIT", "resolved dependency depth exceeds 16"));
            }
            memo.insert(name.into(), depth);
            Ok(depth)
        }
        visit(self, root, &mut Map::new()).map(|_| ())
    }
    fn lock(&self, root: &str) -> Result<Lock> {
        let reference = |name: &str| -> PackageRef {
            let p = &self.packages[name];
            PackageRef {
                name: name.into(),
                version: p.manifest.package.version.clone(),
                content_sha256: p.content.clone(),
            }
        };
        let packages = self
            .packages
            .values()
            .map(|p| LockedPackage {
                name: p.manifest.package.name.clone(),
                version: p.manifest.package.version.clone(),
                edition: p.manifest.package.edition.clone(),
                profile: p.manifest.package.profile.clone(),
                root: p.manifest.package.root.clone(),
                kind: p.manifest.package.kind.clone(),
                content_sha256: p.content.clone(),
                dependencies: p
                    .deps
                    .iter()
                    .map(|(a, d)| (a.clone(), reference(d)))
                    .collect(),
            })
            .collect();
        let mut lock = Lock {
            schema: "ti.lock/1".into(),
            root_package: reference(root),
            packages,
            graph_sha256: String::new(),
        };
        lock.graph_sha256 = digest(
            "ti.package-graph/1",
            b'g',
            &canonical(
                &serde_json::json!({"root_package":lock.root_package,"packages":lock.packages}),
            )?,
        )?;
        Ok(lock)
    }
}
/// Explicit lock update. Parses and links all sources before atomic replacement.
pub fn lock(manifest_path: &Path, workspace_roots: &[PathBuf]) -> Result<Project> {
    load(manifest_path, workspace_roots, true)
}
/// Read-only normal loading; never creates or repairs Ti.lock.
pub fn open(manifest_path: &Path, workspace_roots: &[PathBuf]) -> Result<Project> {
    load(manifest_path, workspace_roots, false)
}
fn load(manifest_path: &Path, workspace_roots: &[PathBuf], update: bool) -> Result<Project> {
    let mut path = normalize(manifest_path)?;
    // User-selected workspace roots are trusted capability locators. Rebase
    // their spelling (for example macOS /tmp) onto the canonical opened root;
    // dependency/source components below that root still use O_NOFOLLOW.
    let selected = if workspace_roots.is_empty() {
        vec![path
            .parent()
            .ok_or_else(|| fail("TI_PROJECT", "manifest parent missing"))?
            .to_owned()]
    } else {
        workspace_roots.to_vec()
    };
    for root in &selected {
        let lexical = normalize(root)?;
        if let Ok(relative) = path.strip_prefix(&lexical) {
            let canonical = std::fs::canonicalize(&lexical)
                .map_err(|_| fail("TI_PROJECT", "workspace root unavailable"))?;
            path = canonical.join(relative);
            break;
        }
    }
    if path.file_name().and_then(|s| s.to_str()) != Some("Ti.toml") {
        return Err(fail("TI_PROJECT", "manifest must be named Ti.toml"));
    }
    let mut loader = Loader::new(&path, workspace_roots)?;
    let root = loader.load(&path, None, 1)?;
    loader.graph_depth(&root)?;
    let lock = loader.lock(&root)?;
    let lock_path = path.with_file_name("Ti.lock");
    if !update {
        let bytes = loader
            .root(&lock_path)?
            .read(&lock_path, 262144)
            .map_err(|e| {
                if e.code == "TI_LIMIT" {
                    e
                } else {
                    fail(
                        "TI_LOCK_STALE",
                        "missing or unreadable Ti.lock; run ti lock explicitly",
                    )
                }
            })?;
        let value = ti_language::wire::decode_json_document(&bytes, 262144)
            .map_err(|_| fail("TI_LOCK_STALE", "invalid or duplicate lock JSON fields"))?;
        let stored: Lock = serde_json::from_value(value)
            .map_err(|_| fail("TI_LOCK_STALE", "invalid lock schema or fields"))?;
        if stored != lock {
            return Err(fail(
                "TI_LOCK_STALE",
                "Ti.lock is stale or corrupt; run ti lock explicitly",
            ));
        }
    }
    let project = Project {
        packages: loader.packages.clone(),
        root,
        lock,
    };
    if update {
        project.compile()?;
        let mut bytes = serde_json::to_vec_pretty(&project.lock)
            .map_err(|_| fail("TI_PROJECT", "cannot encode lock"))?;
        bytes.push(b'\n');
        if bytes.len() > 262144 {
            return Err(fail("TI_LIMIT", "lock byte budget"));
        }
        loader.root(&lock_path)?.atomic(&lock_path, &bytes)?;
    }
    Ok(project)
}
