# Delivery stages and acceptance

Status: proposed sequence. No implementation milestone is complete.
The [standard draft](ti-language-standard.md) specifies the baseline;
[conformance cases](conformance.md) define required implementation evidence.

## Stage 1: Freeze the semantic contract

Review and freeze the draft grammar, expression and state-symbol typing,
diagnostics, canonical input encoding, identity-role binding, resource limits,
model artifact format, and activation compatibility. Produce approved source-to-IR examples and invalid-case
diagnostics before implementing dependent execution behavior.

## Stage 2: Validate models

Implement a parser and semantic validator producing immutable, versioned IR.
Validate the approval example and reject missing allows, bad event payloads,
unknown states, duplicate assignments, forbidden calls, and malformed/deep inputs.
No storage or deployment permissions follow from successful compilation.

## Stage 3: Execute pure action evaluation

Given trusted context, an old record, and typed inputs, return a candidate record
and event payloads or a structured rejection. Verify simultaneous assignment,
old-record preconditions, post-state invariants, null handling, and deterministic
results. This stage proves no persistence or concurrency property.

## Stage 4: Integrate one real approval model

Use one environment and one managed collection on Mensoria's pinned backend.
Implement controlled atomic action execution and close generic-write bypasses.
Activate models through the existing maintenance boundary. Expose a minimal SDK
journey before adding a visual model editor.

Required behavioral evidence:

| Scenario | Required outcome |
| --- | --- |
| Valid create/edit/submit/approve | Expected revisions, states, audit and event |
| Applicant approves own request | Denied; no business mutation |
| Actor lacks platform data access | Business allow cannot override denial |
| Wrong environment / application tenant | No unauthorized access or existence leak |
| Invalid input or post-state invariant | Atomic rejection |
| Concurrent approve and withdraw | One successful transition from the source revision |
| Same-key identical retry | Original authorized outcome; no duplicate event |
| Same-key changed input/model token | Idempotency conflict |
| Commit response is lost | Unknown outcome, original-operation reconciliation |
| Outbox/audit persistence fails | No partial business commit |
| Event consumer fails | Business remains committed; stable event can be retried |
| Generic CRUD/SQL/batch write | Cannot bypass managed-collection restriction |
| Stale model / incompatible activation | Reject or block explicit activation |
| Revoked actor requests old result | Current result-release policy enforced |

## Stage 5: Authoring experience

Add Studio validation, a generated state graph, model diff and activation review,
and action/operation inspection. Display confirmed business state separately from
pending event delivery. Never show request timeout as proof of failed approval.

## Deferred

Multi-object actions, delegation, multi-stage/quorum approvals, timers, external
payments, arbitrary host extensions, AOT, hot model replacement, and monetary/date
codecs. Each requires a separate semantic and failure contract.

Report documentation review, local tests, backend fault checks, CI, and business
acceptance separately. Synthetic and pure-evaluation tests do not establish
transaction, recovery, revocation, or production readiness.
