use std::{
    collections::{BTreeMap as Map, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use ti_lang::{project, Actor, Value};
struct Fixture {
    path: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        if e.file_name() == "Ti.lock" {
            continue;
        }
        if e.file_type().unwrap().is_dir() {
            copy(&e.path(), &to.join(e.file_name()));
        } else {
            fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = fs::canonicalize(std::env::temp_dir()).unwrap();
        let path = base.join(format!(
            "ti-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        copy(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/package-design"),
            &path,
        );
        Self { path }
    }
    fn manifest(&self) -> PathBuf {
        self.path.join("leave/Ti.toml")
    }
    fn roots(&self) -> Vec<PathBuf> {
        vec![self.path.clone()]
    }
    fn lock(&self) -> ti_lang::Result<project::Project> {
        project::lock(&self.manifest(), &self.roots())
    }
    fn open(&self) -> ti_lang::Result<project::Project> {
        project::open(&self.manifest(), &self.roots())
    }
    fn edit(&self, path: &str, from: &str, to: &str) {
        let path = self.path.join(path);
        let s = fs::read_to_string(&path).unwrap();
        assert!(s.contains(from), "{from}");
        fs::write(path, s.replace(from, to)).unwrap();
    }
    fn write(&self, path: &str, source: &str) {
        let path = self.path.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
}
fn code<T>(r: ti_lang::Result<T>) -> String {
    r.err().expect("expected error").code
}
#[test]
fn pc_001_005_multifile_approval() {
    let f = Fixture::new();
    let p = f.lock().unwrap();
    assert_eq!(p.lockfile().packages.len(), 2);
    let m = f.open().unwrap().compile().unwrap();
    assert_eq!(m.edition(), "ti/0.2");
    assert_eq!(m.module_name(), "leave");
    let applicant = Actor {
        id: "alice".into(),
        roles: BTreeSet::from(["leave.applicant".into()]),
    };
    let reviewer = Actor {
        id: "bob".into(),
        roles: BTreeSet::from(["leave.reviewer".into()]),
    };
    let inputs = Map::from([
        ("reason".into(), Value::String(" break ".into())),
        ("days".into(), Value::Int64("2".into())),
    ]);
    let created = m
        .evaluate("LeaveRequest", "Create", &applicant, &inputs, None, "one")
        .unwrap();
    assert_eq!(
        created.record.fields["reason"],
        Value::String("break".into())
    );
    let pending = m
        .evaluate(
            "LeaveRequest",
            "Submit",
            &applicant,
            &Map::new(),
            Some(&created.record),
            "",
        )
        .unwrap();
    assert!(m
        .evaluate(
            "LeaveRequest",
            "Approve",
            &applicant,
            &Map::new(),
            Some(&pending.record),
            ""
        )
        .is_err());
    let approved = m
        .evaluate(
            "LeaveRequest",
            "Approve",
            &reviewer,
            &Map::new(),
            Some(&pending.record),
            "",
        )
        .unwrap();
    assert_eq!(approved.record.state, "Approved");
    assert_eq!(approved.events.len(), 1);
    assert_eq!(approved.events[0].fields["days"], Value::Int64("2".into()));
    assert!(approved.events[0].fields.contains_key("requestId"));
    assert!(m.exports().contains_key("LeaveRequestId"));
}
#[test]
fn pc_002_manifest_unknown_duplicate_and_hooks() {
    for suffix in ["\nunknown=1\n", "\n[build]\nscript='anything'\n"] {
        let f = Fixture::new();
        let path = f.manifest();
        let mut s = fs::read_to_string(&path).unwrap();
        s += suffix;
        fs::write(path, s).unwrap();
        assert!(f.lock().is_err());
    }
    let f = Fixture::new();
    f.edit(
        "leave/Ti.toml",
        "schema = \"ti.project/1\"",
        "schema = \"ti.project/1\"\nschema = \"ti.project/1\"",
    );
    assert_eq!(code(f.lock()), "TI_PROJECT");
}
#[test]
fn pc_003_exact_dependency_identity_even_cached() {
    let f = Fixture::new();
    f.edit(
        "leave/Ti.toml",
        "version = \"1.2.0\", path",
        "version = \"^1.2\", path",
    );
    assert_eq!(code(f.lock()), "TI_PACKAGE");
    let f = Fixture::new();
    let path = f.manifest();
    let mut s = fs::read_to_string(&path).unwrap();
    s+="other = { package = \"acme.wrong\", version = \"1.2.0\", path = \"../approval-events\" }\n";
    fs::write(path, s).unwrap();
    assert_eq!(code(f.lock()), "TI_PACKAGE");
}
#[test]
fn pc_006_path_header_and_portable_names() {
    let f = Fixture::new();
    f.edit(
        "leave/src/leave/policies.ti",
        "module leave.policies;",
        "module leave.wrong;",
    );
    assert_eq!(code(f.lock()), "TI_PACKAGE");
    let f = Fixture::new();
    f.write("leave/src/leave/Bad.ti", "module leave.Bad;");
    assert_eq!(code(f.lock()), "TI_PACKAGE");
}
#[test]
fn pc_007_016_workspace_roots_are_explicit() {
    let f = Fixture::new();
    assert_eq!(code(project::lock(&f.manifest(), &[])), "TI_PROJECT");
    assert!(f.lock().is_ok());
}
#[cfg(unix)]
#[test]
fn pc_007_symlink_and_special_file_rejection() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = Fixture::new();
    symlink(
        outside.path.join("leave/src/leave/model.ti"),
        f.path.join("leave/src/leave/escape.ti"),
    )
    .unwrap();
    assert_eq!(code(f.lock()), "TI_PROJECT");
    assert!(!f.path.join("leave/Ti.lock").exists());
    let f = Fixture::new();
    symlink(f.path.join("approval-events"), f.path.join("link")).unwrap();
    f.edit("leave/Ti.toml", "../approval-events", "../link");
    assert_eq!(code(f.lock()), "TI_PROJECT");
}
#[test]
fn pc_008_unused_module_is_checked() {
    let f = Fixture::new();
    f.write(
        "leave/src/leave/unused.ti",
        "module leave.unused; nonsense;",
    );
    assert_eq!(code(f.lock()), "TI_SYNTAX");
    assert!(!f.path.join("leave/Ti.lock").exists());
}
#[test]
fn pc_009_alias_and_reexport_preserve_identity() {
    let f = Fixture::new();
    let m = f.lock().unwrap().compile().unwrap();
    let e = &m.exports()["LeaveRequest"];
    assert_eq!(e.package, "acme.leave");
    assert_eq!(e.module, "leave.model");
    let reviewer = m
        .symbols()
        .values()
        .filter(|s| s.name == "Reviewer")
        .collect::<Vec<_>>();
    assert_eq!(reviewer.len(), 1);
    assert!(!m.symbols().values().any(|s| s.name == "CanReview"));
}
#[test]
fn pc_010_private_import_and_reexport() {
    let f = Fixture::new();
    f.edit(
        "leave/src/leave/policies.ti",
        "pub policy Reviewer",
        "policy Reviewer",
    );
    assert_eq!(code(f.lock()), "TI_VISIBILITY");
    let f = Fixture::new();
    f.edit(
        "leave/src/leave/events.ti",
        "pub event LeaveApproved",
        "event LeaveApproved",
    );
    assert_eq!(code(f.lock()), "TI_VISIBILITY");
}
#[test]
fn pc_011_alias_shadowing() {
    let f = Fixture::new();
    f.edit(
        "leave/src/leave/actions.ti",
        "Reviewer as CanReview",
        "Reviewer as Applicant",
    );
    assert_eq!(code(f.lock()), "TI_NAME");
}
#[test]
fn pc_012_public_type_closure() {
    let f = Fixture::new();
    f.write("leave/src/leave/private.ti","module leave.private; entity Hidden {state status{Draft initial;}} create Hidden(){allow true;} pub event Leaked {field target:HiddenId;}");
    assert_eq!(code(f.lock()), "TI_VISIBILITY");
}
#[test]
fn pc_013_missing_and_duplicate_creation() {
    let f = Fixture::new();
    f.write("leave/src/leave/extra.ti","module leave.extra; use leave.model::{LeaveRequest}; create LeaveRequest(){allow true;set applicant=actor.id;set reason=\"x\";set days=1;}");
    assert_eq!(code(f.lock()), "TI_MODEL");
}
#[test]
fn pc_014_foreign_entity_extension() {
    let f = Fixture::new();
    f.write("approval-events/src/approval_events/item.ti","module approval_events.item;pub entity Item{state status{Draft initial;}}create Item(){allow true;}");
    f.write("leave/src/leave/extra.ti","module leave.extra;use approval_events.item::{Item};action Extra on Item{from Draft to Draft;allow true;}");
    assert_eq!(code(f.lock()), "TI_EFFECT");
}
#[test]
fn pc_015_foreign_root_activation_denied() {
    let f = Fixture::new();
    f.write("approval-events/src/approval_events/item.ti","module approval_events.item;pub entity Item{state status{Draft initial;}}create Item(){allow true;}");
    f.write(
        "leave/src/leave.ti",
        "module leave;pub use approval_events.item::{Item};",
    );
    assert_eq!(code(f.lock()), "TI_EFFECT");
}
#[test]
fn pc_017_missing_stale_and_transitive_source_locks() {
    let f = Fixture::new();
    assert_eq!(code(f.open()), "TI_LOCK_STALE");
    f.lock().unwrap();
    let path = f.path.join("leave/Ti.lock");
    let original = fs::read(&path).unwrap();
    f.edit(
        "approval-events/src/approval_events/events.ti",
        "field decision: String;",
        "field decision: String;\n// changed",
    );
    assert_eq!(code(f.open()), "TI_LOCK_STALE");
    assert_eq!(fs::read(&path).unwrap(), original);
    let new = f.lock().unwrap();
    assert_ne!(
        new.lockfile().graph_sha256,
        serde_json::from_slice::<project::Lock>(&original)
            .unwrap()
            .graph_sha256
    );
}
#[test]
fn pc_018_lock_duplicate_unknown_and_digest() {
    for mode in 0..3 {
        let f = Fixture::new();
        f.lock().unwrap();
        let path = f.path.join("leave/Ti.lock");
        let s = fs::read_to_string(&path).unwrap();
        let bad = match mode {
            0 => s.replacen('{', "{\"schema\":\"ti.lock/1\",", 1),
            1 => s.replacen('{', "{\"unknown\":1,", 1),
            _ => s.replace("\"graph_sha256\": \"", "\"graph_sha256\": \"bad"),
        };
        fs::write(path, bad).unwrap();
        assert_eq!(code(f.open()), "TI_LOCK_STALE");
    }
}
#[test]
fn pc_019_module_and_package_cycles() {
    let f = Fixture::new();
    f.edit(
        "leave/src/leave/model.ti",
        "module leave.model;",
        "module leave.model;\nuse leave::{LeaveApproved};",
    );
    assert_eq!(code(f.lock()), "TI_DEPENDENCY_CYCLE");
    let f = Fixture::new();
    let path = f.path.join("approval-events/Ti.toml");
    let mut s = fs::read_to_string(&path).unwrap();
    s += "leave = {package=\"acme.leave\",version=\"0.1.0\",path=\"../leave\"}\n";
    fs::write(path, s).unwrap();
    assert_eq!(code(f.lock()), "TI_DEPENDENCY_CYCLE");
}
#[test]
fn pc_020_checkout_location_does_not_change_identity() {
    let a = Fixture::new();
    let b = Fixture::new();
    let x = a.lock().unwrap();
    let y = b.lock().unwrap();
    assert_eq!(x.lockfile(), y.lockfile());
    assert_eq!(
        serde_json::to_value(x.compile().unwrap()).unwrap(),
        serde_json::to_value(y.compile().unwrap()).unwrap()
    );
}
#[test]
fn pc_021_alias_changes_build_identity() {
    let f = Fixture::new();
    let original = f.lock().unwrap().lockfile().graph_sha256.clone();
    f.edit("leave/Ti.toml", "approval_events =", "decisions =");
    f.edit(
        "leave/src/leave/events.ti",
        "use approval_events.events",
        "use decisions.events",
    );
    assert_ne!(f.lock().unwrap().lockfile().graph_sha256, original);
}
#[test]
fn pc_023_same_short_names_are_distinct() {
    let f = Fixture::new();
    for (module, alias) in [("first", "First"), ("second", "Second")] {
        f.write(&format!("leave/src/leave/{module}.ti"),&format!("module leave.{module};pub entity Item{{state status{{Draft initial;}}}}create Item(){{allow true;}}"));
        let path = f.path.join("leave/src/leave.ti");
        let mut text = fs::read_to_string(&path).unwrap();
        text += &format!("pub use leave.{module}::{{Item as {alias}}};\n");
        fs::write(path, text).unwrap();
    }
    let m = f.lock().unwrap().compile().unwrap();
    assert_ne!(m.exports()["First"].handle, m.exports()["Second"].handle);
}
#[test]
fn pc_024_policy_entity_argument_is_nominal() {
    let f = Fixture::new();
    f.write("leave/src/leave/other.ti","module leave.other;pub entity Other{state status{Draft initial;}}create Other(){allow true;}pub policy Permit(x:Other)=true;");
    f.edit(
        "leave/src/leave/actions.ti",
        "use leave.events::{LeaveApproved};",
        "use leave.events::{LeaveApproved};\nuse leave.other::{Permit};",
    );
    f.edit(
        "leave/src/leave/actions.ti",
        "allow CanReview(self);",
        "allow Permit(self);",
    );
    assert_eq!(code(f.lock()), "TI_TYPE");
}
#[test]
fn pc_025_source_and_manifest_limits() {
    let f = Fixture::new();
    f.write("leave/src/leave/large.ti", &" ".repeat(262145));
    assert_eq!(code(f.lock()), "TI_LIMIT");
    let f = Fixture::new();
    let path = f.manifest();
    let text = fs::read_to_string(&path).unwrap() + &" ".repeat(65536);
    fs::write(path, text).unwrap();
    assert_eq!(code(f.lock()), "TI_LIMIT");
}
#[test]
fn pc_027_failed_relock_preserves_original() {
    let f = Fixture::new();
    f.lock().unwrap();
    let path = f.path.join("leave/Ti.lock");
    let original = fs::read(&path).unwrap();
    f.write(
        "leave/src/leave/invalid.ti",
        "module leave.invalid; invalid;",
    );
    assert!(f.lock().is_err());
    assert_eq!(fs::read(path).unwrap(), original);
    assert!(!fs::read_dir(f.path.join("leave")).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".Ti.lockfile().")));
}
#[test]
fn cli_projects_and_legacy_are_separate() {
    let f = Fixture::new();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ti"))
        .args([
            "lock",
            "--manifest",
            f.manifest().to_str().unwrap(),
            "--workspace-root",
            f.path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ti"))
        .args([
            "check",
            "--manifest",
            f.manifest().to_str().unwrap(),
            "--workspace-root",
            f.path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(ti_lang::compile("module leave; pub event E{}").is_err());
}
#[test]
fn pc_025_dependency_depth_including_cached_subgraphs() {
    fn chain(f: &Fixture, count: usize) {
        for i in 0..count {
            let mut manifest=format!("schema=\"ti.project/1\"\n[package]\nname=\"acme.p{i}\"\nversion=\"1.0.0\"\nedition=\"ti/0.2\"\nprofile=\"approval-package/0.2\"\nroot=\"p{i}\"\nkind=\"library\"\n[dependencies]\n");
            if i + 1 < count {
                manifest += &format!(
                    "next={{package=\"acme.p{}\",version=\"1.0.0\",path=\"../p{}\"}}\n",
                    i + 1,
                    i + 1
                );
            }
            f.write(&format!("p{i}/Ti.toml"), &manifest);
            f.write(&format!("p{i}/src/p{i}.ti"), &format!("module p{i};"));
        }
    }
    let f = Fixture::new();
    chain(&f, 16);
    assert!(project::lock(&f.path.join("p0/Ti.toml"), &f.roots()).is_ok());
    let f = Fixture::new();
    chain(&f, 17);
    assert_eq!(
        code(project::lock(&f.path.join("p0/Ti.toml"), &f.roots())),
        "TI_LIMIT"
    );
    let f = Fixture::new();
    chain(&f, 25);
    f.write("p0/Ti.toml","schema=\"ti.project/1\"\n[package]\nname=\"acme.p0\"\nversion=\"1.0.0\"\nedition=\"ti/0.2\"\nprofile=\"approval-package/0.2\"\nroot=\"p0\"\nkind=\"library\"\n[dependencies]\na={package=\"acme.p10\",version=\"1.0.0\",path=\"../p10\"}\nz={package=\"acme.p1\",version=\"1.0.0\",path=\"../p1\"}\n");
    assert_eq!(
        code(project::lock(&f.path.join("p0/Ti.toml"), &f.roots())),
        "TI_LIMIT"
    );
}
#[cfg(unix)]
#[test]
fn pc_027_lock_symlink_never_overwrites_target() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    f.write("untouched.txt", "original");
    symlink(f.path.join("untouched.txt"), f.path.join("leave/Ti.lock")).unwrap();
    assert_eq!(code(f.lock()), "TI_PROJECT");
    assert_eq!(
        fs::read_to_string(f.path.join("untouched.txt")).unwrap(),
        "original"
    );
}
#[test]
fn pc_004_release_and_model_revision_are_independent() {
    let f = Fixture::new();
    let a = f.lock().unwrap().compile().unwrap();
    f.edit(
        "leave/Ti.toml",
        "version = \"0.1.0\"",
        "version = \"0.1.1\"",
    );
    let b = f.lock().unwrap().compile().unwrap();
    assert_eq!(a.model_version(), b.model_version());
    assert_ne!(
        a.exports()["LeaveRequest"].handle,
        b.exports()["LeaveRequest"].handle
    );
    f.edit("leave/Ti.toml", "revision = 1", "revision = 2");
    assert_eq!(f.lock().unwrap().compile().unwrap().model_version(), 2);
}
#[test]
fn pc_026_declaration_order_keeps_linked_model_order() {
    let a = Fixture::new();
    let b = Fixture::new();
    let path = "leave/src/leave/events.ti";
    let original = fs::read_to_string(a.path.join(path)).unwrap();
    let (header, event) = original.split_once("pub event").unwrap();
    let note = "pub event Note{field text:String;}";
    a.write(path, &format!("{header}{note}\npub event{event}"));
    b.write(path, &format!("{header}pub event{event}\n{note}"));
    let x = serde_json::to_value(a.lock().unwrap().compile().unwrap()).unwrap();
    let y = serde_json::to_value(b.lock().unwrap().compile().unwrap()).unwrap();
    assert_eq!(x["checked"]["model"], y["checked"]["model"]);
    assert_ne!(x["graph_sha256"], y["graph_sha256"]);
}
