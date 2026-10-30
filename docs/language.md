# Ti language overview

Ti defines business objects and their permitted changes. It does not describe a
sequence of general-purpose instructions. A trusted host executes a validated
model with explicit identity, concurrency, and failure rules.

**Authority:** [Ti Language Standard — Draft 0.1](ti-language-standard.md).
This page is informative and does not define a second language contract.

## Core concepts

| Construct | Meaning |
| --- | --- |
| `entity` | Persisted business fields and one state machine |
| `invariant` | Actor-independent condition checked on every write |
| `policy` | Pure authorization predicate over one entity and trusted actor |
| `create` | Authorized creation with required-field initialization |
| `action` | Explicitly selected, atomic transition of one record |
| `event` | Typed business fact recorded alongside the state change |

## Example

```text
action Reject(reason: String) on LeaveRequest {
    from Pending to Rejected;
    allow Reviewer(self);
    require nonempty(input.reason) else "rejection_reason_required";
    set rejectionReason = trim(input.reason);
}
```

The complete [leave approval model](../examples/approval.ti) includes creation,
draft editing, submission, approval, rejection, and withdrawal. It can be checked and evaluated with the Rust reference core; it has no database host.

All assignment expressions see the old record; assignments apply simultaneously.
Invariants and emitted payloads see the candidate record. Business state, audit,
operation result, and outbox must commit atomically through the host. Unknown
outcomes require original-operation reconciliation, not blind action replay.

## Initial boundary

The `approval-core/0.1` profile supports Bool, String, Int64, nominal IDs, nullable
scalar fields, pure bounded expressions, one state machine, and one-record actions.
It excludes general functions, arithmetic, money/date codecs, cross-record reads,
loops, external I/O, timers, and long-running workflow orchestration.

See the standard for EBNF, exact typing, evaluation contexts, limits, diagnostics,
model evolution, and host requirements; see the [conformance matrix](conformance.md)
for the evidence required before claiming implementation support.
