# Package and module acceptance

Status: implemented project regressions with partial acceptance coverage.
The [project receipt](qualification/projects-latest.json) records actual named
checks; unexecuted rows remain unexecuted. Current checks cover loading/locking,
imports/visibility, linking, nominal identities, selected limits and offline
approval. They do not establish complete span diagnostics, all resource
boundaries, cache/artifact verification or 0.2 host qualification. These PC cases
are separate from the 44 baseline TC scenarios. Authority:
[package proposal](project-and-packages.md).

| ID | Scenario | Required result | Clauses |
| --- | --- | --- | --- |
| PC-001 | Valid model/library manifests | Exact schema and kind/model requirements | PKG-001–PKG-003 |
| PC-002 | Unknown/duplicate manifest fields, executable hook | Reject before loading source | PKG-003 |
| PC-003 | Name/version limits, ranges, duplicate package versions | Exact acceptance/rejection; no implicit solving | PKG-004–PKG-005 |
| PC-004 | Model revision vs package release | Separate identities; no activation from release | PKG-005 |
| PC-005 | Valid multi-file approval example | Every module checked and correctly linked | PKG-002, PKG-006 |
| PC-006 | Path/header mismatch, duplicate or case-colliding modules | Reject deterministically | PKG-006 |
| PC-007 | Symlink/absolute/parent traversal/network path | Reject, no read outside authorized roots | PKG-006, PKG-016 |
| PC-008 | Invalid unused source module | Whole build fails, no runnable artifact | PKG-002, PKG-021 |
| PC-009 | Explicit import and alias | Resolve defining symbol independently of alias | PKG-007–PKG-008 |
| PC-010 | Private import/re-export or wildcard | Reject with primary and related spans | PKG-008–PKG-009 |
| PC-011 | Alias collision, shadowing, re-export cycle | Reject, independent of file order | PKG-007–PKG-009 |
| PC-012 | Generated ID imports, collision, private signature type | Exact nominal/public closure checks | PKG-009–PKG-010 |
| PC-013 | Creation/actions spread across local modules | Link one complete entity operation group | PKG-011 |
| PC-014 | Foreign entity action/field/state extension | Reject before artifact or host work | PKG-012 |
| PC-015 | Root exports vs unexported/foreign entities | Only allowed root-owned set is activatable | PKG-013–PKG-015 |
| PC-016 | Valid sibling local path dependency | Resolve only through explicit authorized roots | PKG-016 |
| PC-017 | Missing/stale lock or changed manifest | Fail without implicit update | PKG-017 |
| PC-018 | Duplicate/unknown lock fields, wrong graph hash | Reject | PKG-018–PKG-019 |
| PC-019 | Dependency/package/module cycle | Reject with a bounded complete cycle trace | PKG-009, PKG-015–PKG-017 |
| PC-020 | Same content after moving checkout | Same locked source/graph identity | PKG-018–PKG-019 |
| PC-021 | Changed transitive dependency or import alias | Different graph/build identity or stale lock | PKG-019–PKG-020 |
| PC-022 | Corrupt cache/forged IR | Verify or recheck; never issue checked execution handle | PKG-020–PKG-021 |
| PC-023 | Equal short names from different defining symbols | Distinct nominal types; reject conflation | PKG-013, PKG-023 |
| PC-024 | Policy linkage and entity-parameter mismatch | Exact nominal checks and one shared budget | PKG-015 |
| PC-025 | Every project limit at maximum and one over | Bounded exact acceptance/rejection | PKG-024 |
| PC-026 | Reordered filesystem/declarations | Same symbol resolution, IR order and diagnostics | PKG-007, PKG-021, PKG-025 |
| PC-027 | Lock/build interrupted or init into nonempty directory | No partial output, no overwritten user files | PKG-017, PKG-021, PKG-026 |
| PC-028 | Activation without maintenance or dependency tries host I/O | Deny; no storage authority from imports | PKG-022 |
| PC-029 | New package/codecs and original 0.1 result | Exact original artifact/protocol; no replay under new rules | PKG-005, PKG-023 |
| PC-030 | Qualified entity action commit/rollback/lost response | Native atomicity, IDs, current result access and reconciliation | PKG-022–PKG-023 |

Evidence must record source snapshots, manifests/locks, source-map spans, exact
compiler/evaluator/codec versions, expected IR/symbol identities or diagnostics,
limits and observed outputs. Host rows additionally require actual target backend,
migration stack, bindings, fault timing and durable state. Acceptance requirements in
this table are not themselves test results. See [delivery gates](implementation-plan.md).
