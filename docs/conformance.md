# Ti 0.1 conformance matrix

Status: all 44 numbered scenarios have executable Rust regression checks.
Observed results and exact source/backend hashes are in the
[qualification receipt](qualification/latest.json). This matrix remains the required evidence specification; scenario coverage is not full implementation certification. The normative source
is [the standard](ti-language-standard.md). V = validator, E = pure evaluator,
H = host binding. IDs are stable acceptance references, not passing test results.

| ID | Layer | Case | Required result | Clauses |
| --- | --- | --- | --- | --- |
| TC-001 | V | Validate approval.ti | Accept and resolve every declaration | TI-010–TI-032 |
| TC-002 | V | Invalid UTF-8, BOM, lone CR | Reject with source/lexical diagnostic | TI-005 |
| TC-003 | V | Keywords/case, identifier length, unexpected trailing token | Exact lexical rules; no recovery into execution | TI-006, TI-009 |
| TC-004 | V/E | Escaped emoji pair, lone surrogate, raw control, composed/decomposed strings | Valid scalars only; no normalization | TI-008, TI-017 |
| TC-005 | V | Min/max Int64; overflow; -0; 01; +1 | Accept only canonical in-range integers | TI-007 |
| TC-006 | V | Duplicate names; EId collision; missing creation | Reject | TI-010–TI-013 |
| TC-007 | V | String assigned to UserId or wrong entity ID | Reject nominal type mismatch | TI-014–TI-016 |
| TC-008 | V/E | T to T?; null to T?; null to T; T?? | First two valid, last two invalid | TI-014, TI-016 |
| TC-009 | V | Nullable equality to scalar; nullable predicate; ordered strings | Reject; no implicit truthiness | TI-018–TI-019 |
| TC-010 | E | Scalar length of emoji; trim ASCII vs nonbreaking space; nonempty(null) | Scalar count; only specified whitespace trimmed; false | TI-020 |
| TC-011 | V | Dynamic role name, ambient actor in invariant, self in creation/event actor | Reject forbidden contexts | TI-020–TI-024 |
| TC-012 | V/E | Precedence, parentheses, short circuit, chained comparisons | Follow grammar; reject chains; validate dead branch | TI-022 |
| TC-013 | V | Direct/mutual policy recursion; storage navigation | Reject | TI-024 |
| TC-014 | V | Multiple initial states; final-state outgoing action; missing state | Reject | TI-026–TI-027 |
| TC-015 | E | Multiple matching outgoing actions | Execute only explicitly requested action | TI-027 |
| TC-016 | V/E | Missing required create field; omitted optional field | Reject first; initialize second to null | TI-030 |
| TC-017 | V | Missing allow; repeated set; set state/id; duplicate require label | Reject | TI-031–TI-032 |
| TC-018 | E | Swap fields; same-state draft editing | Both RHS use old record; valid candidate | TI-032 |
| TC-019 | E | Old invariant fails; new invariant fails; require fails | Distinct internal error categories; no candidate commit | TI-028, TI-033 |
| TC-020 | E | Event reads changed field; missing/duplicate/extra event field | New value; malformed payloads rejected | TI-032, TI-037 |
| TC-021 | E | Repeated policy calls exhaust shared evaluation budget | Fail without skipping later checks | TI-025, TI-053 |
| TC-022 | H | Applicant attempts self-approval | Deny; no business mutation/event | TI-021, TI-033 |
| TC-023 | H | Business allow passes, platform denies | Deny without privilege escalation | TI-033–TI-034 |
| TC-024 | H | Wrong env/domain/incarnation/tenant; hidden record | Deny; no existence disclosure | TI-021, TI-033, TI-052 |
| TC-025 | H | Concurrent approve/withdraw from one revision | At most one commit; no fresh-key retry | TI-035 |
| TC-026 | H | Same-state unchanged action | Exactly one revision advance | TI-035 |
| TC-027 | H | CRUD, SQL, batch, import bypass | Managed collection writes denied | TI-036 |
| TC-028 | H | Outbox/audit/operation write fails | Whole business commit rolls back | TI-033, TI-038–TI-041 |
| TC-029 | H | Same original key/fingerprint after successful response or response loss | Same original outcome/IDs; no duplicate event | TI-038, TI-043–TI-046 |
| TC-030 | H | Changed inputs, model, revision, or target under same key | Idempotency conflict | TI-043–TI-045 |
| TC-031 | H | Commit response lost; cancellation; receipt unavailable | Unknown/pending as appropriate; no false rollback | TI-045–TI-047 |
| TC-032 | H | Receipt expires, then same key is sent | Reject/reconcile; never a new action | TI-047 |
| TC-033 | H | Actor revoked before result retrieval | Enforce current result access | TI-034, TI-047 |
| TC-034 | H | Event redelivery or consumer outage | Stable event ID; durable retry policy; business remains committed | TI-038–TI-040 |
| TC-035 | H | Event includes protected data; external side effect outcome unknown | Enforce disclosure/adapter contract; no blind replay | TI-040 |
| TC-036 | V/H | Mutation of published artifact; client selects old model | Reject | TI-048–TI-049 |
| TC-037 | H | Incompatible activation with existing records/in-flight operations | Drain/fence/reconcile and explicit migration; no unsafe switch | TI-050–TI-051 |
| TC-038 | H | Restore into new incarnation with old artifacts/keys | Requalify binding; stale authority rejected | TI-043, TI-051 |
| TC-039 | V/E/H | Limits at boundary and one over | Exact acceptance/rejection without unbounded work | TI-025, TI-053 |
| TC-040 | V | Native query, arbitrary call, loop, decimal, import | Reject unsupported feature | TI-004, TI-054 |
| TC-041 | H | Duplicate JSON keys; precision-losing integer; mistagged ID | Reject before business evaluation | TI-042 |
| TC-042 | H | Canonical tuple framing ambiguity; reordered input object | Distinct semantic tuples differ; key order has no effect | TI-044, Appendix C |
| TC-043 | H | State/revision error on concealed target | Sanitized public denial | TI-052 |
| TC-044 | H | Reconcile old operation after new model activation | Original artifact retained; no execution under new rules | TI-045, TI-049–TI-051 |

## Evidence packaging

Validator fixtures must include source, edition/profile/limits, expected diagnostic
code and span, or validated semantic artifact expectations. Evaluator fixtures
must include trusted context, old record, inputs, selected action, and expected
candidate/events or rejection. No fixture may treat untrusted request actor fields
as an authenticated actor.

Host evidence must identify exact source/host/backend versions, configuration,
Schema/model artifacts, identities, fault timing, operation keys/IDs, and observed
persisted outcomes. Retain failed and unknown runs. A deterministic simulator is
useful but does not replace real transaction and response-loss qualification.

Documentation review, validator tests, evaluator tests, host fault tests, CI,
and business acceptance are separate evidence categories. The local 44-scenario run and native host fault checks are recorded in
[implementation status](implementation.md). Full deployed-platform and exhaustive
standard qualification remain separate.
Prose examples are not test results.
