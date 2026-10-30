# Rust reference implementation

Status: initial validator and pure-evaluator implementation, 2026-10-30.
This is not an approval-core/0.1 conformance claim or a database host.

## Run

Rust 1.96.0 is used for local verification and CI. Dependencies are fixed by Cargo.lock.

```sh
cargo run --locked -- check examples/approval.ti
cargo run --locked -- compile examples/approval.ti
cargo run --locked -- eval examples/approval.ti examples/create-request.json
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

`check` parses and checks names, types, contexts, policies, states, actions and event
payloads. `compile` prints the validated model and exact source SHA-256, edition
and profile. This is inspectable IR, not the final host artifact manifest or ABI.
`eval` evaluates one action and prints a candidate record and ordered event
payloads. No database write, identity verification or event delivery takes place.
Errors exit nonzero. Library callers use `compile` then `Compiled::evaluate`.
The validated model cannot be constructed by deserializing unchecked JSON.

## Input and output

The example JSON is an offline evaluator harness. Its actor is an explicitly
supplied test snapshot, never an authenticated network identity. The evaluator
checks its shape but does not authenticate it. `old` supplies the old record for
an existing-record action. `new_id` is required only for creation. Returned records
can be supplied as `old` to the next call; callers cannot mutate the input record
through evaluation. State/revision CAS is not simulated here.

Primitive harness values are tagged. Internal entity IDs use
`{"type":"id","value":{"entity":"LeaveRequest","id":"leave-1"}}`.
This diagnostic representation is not the public TI-042 transport codec. A host
binding must implement the standard's nominal ID tags and strict duplicate-key
handling before exposing network requests.

## Implemented behavior

- Bounded UTF-8 lexical analysis, canonical Int64 literals, JSON string escapes.
- Entity, event, policy, creation and action parsing with bounded expression depth.
- Nominal types, nullable assignment, Boolean predicates and scoped references.
- Policy cycle detection without expanding the policy graph exponentially.
- Final-state restrictions, explicit authorization, initialization and event checks.
- Old-record requirements, simultaneous assignments, candidate invariants and events.
- Shared evaluation budget, with an additional implementation nesting limit of 256.
- A complete approval journey, self-approval rejection, invalid inputs/invariants,
  field swapping, candidate event values and adversarial policy/lexical checks.

The approval source now uses `field` in event declarations, matching the normative
EBNF. The previous example omitted that keyword; the grammar was not broadened.

## Remaining qualification work

The 44 conformance cases remain the acceptance authority; ten integration tests
cover selected behaviors and multiple negative cases, not all 44 cases. In
particular: complete per-expression diagnostic spans, static work limits for all
compound expressions, complete public request/event codec budgets, persistent frozen
artifact identity/dependency formats, warnings for unreachable states, full
machine-readable corpus, and exact host codec fixtures still require work.

The internal state literal implementation is not a general entity-state type
export. Additional edge cases must be qualified before declaring full validation
conformance. The source hash alone is not artifact identity or authorization.

Host work remains: trusted scoped identity, current data permission and revocation,
active-model resolution, revision CAS, atomic record/Operation/audit/outbox writes,
original-key reconciliation, expiry tombstones, and alternate-write enforcement.
No Mensoria business repository or backend data was modified in this implementation.
