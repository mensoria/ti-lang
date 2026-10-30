# Rust implementation and native qualification

Status: working reference implementation and optional Mensoria transaction host,
2026-10-30. All 44 numbered regression scenarios have executable checks. This is
not a claim of complete Ti standard, full Mensoria platform, or production
qualification. The checked-in [qualification receipt](qualification/latest.json)
records the exact source hashes, native backend and test results.

## Workspace organization

The implementation is extracted into `ti-language`, `ti-artifact`, `ti-cli` and
`ti-host-mensoria`, plus the filesystem loader `ti-project`, with the root `ti-lang` compatibility facade and single `ti`
binary. Source ownership and interfaces follow [compiler architecture](compiler-architecture.md).
`python3 scripts/check_workspace.py` checks the actual Cargo edges and default
host-free dependency closure. Project syntax uses the separate
[next-edition draft](project-and-packages.md); its loading/import/offline evaluator
slice is implemented and its [project regression receipt](qualification/projects-latest.json)
is separate from legacy/native qualification.
The existing 44-case report must be regenerated after source moves. New compiler
and host fingerprints require explicit maintenance review, not silent active-head
replacement. See [delivery and migration gates](implementation-plan.md).

## Run the language

Use Rust 1.96.0 and the checked-in Cargo.lock.

```sh
cargo run --locked -- check examples/approval.ti
cargo run --locked -- compile examples/approval.ti
cargo run --locked -- freeze examples/approval.ti
cargo run --locked -- eval examples/approval.ti examples/create-request.json
cargo test --workspace --locked
cargo clippy --workspace --locked --all-targets --all-features -- -D warnings
```

`check` validates source; `compile` prints inspectable IR; `freeze` prints a frozen
manifest; `eval` returns a candidate without writing a database. The offline
harness actor is test context, never an authenticated network identity.

Artifacts bind exact source, IR, compiler source fingerprint, declared lockfile,
edition/profile, dependency bindings, deterministic limits and required host
capabilities. Registry publication forbids changing a module/version. These
fingerprints identify source/build inputs, not binary attestations. A consuming
workspace must separately record its actual dependency lock and build evidence.

The evaluator uses an explicit work stack and one shared 100000-node budget.
Tests exercise exactly 100000 nodes and one over, deep acyclic policy calls,
simultaneous assignments, old/new invariants and candidate event values. Static
limits, 128 KiB input/event payload boundaries, scalar strings and nominal IDs are
checked. `wire` implements duplicate-key rejection, standard scalar/event tags,
canonical Int64 values and Appendix C fingerprint framing. Internal harness ID
objects are not accepted as public nominal scalar values.

## Run a locked project

```sh
cargo run --locked -- check --manifest examples/package-design/leave/Ti.toml --workspace-root examples/package-design
cargo run --locked -- compile --manifest examples/package-design/leave/Ti.toml --workspace-root examples/package-design
cargo run --locked -- eval --manifest examples/package-design/leave/Ti.toml --workspace-root examples/package-design examples/create-request.json
python3 scripts/verify_projects.py
```

After intentional changes, run `ti lock` with the same manifest/workspace options.
Normal loading is read-only and rejects missing/stale/invalid locks. Current
filesystem loading is qualified on Unix and uses directory descriptors with
O_NOFOLLOW; other platforms fail closed until an equivalent adapter is provided.
The project compiler accepts `use`, aliases and `pub use`, validates every module
and dependency, prevents foreign entity extension and executes the existing pure
approval semantics. Its development IR is `ti.project-ir/0.2-dev.1`, not a final
public typed IR ABI. `freeze --manifest` is rejected: no 0.2 database activation
or wire protocol is provided. Complete source/related spans remain pending.

## Run all 44 numbered scenarios

```sh
python3 scripts/setup_native_backend.py
TI_SURREAL_BIN="$PWD/.cache/native-backend/surreal" python3 scripts/verify_conformance.py
```

The setup script verifies a fixed archive SHA-256. The runner fails if the native
server is missing; it never silently skips H cases. Each native test starts its
own loopback-only SurrealDB 3.3.0 memory process and destroys only that process.
The [fixture manifest](../tests/fixtures/mensoria/manifest.json) pins five exact
Mensoria migrations. The source baseline had no commits, so hashes identify it.
`MENSORIA_SOURCE_ROOT` can recheck those files in another checkout; drift fails.

The runner retains timestamped logs and a machine-readable report under
`.cache/qualification/`, including failures. It maps observed Rust test names to
TC-001 through TC-044. More than one test can implement a case. CI compiles the
host feature and runs the same real-backend scenarios on Linux; local results
qualify only the OS/backend/source tuple in their receipt.

## Mensoria host contract

Enable `mensoria-host`. Runtime uses a record-authenticated user connection and an
environment-scoped `data_issuer` connection. It never uses root credentials for
actions. `SessionBinding` comes from verified ApiSession/SafetySnapshot state;
it is not a request DTO. `Command::decode` rejects duplicate keys and unknown
members before evaluation. Inputs cannot contain actor, tenant or role authority.

