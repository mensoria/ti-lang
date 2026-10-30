# Project model and compiler delivery plan

Status: design revision 1; gates A/B are implemented and the manifest/lock/module
slice of gates C/D is implemented. Final spans, deployable artifacts and 0.2 host
activation remain separate. This plan separates a behavior-preserving workspace
extraction from implementation of the proposed next language edition. The
[package standard](project-and-packages.md), [architecture](compiler-architecture.md)
and [acceptance matrix](package-conformance.md) define the scope.

## Decisions

**MIG-001.** Preserve `ti/0.1` and its single-source grammar during extraction.
Do not implement half of `use`, infer packages from directories, or relabel the
75 existing regression tests as package qualification. New project syntax targets
`ti/0.2` and `approval-package/0.2` only. Each stage has its own evidence and may
ship independently without implying later stages are available.

**MIG-002.** The extraction uses this ownership map:

| Existing implementation | Destination | Interface constraint |
| --- | --- | --- |
| `src/lib.rs` values/diagnostics/model/check/compile | `crates/ti-language/src/{lib,model,check}.rs` | Checked constructor stays private |
| `src/parser.rs` | `crates/ti-language/src/syntax.rs` | Existing token/AST rules |
| `src/evaluator.rs`, `src/wire.rs` | `crates/ti-language/src/` | Existing evaluation and encoding bytes |
| `src/artifact.rs` | `crates/ti-artifact/src/lib.rs` | Use checked getters/IR bytes |
| `src/mensoria*.rs`, fixed SQL | `crates/ti-host-mensoria/src/` | Native transaction text/authorization unchanged |
| `src/main.rs` dispatch | `crates/ti-cli/src/lib.rs` | Root keeps the single `ti` executable |
| Root library and binary | `src/{lib,main}.rs` | Re-exports and dispatch only |
| `tests/`, native fixtures, integration source | Keep at root | Consumer-facing regression surface |

No empty lexer/type/IR/LSP crate is added. Compiler logical layers remain explicit
internal modules until separate consumers and representation contracts justify
further extraction. The filesystem project loader is now the sixth actual package, `ti-project`,
depending on the compiler rather than being called from within it.

## Delivery gates

**MIG-003.** Gate A — design and publication: paired English/Chinese PKG, ARC,
MIG and PC identifiers; identical syntax/examples; a complete approval package
layout; explicit implementation-status labels; navigation/search entries; link
and bilingual checks. This gate runs no package code and proves no new semantics.

**MIG-004.** Gate B — workspace extraction: create the four implementation crates
and compatibility facade; centralize Cargo dependency versions; retain root
commands/features; preserve existing tests without weakening assertions; expose
only checked read-only IR access needed by artifact construction. Acceptance:
workspace formatting/linting, default build without host dependencies, all 44
numbered scenarios, CLI behavior, native action/delivery/fault tests and actual
Mensoria adapter compilation. Inspect dependency edges for cycles and native SDK
leaks. Regenerate source fingerprints and receipts. Syntax and import support
remain unchanged.

**MIG-005.** Gate C — source infrastructure and project loading: introduce
SourceId/spans, manifest and lock schemas, bounded safe traversal, explicit
workspace roots, snapshot-based compilation, offline full-graph validation and
atomic lock writing. No network dependency fetching. Execute PC-001–PC-008,
PC-016–PC-022, PC-025 and PC-027 before adding semantic imports. Keep the legacy
compiler's entry point and limits independently qualified.

**MIG-006.** Gate D — module syntax and semantic linking: implement the new
header/visibility/use grammar only under the new edition, deterministic symbol
IDs, module import/export DAGs, visibility/public closure, nominal ID resolution,
closed entity operation groups and root activation exports. Replace unresolved
names with checked symbol/type handles. Execute PC-009–PC-015, PC-023–PC-024 and
PC-026; then run every static/evaluator package scenario together. Unsupported
constructs must reject before artifact creation.

**MIG-007.** Gate E — deterministic package artifacts: specify and freeze exact
IR/artifact/source-map/qualified-ID wire formats and golden byte fixtures;
implement the currently reserved 0.2 codec before claiming 0.2 execution support.
Add inspect/build commands, dependency-closed digests, cache verification and
immutable package/model publication. Test moved workspaces, reordered discovery,
changed transitive dependencies, edited manifests, corrupted cache and forged
artifact input. Activation remains disabled until the new host binding is ready.

**MIG-008.** Gate F — native package activation: keep existing admission and
maintenance ownership; bind exposed entity symbols to managed collections;
requalify current scope/roles, old-operation retention, schema compatibility,
revision contention, atomic audit/operation/outbox and generic-write protection
with qualified type/event identifiers. Verify rollback, actual response loss,
cancellation and old-protocol reconciliation. Execute PC-028–PC-030 against the
actual target Mensoria migration stack. Successful compilation alone cannot
activate a package or migrate existing records.

**MIG-009.** Gate G — tooling: project-aware CLI diagnostics and source maps,
editor snapshots, symbol navigation, model review and activation diff. Add LSP as
a real consumer only after SourceId/import resolution contracts are stable.
Network registries, dependency solver, publishing/signing, scalar/generic policy
parameters, cross-record execution, entity adoption, new date/money semantics and
additional database adapters remain explicit later proposals.

## Compatibility and recovery

**MIG-010.** Extraction preserves serialized 0.1 IR, payload codecs, evaluation
results, error codes, original-key behavior and host SQL semantics for identical
inputs. It does not promise unchanged compiler/artifact hashes: fingerprint inputs
and Cargo.lock change with the workspace. Existing deployed heads must not be
silently overwritten to accept the new host fingerprint. Maintenance activation
must explicitly review/requalify a new artifact/revision, while old operation
receipts remain readable under their original artifact identity and current
result authorization. Tests use disposable native environments; no shared DB
schema or active head is changed by restructuring the source checkout.

Record baseline and post-extraction source manifests, actual Cargo dependency
locks and native backend versions. Preserve failing runs. Roll back source by a
normal version-control change; do not repair a failed runtime rollout by
re-executing old operations under new rules. A future 0.1→0.2 upgrade needs an
explicit type/storage/codec migration and dual-protocol original-result reading;
it is not a source-file relocation command.
