# Language projects, packages and modules

Status: standard proposal `package-model/0.2`, design revision 1. English is the
normative proposal; Chinese uses the same PKG identifiers, grammar and examples.
These rules target edition `ti/0.2` and profile `approval-package/0.2`. They are
accepted only through the project entry point; the legacy `ti/0.1` single-file
entry point still rejects them. Manifest/lock loading, module imports and offline
evaluation are implemented; the 0.2 deployment/artifact protocol is not. The existing
[single-source standard](ti-language-standard.md) remains unchanged. See [implementation status](implementation.md) for the supported slice.

## 1. Terms and units

**PKG-001.** A project is a filesystem workspace. A package is a versioned source
and dependency unit. A module is a named declaration namespace inside a package.
A model is a host activation unit with a stable logical name and integer revision.
A Rust crate is an implementation unit and never a Ti import target. A source
file is not a deployment, transaction, identity or permission unit.

**PKG-002.** A package has exactly one `Ti.toml`, one source root `src/`, and one
root module. All regular `.ti` files under `src/` belong to it, including modules
not imported by the root. Every file is checked; unused files cannot hide invalid
or privileged declarations. Nested manifests, generated source discovery and
conditional compilation are rejected in this profile. A workspace may contain
multiple packages, but builds and activates explicit package roots independently.

```text
leave/
  Ti.toml
  Ti.lock
  src/
    leave.ti
    leave/model.ti
    leave/policies.ti
    leave/actions.ti
    leave/events.ti
```

## 2. Manifest and version domains

**PKG-003.** The manifest is UTF-8 TOML with schema `ti.project/1`; unknown keys,
duplicate keys, missing required fields and conflicting aliases fail validation.
There is no executable manifest, build script or environment-variable expansion.
The initial complete shape is:

```toml
schema = "ti.project/1"

[package]
name = "acme.leave"
version = "0.1.0"
edition = "ti/0.2"
profile = "approval-package/0.2"
root = "leave"
kind = "model"

[model]
name = "leave"
revision = 1

[dependencies]
approval_events = { package = "acme.approval_events", version = "1.2.0", path = "../approval-events" }
```

`kind` is `model` or `library`. A model requires `[model]`; a library forbids it
and cannot be activated directly. A library can contain complete entities and
pure declarations; it does not gain storage authority by being imported.
`edition` and `profile` must be supported exact strings, not version ranges.
`root` is one identifier. Initial source roots and filenames are fixed, not
configurable globs.

**PKG-004.** Package names are two or more dot-separated lowercase ASCII segments
matching `[a-z][a-z0-9_]*`, at most 63 bytes per segment and 191 bytes total.
Dependency aliases and module segments are case-sensitive Ti identifiers, at most
63 ASCII bytes, excluding keywords. Portable source paths are lowercase; a
case-folded collision fails even on a case-sensitive filesystem. A dependency
alias cannot equal the local root module. Package versions are exact stable
`major.minor.patch` decimal triples with no leading zeros, prerelease or build
metadata; each component is at most 2147483647. Ranges and multiple versions of
the same canonical package in one resolved graph are not supported initially.

**PKG-005.** Edition, profile, package release, model revision, artifact identity,
and runtime incarnation are separate domains. Model revision is 1–2147483647
and belongs to the model's stable host name. Releasing package 0.2.0 does not
activate model revision 2. A new artifact for an already published model revision
requires a new model revision under the existing immutability contract. Package
publication also forbids replacing the content of a published name/version.

## 3. Source paths and namespaces

**PKG-006.** `src/leave.ti` declares `module leave;` and
`src/leave/policies.ti` declares `module leave.policies;`. The logical module path
must exactly match its source path without `.ti`. One file declares one module;
module declarations cannot be reopened in another file. Every source path is
relative, normalized with `/`, sorted by UTF-8 bytes, and recorded in the source
map. Symlinks, `..`, absolute source paths, case collisions, files outside the
package root, and invalid UTF-8/BOM/lone CR are rejected. Comments and CRLF follow
the baseline lexical rules; raw source hashing does not normalize line endings.