The current binding is deliberately small: one model module, native role aliases,
owner-or-reader visibility, an action writer role set, and a separate event role
set. These rules grant full model snapshots to authorized readers. Field masks,
external policy engines and implicit application tenants are not implemented.
A required application tenant must be part of the trusted host scope and policy;
this adapter does not invent it from user-supplied inputs.

The fixed native transaction rechecks guard incarnation/security sequence, active
user/role/epoch, session-family generation/expiry, active model and expected record
revision. It contends on the existing environment guard and session family and
atomically writes:

- `ti_record`: managed candidate and one revision advance;
- `ti_audit`: scoped actor/action/artifact/target and previous/new revisions/states;
- existing `operation`: original receipt, fingerprint and retained key tombstone;
- existing `outbox`: stable scoped event envelopes and durable delivery state.

Appendix C defines `input_digest`. Creation additionally reserves its logical
allocation ID in the operation; changing that reservation under the original key
conflicts. Operation lookup binds a framed scope/incarnation/identity/key tuple.
Same-key reconciliation happens before active-artifact selection, so an old
operation keeps its original artifact and outcome after activation. Current
write/read authority is checked before releasing its original snapshot. Expiry
removes a receipt but retains its key/digest tombstone.

A native request has a three-second observation deadline. Timeout, connection
loss, cancellation or an unclassified database failure never proves rollback.
Callers reconcile the original key; they must not create a fresh-key retry.
Business evaluation failures happen before transaction admission. This host does
not offer an asynchronous accepted/pending submission queue. Explicit transaction
abort errors are distinguished only when the complete abort response is available.

Direct user CRUD, SQL, transaction/bulk inserts and deletes cannot write Ti's
managed tables. General Mensoria collection routes use their existing `p1_`
namespace. The adapter does not widen their owner-write permissions. Root repair
and import tools remain privileged maintenance paths and need their own policy.

## Activation and delivery

`install` and `activate_closed` require a privileged maintenance caller with
admission closed and its existing maintenance lease held. They never run during
ordinary startup/action handling. Installation preserves existing table kinds,
schemas and permission predicates, adding scoped service branches. Activation
validates existing records, fences the active head, preserves published artifacts
and refuses incompatible records. A failed activation leaves admission closed;
SafetyManager owns reopening. This runner supports at most 1000 existing records,
read one per page. Larger migrations require a separately qualified runner.

The event adapter returns public tagged payloads only to a current event role.
Stable IDs survive redelivery. `begin_delivery` durably records an attempt;
`finish_delivery` records Delivered, explicitly RetrySafe, or Unknown. Only an
adapter with an idempotent-effect contract may select RetrySafe. An unfinished or
unknown attempt requires reconciliation and cannot be blindly redelivered.
No email/HTTP side effect or automatic platform worker is installed here.

Other binding limits: public command envelopes 132 KiB (inputs retain their full
128 KiB budget), keys 128 bytes, logical IDs 256 bytes, 64 inputs, 256 native
role mappings/aliases and actor roles, 128-byte role names, three seconds per
native query. Model scalar/aggregate limits also apply. Runtime connections are
provided by the existing platform; this crate does not create another scheduler.

## Existing DataFacade integration

The source adapter is versioned in `integrations/mensoria/ti_language.rs`.

```sh
python3 scripts/install_mensoria_adapter.py /path/to/surrealdb-rust-ai-baas
# In that Mensoria workspace:
cargo check -p baas-adapters --features ti-language
```

`DataFacade::execute_ti_action(project, environment, access_token, artifact, bytes)`
resolves the existing opaque access session, verifies native identity/family,
acquires ControlGate admission, binds the host to its SafetySnapshot, executes,
and rechecks current identity before publication. A publication failure after
commit is an unknown outcome. The artifact parameter is selected by the server's
registry. No HTTP route is registered by this optional feature. The installer
changes only the adapter's Cargo/module files; it does not run database DDL.

The current local source adapter compiles against the actual Mensoria workspace.
Its existing management-plane/session/HTTP path has not been exercised end to end
by this standalone fixture. Deployment must qualify that path and the full target
migration stack under the platform's existing release process.

## Evidence boundaries

Native tests include real table-write failures, three transaction-abort points,
concurrent approval/withdrawal, receipt retention, role/epoch/family/security
revocation, incompatible activation and old-artifact reconciliation. A loopback
TCP proxy drops an actual native commit response or holds it while the client
request is cancelled; persisted original receipts are then reconciled. The
separate `fault=4` probe only simulates client observation loss and is not the
network-fault evidence.

Remaining standard work includes exact source spans for every semantic expression,
phase/operation/retry metadata on every public error, a portable exhaustive fixture
corpus beyond these 44 regression scenarios, durable admitted rejection receipts,
large-environment migration, field-level disclosure, complete transport/generic
API bypass acceptance, and distributed/restore/revocation qualification against the
full deployed Mensoria stack. An incarnation-fence test is not a physical restore
exercise. A local passing receipt does not certify production, HA, backups or
external effects.
