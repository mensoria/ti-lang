# Compiler layering and implementation packages

Status: design revision 1. ARC clauses define the target interfaces and the
behavior-preserving extraction described in the [implementation plan](implementation-plan.md).
[Project/package syntax](project-and-packages.md) remains a draft next edition;
its loader/import/offline-evaluation slice is implemented.

## 1. Separate language, build and execution authority

**ARC-001.** The language implementation has seven logical layers:

| Layer | Input → output | Owns | Must not do |
| --- | --- | --- | --- |
| Project loader | Explicit roots/manifests → immutable source snapshot | Files, dependency locations, byte limits, source IDs | Authenticate users or activate models |
| Syntax | Source unit → unchecked AST + spans | Tokens, grammar, source-local syntax diagnostics | Treat AST as executable |
| Semantic analysis/linking | AST graph → checked IR or diagnostics | Names, types, visibility, effects, cycles, entity operation groups | Fetch dependencies or query data |
| Artifact construction | Checked compilation + build provenance → frozen artifact | Versioned serialization, digests, dependency closure, capabilities | Grant permission or load unchecked IR |
| Pure evaluation | Checked IR + fixed context/record/input → candidate/events | Deterministic expression semantics, shared budgets, invariants | Read filesystem/clock/network/database |
| Host adapter | Exact active artifact + authenticated command → original outcome | Admission, current authority, transactions, result disclosure | Reinterpret language rules or retry with a fresh key |
| CLI/tooling | User intent → calls through the above interfaces | Output, exit status, explicit local I/O | Duplicate semantic or authorization checks |

Project loading happens outside the compiler. Compilation takes immutable bytes
and trusted build configuration, so a CLI and a future editor can use the same
compiler without giving it ambient filesystem access. Parse/check failure returns
no executable object. A candidate is not a committed record.

**ARC-002.** Physical packages follow existing independent responsibilities, not
one crate per phase. The current workspace has six packages, all containing real
implementation. AST, semantic checks and the evaluator share `ti-language` until
a resolved IR actually justifies a separate public representation. This keeps the
checked-model constructor private and avoids exposing unchecked executable data
merely to cross a crate seam.

```text
ti-lang (compatibility library + ti executable shell)
  ├── ti-language
  ├── ti-artifact ───────→ ti-language
  ├── ti-project ────────→ ti-language
  ├── ti-cli ────────────→ ti-language, ti-artifact, ti-project
  └── ti-host-mensoria ──→ ti-language, ti-artifact  [optional]
```

Arrows mean compile-time dependency. No leaf depends on `ti-lang`; the language,
artifact and CLI libraries do not depend on the Mensoria adapter. SurrealDB and
Tokio belong to the host package and qualification harness, not the language
compiler. No new runtime, scheduler, generic plugin engine or empty host trait
package is introduced.

## 2. Package contracts

**ARC-003.** `ti-language` owns scalar/nominal values, source diagnostics, syntax,
semantic validation, checked model construction, pure evaluation and the normative
wire codec/framing. Its small external interface is:

```rust,ignore
compile(source: &str) -> Result<Compiled>
compile_bytes(source: &[u8]) -> Result<Compiled>
Compiled::evaluate(entity, action, actor, inputs, old, new_id) -> Result<Candidate>
Compiled::validate_record(entity, record) -> Result<()>
Compiled::module_name() -> &str
Compiled::model_version() -> i64
Compiled::ir_bytes() -> Result<Vec<u8>>
```

Existing signatures and harness encodings stay compatible. AST/model fields and
constructors remain private. `Compiled` cannot be deserialized from JSON.
Metadata access is not a validation bypass. Existing public metadata fields are
legacy compatibility surfaces; only frozen artifacts are eligible for host use.
Their later removal requires an explicit Rust interface revision.

Internally, `syntax` owns lexing/parsing; `model` owns the current expression/model
representation; `check` owns name/type/effect/state validation; `evaluator` owns
candidate production; `wire` owns exact encoding and equality framing. These are
actual modules with distinct responsibilities, not promises of separate crates.
The existing checked representation remains string-bound IR schema 0.1; it must
not be described as the future fully resolved typed IR.

**ARC-004.** `ti-artifact` owns `Artifact::freeze`, manifest serialization,
immutable identity and registry publication. It consumes only the checked model's
read-only interface and canonical IR bytes. It does not inspect private AST
fields or create a model by deserializing an artifact. Compilation fingerprints
cover all actual semantic/parser/evaluator/codec source inputs and the declared
lockfile. Actual consuming-workspace dependency resolution remains separately
recorded build evidence. A source move legitimately changes compiler identity.