**PKG-007.** Entity, policy and event declarations share the owning module's
namespace. Import aliases participate in that namespace. Names are resolved from
explicit declarations/imports only; no global prelude, filename inference,
recursive parent lookup or last-declaration-wins rule exists. Reordering files or
declarations cannot change symbol resolution. Entity IDs and action selectors are
resolved using the rules below, not string concatenation at runtime.

## 4. Imports and visibility

**PKG-008.** Declarations are module-private unless marked `pub`. `use` imports an
explicit named public declaration; `pub use` additionally re-exports it. Neither
form evaluates code. Wildcards, module-wide imports, dynamic paths, filesystem
paths in source, implicit imports and host/native imports are forbidden.

```text
module leave;
pub use leave.model::{LeaveRequest};
pub use leave.events::{LeaveApproved};
```

```text
module leave.actions;
use leave.model::{LeaveRequest};
use leave.policies::{Reviewer as CanReview};
use leave.events::{LeaveApproved};
```

A module path begins with the local root module or one manifest dependency alias.
For a dependency, the alias replaces that package's root segment: importing
`approval_events.events::{DecisionRecorded}` maps to its declared root module
plus `.events`. Imports must name an actual module and exported declaration.
There are no qualified expression calls: an imported local alias is used in the
existing expression grammar.

**PKG-009.** A declaration cannot import or re-export a private name. A re-export
retains its defining symbol identity and cannot change its nominal type or grant
new authority. Module import cycles (including re-export cycles), name collisions, duplicate aliases and implicit
shadowing fail. Public signatures must have a complete public type closure:
referenced entities, nominal IDs and emitted event types must be exported and
resolvable by consumers. Public entities expose their field/state schema; this is
compile-time visibility, not permission to read stored values.

**PKG-010.** Exporting entity `E` also exports its nominal `EId` under the same
visibility. Importing `E` does not automatically import `EId`; importing either
requires an explicit item. A generated ID name collision fails. State symbols
remain scoped to their entity and are never standalone exports. Existing
nullable, comparison and invariant-context rules still apply.

**PKG-011.** Actions and creation declarations belong to their target entity,
not to a global function namespace. They cannot be imported, aliased or re-exported
as independent functions. Different modules in the defining package may declare
operations for that package's entities after explicitly importing the entity.
The compiler links the complete operation group: exactly one creation per entity,
unique action names per entity, no final-state outgoing actions, and the existing
initialization/invariant/event obligations.

**PKG-012.** Only a defining package can add creation/actions or change an entity's
fields, states or invariants. Importing or re-exporting a foreign entity cannot
attach actions, override authorization or reopen its schema. This proposal has no
inheritance, partial entities, traits, extension methods or policy replacement.
Libraries obey the same closed entity-group checks as models.

## 5. Nominal identity and model composition

**PKG-013.** A symbol ID is `(canonical package name, exact package version,
module path, declaration kind, declaration name)` encoded by a versioned framed
tuple. An entity ID type refers to that defining symbol, not its import alias,
source pathname or spelling in the consumer. A lock graph permits one version of
each package. Changing a package version changes its nominal symbol IDs and
requires explicit compatibility/migration analysis, even if a name is unchanged.
A content hash distinguishes immutable source artifacts; it does not replace
nominal identity or authenticate a publisher.

**PKG-014.** A model's public root entity exports define its activated entity
set. Its own unexported entities are checked but not exposed as action targets.
This initial profile forbids root re-export/activation of dependency-owned
entities. Those may be imported as nominal schema/ID/policy dependencies but do
not register collections or executable actions in the consumer. Multi-model
composition, entity adoption and scalar/generic policy parameters require later
proposals; this standard does not promise that every entity-parameter policy can
be reused with an unrelated local entity. Shared event declarations and modules
inside one package are immediately useful without weakening nominal typing.

**PKG-015.** Exported policies keep their existing single entity parameter and
pure-expression restrictions. Dependencies must be acyclic; a shared policy
cannot introduce a cycle back to its consumer's entity package. No implicit type
projection, structural entity compatibility or cross-record lookup is introduced.
Calling any linked policy consumes the same operation-wide evaluation budget.
Only the authenticated host supplies `actor`; imported code cannot supply role
membership or expand the host capability set.