**ARC-005.** `ti-host-mensoria` owns native `Command`, `Receipt`, `SessionBinding`,
role mappings, fixed action/delivery transactions, maintenance installation and
activation, and current-result checks. It receives connections; it does not create
root runtime connections. The existing `MensoriaHost::bind/execute`, delivery and
closed-activation interfaces stay source compatible through the facade.
Qualification fault hooks remain behind `host-qualification` and are absent from
ordinary host builds. SQL lives with the host package and is part of its source
fingerprint. No host code is reachable from a pure evaluator call.

**ARC-006.** `ti-cli` owns argument dispatch, bounded file reading, serialization,
stdout/stderr and exit status. Its `run(args)` interface is usable from the root
executable shell. There is one `ti` binary target; adding a CLI library does not
create two conflicting workspace binaries. Initial extraction preserves
`check`, `compile`, `freeze`, `eval` and their existing error/exit behavior. Proposed
project commands are added only with their language/project implementation.

**ARC-007.** Root `ti-lang` is a compatibility facade. It re-exports existing
language types/functions, `artifact` and `wire` module paths, and the feature-gated
`mensoria` path. Existing Mensoria source integration continues depending on it.
The facade owns no parser, validator, transaction or expression logic. Ordinary
builds stay free of native host dependencies. Root integration tests continue to
exercise the same external interface as consumers.

## 3. Compiler data contracts

**ARC-008.** The loader now returns an immutable `Project` snapshot (the implementation of the
planned `ProjectSnapshot`) containing
validated manifest semantics, exact source units `(SourceId, relative path,
immutable UTF-8 bytes)`, locked dependency edges and source/graph digests. It is
immutable after checking the filesystem. Open editor buffers are explicit
snapshot replacements with content digests, never unsaved disk assumptions.
The compiler gets no paths it may open and no implicit environment configuration.
`ti-project::lock/open` own manifest parsing, safe Unix descriptor-relative reads,
lock comparison and atomic lock writing. `Project::compile` passes source snapshots
to `ti-language::project::compile_packages`. Import names lower to collision-checked
internal handles retaining full defining symbol tuples. The development format
`ti.project-ir/0.2-dev.1` reuses checked expression evaluation; complete source spans,
a final typed IR ABI and a deployable qualified-ID codec remain later gates.

**ARC-009.** Next-edition syntax AST nodes carry half-open source spans and
unresolved names; successful parsing conveys no type or effect guarantee.
Semantic analysis runs declaration collection, import/export resolution,
nominal-type assignment, operation-group linking, expression/type/context
checking, policy-cycle detection and limit checking before producing `CheckedModel`.
Resolved IR uses stable symbol/type/operation handles, checked operand/call types,
source spans, deterministic operation/event order and a versioned intrinsic table.
Only the validating constructor may issue its checked handle. Public editable AST
or decoded IR must be validated again before evaluation or freezing.

**ARC-010.** IR serialization and source maps are separate, versioned formats.
Serialization order follows stable symbol identity, not hash-map iteration,
filesystem enumeration or allocation order. Source bytes, package graph,
compile configuration, limits and compiler/evaluator identity all participate in
build identity. Moving sources can change the artifact even if the semantic IR
is identical. A deterministic build does not mean different compiler releases
have the same artifact identity.

**ARC-011.** Semantic and runtime failures use structured diagnostics; rendering
belongs to the CLI/editor. The next-edition interface uses a diagnostic collection
with deterministic ordering `(SourceId, byte_start, code)`, primary/related spans
and phase. Runtime public errors additionally carry an observable original
operation reference and reconciliation hint under current disclosure rules.
Existing line/column diagnostics are retained during extraction and are not
claimed to satisfy complete span requirements. No new diagnostic engine is
fabricated by wrapping all errors in a generic package failure.

## 4. Execution and dependency discipline

**ARC-012.** A host command first resolves current trusted scope/identity and the
original key, then selects the applicable immutable artifact or reconciles the
original outcome, evaluates its selected operation, and commits through the fixed
native transaction. It rechecks admission/authority at commit and result release.
Package/link metadata cannot widen role membership, data visibility or external
side-effect permission. The pure evaluator keeps simultaneous assignments,
candidate-event reads and its operation-wide budget after extraction.

**ARC-013.** Dependencies are passed explicitly. The language and artifact
packages can be compiled/tested without a database. No workspace package performs
network or database work during compilation/build scripts. The existing pinned
backend downloader and native qualification harness are explicit developer/CI
tasks. A future backend adapter must qualify the same behavioral host contract;
a generic adapter interface is introduced only when a second real implementation
requires it. The current Mensoria SessionBinding is not a portable authentication
object and cannot be created from public action JSON.

**ARC-014.** Evidence follows the changed interfaces. Compiler/evaluator/codec
checks remain language evidence; source adapter compilation remains integration
build evidence; real database and TCP fault tests remain host evidence. Moving
files changes source and implementation fingerprints: new reports must enumerate
every crate source and Cargo manifest, preserve previous failed/successful runs,
and requalify the actual consuming Mensoria workspace. No directory move upgrades
an old local result into next-edition or production qualification.