## 6. Dependency resolution and lockfile

**PKG-016.** Initial dependency sources are explicit local paths only. Paths in
`Ti.toml` resolve relative to that manifest; dependency roots may be siblings,
but must be within caller-approved workspace roots. Canonical symlink traversal,
network paths, source-level filesystem traversal and manifest redirection are
rejected. In-memory library callers provide an explicit immutable package
snapshot and trusted dependency mapping instead of granting filesystem access.
Package resolution is a CLI/project-loader task; compilation itself has no I/O.
The CLI defaults to the root package directory as its allowed workspace root;
repeatable `--workspace-root DIR` options explicitly authorize additional roots.
Sibling dependencies in the example require `--workspace-root .` when invoked
from their common parent. This is an input to the loader, not package authority.

**PKG-017.** `ti lock` validates the entire graph and atomically writes `Ti.lock`
via a temporary file/rename only after success. It performs no network fetching.
It is the only initial command that updates dependency resolution. Normal
`check`/`build`/CI require the lockfile and never repair it silently. Changed manifest semantics, source package, alias, version, content digest
or graph edge makes the lock stale. Moving a dependency locator to identical
verified content does not change lock identity. Missing or stale locks fail before semantic compilation. Workspace
packages are not automatically trusted or resolved by search path precedence.

**PKG-018.** `Ti.lock` uses UTF-8 JSON schema `ti.lock/1`, rejects duplicate and
unknown keys, and stores a sorted full package graph. Each package record has:
`name`, `version`, `edition`, `profile`, `root`, `kind`, `content_sha256`, and a
sorted dependency alias map to exact name/version/content digest. Package records
are sorted by `(name, version)`; each reference is an object with `name`, `version`
and `content_sha256`, and each digest is 64 lowercase hexadecimal characters.
`root_package` uses the same reference shape. The top level
has `schema`, `root_package`, `packages`, and `graph_sha256`. Source locations,
absolute paths, timestamps, user identities and caches are excluded. The manifest
retains source paths; the loader cross-checks the lock records against them.

**PKG-019.** Package content identity uses SHA-256 with domain
`ti.package-source/1` and the existing `F(tag, bytes)` framing: `F('d', domain)`,
`F('m', canonical manifest semantics)`, then for every source in sorted order
`F('p', relative path) || F('s', exact source bytes)`. Canonical manifest semantics
is compact UTF-8 JSON with recursively lexicographically sorted ASCII field keys,
no insignificant whitespace, and the validated semantic values of all fields,
excluding dependency `path` locators. Arrays retain order; dependency aliases are
sorted object keys. Unknown fields fail before hashing. The graph digest uses
`F('d', "ti.package-graph/1") || F('g', canonical JSON of root_package and sorted
package records)`, excluding `graph_sha256` itself. Changing an import alias or
source path changes identity; moving an identical dependency directory does not.

**PKG-020.** A package's imported symbol table binds its exact locked dependency
graph. Identical root sources with changed transitive content must produce a
different build/artifact identity. Lockfiles and digests detect differences; they
are not signatures. A cache hit must match toolchain, edition/profile, limits,
source content and the entire dependency graph, and must verify stored content.
No cache entry or unchecked IR may bypass semantic validation or host activation.
Registries, Git sources, semver solving, signatures and package publishing are
outside this initial profile.

## 7. Build, artifact and activation

**PKG-021.** The proposed pipeline is `load → lock check → parse all files →
resolve imports/exports → check/link → emit checked IR → freeze artifact`.
There is no dead-code-based exemption from validation. The artifact records the
root model name/revision, package coordinates, full graph digest and dependency
records, source-map/content digests, typed/resolved IR schema/version, compiler
and evaluator fingerprints, profile/limits and required capabilities. A model
build is atomic: an error produces diagnostics and no new runnable artifact.
Artifact JSON is not a trusted constructor for checked IR.

**PKG-022.** Building does not activate. A deploy request presents an exact root
artifact and explicit host bindings for each exposed entity. Dependency modules
cannot activate themselves or gain a transaction, credential, policy store or
external side effect. Existing single-record transactions, revision CAS, audit,
outbox, original-key reconciliation and maintenance activation rules remain in
force. A package dependency is not a database relation or distributed transaction.

**PKG-023.** Edition 0.2 uses a new artifact/IR and host protocol version. Its
request/event nominal ID tags refer to defining symbol IDs, with a versioned
symbol table included in the artifact. The exact new transport codec and golden
byte fixtures are a prerequisite to executable 0.2 host qualification; 0.1's
short-name tags and fingerprints must not be reused for qualified identities.
No 0.2 artifact can be loaded by the 0.1 host through feature detection. Existing
0.1 operation results retain their original artifact and protocol on upgrade.

## 8. Limits, diagnostics and commands

**PKG-024.** Initial deterministic project limits are: 32 packages including the
root, 16 dependency edges per package, dependency depth 16 (root is depth 1),
128 source files per package, 256 KiB per source, 2 MiB source bytes per package,
8 MiB across the graph, 256 declarations per package and 1024 across the graph.
Manifest maximum is 64 KiB and lock maximum 256 KiB; public exports and import
bindings are each at most 256 per package. Baseline expression/scalar/action
limits and the shared 100000 evaluation-node budget are unchanged. Enumerating
files stops at the first one-over condition before reading their contents.
Source traversal additionally bounds directory depth at 32, inspected entries at
4096 per package and dependency-location resolutions at 512 per load. The 8 MiB
read budget also includes source bytes inspected through duplicate locators.
Future resource changes require a named profile revision.

**PKG-025.** Project diagnostics include code, phase, sanitized message, primary
`(SourceId, byte_start, byte_end)` span and related import/declaration spans when
attributable. Byte ranges are half-open UTF-8 offsets; display lines/columns are
computed by the source map. Manifest/lock errors use their own SourceIds. No
absolute paths or credentials are exposed in public output. Stable categories:
`TI_PROJECT`, `TI_PACKAGE`, `TI_LOCK_STALE`, `TI_IMPORT`, `TI_VISIBILITY`,
`TI_DEPENDENCY_CYCLE`, plus existing syntax/name/type/model/limit codes. A failure
must identify its package/module context and never start host work.

**PKG-026.** Proposed commands are:

| Command | Inputs and result | Writes or authority |
| --- | --- | --- |
| `ti init DIR --name acme.leave` | Explicit new project skeleton | Refuse nonempty DIR; no network |
| `ti lock --manifest Ti.toml` | Full explicit path graph → validated Ti.lock | Atomic lock replacement only |
| `ti check --manifest Ti.toml` | Locked snapshot → diagnostics | No lock repair, artifact or DB write |
| `ti build --manifest Ti.toml --out DIR` | Locked snapshot → artifact/source map | Atomic generated output; no activation |
| `ti inspect ARTIFACT` | Verify structure/digests → readable metadata | No execution; cannot bless untrusted IR |

`lock` and `check --manifest` are implemented. `compile --manifest` inspects the
current development IR and `eval --manifest` runs an offline candidate harness.
`init`, deployable `build`, and `inspect` remain design contracts. Existing
`ti check/compile/freeze/eval FILE.ti` remain 0.1 entry points. There is no automatic
conversion of a legacy module version into a package release or model revision.

## Appendix A. Package syntax delta

Only this delta replaces the 0.1 source header; all other baseline constructs
remain unless explicitly changed above. `pub` applies to entity/event/policy
only. Actions and creations are package-owned as specified in PKG-011.

```ebnf
source       = "module", module_path, ";", { item } ;
module_path  = IDENT, { ".", IDENT } ;
item         = use_decl | [ "pub" ], ( entity | policy | event )
             | creation | action ;
use_decl     = [ "pub" ], "use", module_path, "::", "{", use_item,
               { ",", use_item }, "}", ";" ;
use_item     = IDENT, [ "as", IDENT ] ;
```

## Appendix B. Approval package walkthrough

The [package example](../examples/package-design/README.md) gives every source
file, manifest and expected link result. It is now a runnable project compiler example with a generated lockfile; it does
not activate a database model. See [package acceptance](package-conformance.md)
and [compiler architecture](compiler-architecture.md) for delivery gates.
