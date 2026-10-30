# Ti Language Standard — Draft 0.1

| Property | Value |
| --- | --- |
| Language | Ti |
| Edition | `ti/0.1` |
| Status | Normative draft; initial reference core, no host conformance |
| Date | 2026-10-29 |
| Initial profile | `approval-core/0.1` |
| Source extension | `.ti` (provisional) |

The next-edition [project/package proposal](project-and-packages.md) and
[compiler architecture](compiler-architecture.md) are separate design documents.
They do not extend the executable grammar of this profile.

## 1. Purpose, authority, and conformance

**TI-001.** Ti is a declarative business language defining persisted entities,
invariants, scoped authorization predicates, atomic actions, and business events.
A model states which changes are legal. A host provides identity, persistence,
transactions, and delivery. Ti is not a general-purpose programming language.

**TI-002.** MUST, MUST NOT, SHOULD, and MAY express requirements in this draft.
This document owns language syntax and observable semantics. The integration
proposal owns the Mensoria binding. Examples are informative. Conflicts MUST be
resolved in the authoritative documents before implementation enables a feature.
No existing Mensoria capability or historical test result establishes Ti conformance.

**TI-003.** Conformance is separately claimed for a **validator**, a **pure
evaluator**, and a **host binding**. Validators implement syntax/static rules;
evaluators implement deterministic candidate-state evaluation; host bindings
add authorization, persistence, concurrency, operations, and event delivery.
A parser-only implementation MUST NOT claim execution conformance. Claims MUST
name edition, profile, implementation fingerprint, limits, and conformance evidence.

**TI-004.** `approval-core/0.1` includes every construct in this draft, with one
record per action and one state machine per entity. Unknown syntax and unsupported
capabilities MUST fail closed. There are no implicit extensions, native-language
escape hatches, import resolution, or network dependency fetching.

## 2. Source and lexical rules

**TI-005.** Source MUST be valid UTF-8 without a BOM. LF and CRLF are accepted;
a lone CR is rejected. Whitespace is ASCII space, tab, LF, or CRLF. `//` starts a
comment through the line end. Block comments are not supported. No Unicode
normalization is applied to source or string values.

**TI-006.** Identifiers match `[A-Za-z_][A-Za-z0-9_]*`, are case-sensitive, and
are limited to 63 ASCII bytes. The words below are reserved. Keywords are lowercase;
`hasRole` is a case-sensitive built-in member name, not a keyword.

```text
module version entity field state initial final invariant policy create action
on from to allow require else set emit event true false null and or not
actor input self
```

**TI-007.** Integer literals are `0` or an optional minus followed by a nonzero
digit and zero or more digits. `-0`, leading zeros, a plus sign, separators, floats,
and exponent notation are rejected. Values MUST fit signed 64-bit range, including
`-9223372036854775808`. A module version is a positive integer at most 2147483647.

**TI-008.** String literals use double quotes and JSON-style escapes: `\"`,
`\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, and `\uXXXX`. Raw control characters
U+0000–U+001F are forbidden. Escaped surrogate pairs decode to one scalar; lone
surrogates are rejected. Strings have no interpolation. String size limits apply
to decoded UTF-8 bytes, including escaped content.

**TI-009.** Source spans use one-based Unicode scalar line/column positions,
with one-past-end ends. A tab counts as one column; CRLF counts as one newline.
Tokenization uses longest match (`!=`, `==`, `<=`, `>=` before single characters).
Unknown tokens and trailing source are errors.

## 3. Modules and names

**TI-010.** A source unit starts with exactly one `module NAME version N;`,
followed by declarations. The host supplies the language edition out of band;
module version and language edition are distinct. A model is one source unit in
this profile. Declaration order does not affect name resolution.

**TI-011.** Entity, event, and policy names share one module namespace. Primitive
type names and built-in function names cannot be redeclared. Each entity `E`
introduces the nominal type `EId`; a collision with any declared name is an error.
Each entity has exactly one creation declaration. Action names are unique per
entity and cannot be `Create`; hosts use `Create` to identify the creation operation.

**TI-012.** Field names are unique within an entity or event. An entity state
field shares the business-field namespace. States are unique within their owning
state machine. Input names are unique within an action/creation declaration.
Policy parameter names cannot shadow reserved or built-in names.

**TI-013.** Entity fields and state fields MUST NOT use platform-owned names
`id`, `revision`, `created_at`, `updated_at`, `scope`, or `model_version`.
These cannot be assigned through Ti. `self.id` is the only platform attribute
readable in core expressions. Event fields may be named `id`, but are payload
fields and never override an event envelope.

## 4. Types and value semantics

**TI-014.** Core field/input/event types are `Bool`, `String`, `Int64`, `UserId`,
and nominal entity IDs such as `LeaveRequestId`. `T?` adds null to a nonnullable
core type. Nested nullable suffixes and nullable entities are rejected. Entity
types are allowed only as policy parameters, not persisted fields or action inputs.

**TI-015.** An entity's state has a nominal state type private to that entity.
State values cannot be accepted from inputs or assigned with `set`. A state name MUST NOT collide with an entity business/state field, a primitive
type, or a built-in function name. A bare state
name, such as `Pending`, is valid only in `from`/`to` or where a comparison with
the current entity's state field supplies its type. Two state constants without
such a typed operand are ambiguous and rejected. State values are not strings.

**TI-016.** Assignments accept the exact type, nonnullable `T` into `T?`, or
null into `T?`. No other implicit conversions exist. IDs are nominal opaque scalar
sequences, not interchangeable strings. Comparing an ID does not authorize access
to its referent. Core expressions cannot dereference related records.

**TI-017.** `Int64` has exact signed 64-bit values. `String` is a sequence of
Unicode scalars; equality is exact, case-sensitive scalar-sequence equality.
No locale collation or normalization applies. Boolean values are exactly true
and false; truthiness and SQL three-valued logic are forbidden.

**TI-018.** `==` and `!=` accept identical nonnullable scalar/state types, or a
nullable operand compared to literal null. Comparing two nullable operands or a
nullable operand with a nonnull value is rejected. Ordered comparisons `<`, `<=`,
`>`, `>=` accept only nonnullable `Int64`. There is no arithmetic in this profile.

**TI-019.** All `allow`, `require`, `invariant`, and policy expressions MUST
have type `Bool`. A nullable Boolean is not a predicate. The profile has no flow
narrowing after null checks; use an explicitly nullable-aware built-in where needed.

### Built-in functions

**TI-020.** Only these functions and validated policy calls are supported:

| Function | Signature and behavior |
| --- | --- |
| `length(x)` | `String -> Int64`; counts Unicode scalars |
| `trim(x)` | `String -> String`; removes leading/trailing U+0020, U+0009, U+000A, U+000D only |
| `nonempty(x)` | `String? -> Bool`; false for null or empty after `trim`, true otherwise; accepts `String` by nullable widening |
| `actor.hasRole(x)` | String literal -> Bool; exact membership in trusted scoped role set |

**TI-021.** Role literals are nonempty ASCII strings of at most 128 bytes.
`actor.id` has type `UserId`. Actor ID and roles MUST come from the same trusted
identity snapshot, with identity-domain and scope bindings. They are never action
inputs. The host MUST separately define role assignment, revocation, and any
business-tenant binding. Platform administrator membership grants no implicit
business role.

## 5. Expressions and evaluation contexts

**TI-022.** Precedence, highest to lowest: parentheses/member access/function
calls; prefix `not`; one comparison; `and`; `or`. Binary Boolean operators are
left-associative and short-circuit left to right. Chained comparisons are rejected.
For `not (a == b)` the parentheses are required. Every expression is statically
checked, including branches that would not execute.

**TI-023.** Expression contexts are closed:

| Context | Available names |
| --- | --- |
| Invariant | Bare current entity field/state names, typed state constants, pure built-ins |
| Policy | Its entity parameter and scalar fields/state/ID, actor, other validated policies, pure built-ins |
| Creation allow/require/set | Input, actor, pure built-ins; no `self` or entity-policy argument |
| Action allow/require/set | Old `self`, input, actor, policies, pure built-ins |
| Event payload | Candidate `self`, input, pure built-ins; no actor or policies |

Invariants cannot depend on actor, input, policies, time, or external data. Their
meaning MUST remain stable when checked by a migration or maintenance validator.
A policy takes exactly one nonnullable entity parameter and returns Bool. Calls
must receive `self` or another entity parameter of the declared type. No entity
value can be constructed as an expression. Event payloads cannot contain whole
entity values or implicit record serialization.

**TI-024.** The policy call graph MUST be acyclic. Policies cannot query storage.
Functions are deterministic and side-effect free. Access paths are limited to
`input.FIELD`, `self.FIELD`, `PARAM.FIELD`, `actor.id`, and `self.id`/`PARAM.id`.
Other chains, arbitrary method calls, indexing, dynamic names, or reflection are
rejected. Bare field names are valid only in invariants.

**TI-025.** Exhausting a declared evaluation budget MUST reject the action
without a partial business commit. Evaluation limits are part of the execution
profile. A host MUST NOT silently alter expression results or skip predicates
when a budget is reached.

## 6. Entities and state machines

**TI-026.** Each entity declares zero or more fields, exactly one state machine,
and zero or more named invariants. There MUST be at least one state, exactly one
initial state, and no state marked both initial and final. Unmarked states may
have outgoing actions. There is no implicit transition or default action.

**TI-027.** Final states have no outgoing action, including same-state actions.
Same-state actions from nonfinal states are allowed. Multiple actions may have
the same source/destination; the caller explicitly selects an action. The executor
MUST NOT choose among actions based on declaration order or predicate matching.

**TI-028.** Every successful create or action MUST produce a record satisfying
all invariants, checked in source order. An already invalid old record MUST be
rejected as `TI_RECORD_INVALID` before ordinary business evaluation; ordinary
actions cannot silently repair invalid stored data. Privileged repair requires a
separate host maintenance contract and post-repair validation.

**TI-029.** The validator SHOULD diagnose unreachable states and nonfinal states
with no outgoing actions. It MUST NOT claim predicates are satisfiable, approval
policy is correct, or the state graph guarantees real-world process completion.

## 7. Creation and atomic actions

**TI-030.** Creation declares typed inputs, exactly one allow predicate, zero or
more requirements, then assignments. It has no source transition, `self`, or
emissions. Required nonnullable fields MUST each be initialized exactly once.
Omitted nullable fields initialize to null. The state initializes to the declared
initial state; the host generates a stable logical ID for the accepted operation.
Retries MUST reuse the original creation outcome and ID.

**TI-031.** An action declares optional typed inputs, one source and destination,
exactly one allow predicate, zero or more requirements, assignments, and emissions,
in that order. Requirements have a literal error label matching
`[a-z][a-z0-9_]{0,62}`; duplicate labels in one operation are rejected. Empty input
lists and omitted parentheses both mean no inputs.

**TI-032.** Assignments target declared business fields only, once per field.
Unassigned fields retain their old values. Every RHS is evaluated on the same old
record and input, then applied simultaneously. Consequently `set a = self.b; set
b = self.a;` swaps two compatible fields. Assignments do not introduce variables.
Events observe the new record, not assignment intermediates.

**TI-033.** The logical new-action execution order is:

1. Validate request structure, typed inputs, trusted identity, and target scope.
2. Resolve any original operation/key under current result-access rules.
3. For a new operation, resolve active model and check expected artifact identity.
4. Enforce platform action/data authorization and obtain a consistent old record.
5. Check the business allow predicate, expected record revision, then source state.
6. Check old-record invariants, then requirements in source order.
7. Evaluate/apply assignments and the transition to form a candidate record.
8. Check candidate invariants; evaluate/type-check events in source order.
9. Atomically commit record, version change, operation result, audit, and outbox.
10. Apply host result-release authorization before disclosing the response.

Creation replaces steps 4–8 with platform create authorization, business allow,
requirements, initialization, and candidate invariant checking. Steps that depend
on data MUST execute in one transaction or an equivalent protocol that binds all
observed dependencies to commit. A separate unguarded PATCH is not conformant. Any model access to protected
fields MUST be validated against a narrowly scoped host evaluation capability;
it MUST NOT silently acquire administrator access or disclose those fields to
the caller. If the required evaluation capability cannot be supplied, model
activation MUST fail.

**TI-034.** The actor snapshot remains fixed during pure evaluation. The host
MUST enforce its declared revocation/admission protocol at commit and result
release; a snapshot is not a permanent authorization grant. External role stores
require an explicitly qualified consistency protocol. Ti does not imply an atomic
transaction across an external identity provider and the environment database.

**TI-035.** Each existing-record action requires an expected revision, and each
successful action advances it once, including same-state or unchanged-value
actions. Revisions are opaque host tokens; clients cannot calculate the next one.
Two transitions from the same revision cannot both commit. Conflicts MUST NOT
trigger automatic fresh-key execution against the latest state.

**TI-036.** The host MUST prevent every public alternate write path from
bypassing these rules. In this profile, generic CRUD, SQL writes, bulk writes,
and imports are denied for Ti-managed collections. Creation and draft editing
also use actions. Schema changes, raw native maintenance, and privileged repair
require separately authorized, audited protocols. Compilation alone cannot enforce
this restriction; it is a host-binding conformance requirement.

## 8. Events, audit, and external effects

**TI-037.** Each emission references a declared event and supplies each field
exactly once with an assignable expression. There are no implicit optional event
fields: nullable fields must explicitly receive a value or null. Events may be
emitted multiple times; source ordinal distinguishes them.

**TI-038.** An event envelope includes a stable event ID, original operation ID,
trusted scope, exact model artifact, event type, emission ordinal, target logical
ID, and committed record revision. Payloads cannot override envelope metadata.
The same operation retry MUST NOT create new event IDs or duplicate outbox entries.

**TI-039.** Event persistence is atomic with business state. Delivery is at least
once, with stable IDs and consumer deduplication. The host declares retry/retention
and failed-delivery handling. No global delivery ordering or exactly-once external
effect is promised. Approval completion and notification delivery are separate
states. A failed consumer does not roll back the business record.

**TI-040.** Event payloads are protected data. Publication/consumption MUST apply
host authorization, disclosure, retention, and revocation rules. An `emit` never
grants permission to email, export, or invoke a model. Irreversible external effects
require a separate adapter contract; unknown external outcomes cannot be blindly
replayed merely because the transport uses at-least-once delivery.

**TI-041.** Audit records bind scope, actor identity domain/ID, model artifact,
action, operation, target ID, and previous/new revisions and states. A creation
has no previous state/revision. Audit MUST NOT automatically copy credentials or
all request/record contents. Audit persistence failure prevents business commit.
Denied attempts may be recorded in a separate security log; they are not a
successful business action audit.

## 9. Request values, idempotency, and observable outcomes

**TI-042.** A transport binding MUST represent typed values without precision
loss or ID conflation. The core logical value encoding is:

```json
{"type":"Int64","value":"3"}
```

| Type | Value member |
| --- | --- |
| Bool | JSON Boolean |
| String | JSON string |
| Int64 | Canonical decimal string, same number rules as integer literals |
| UserId | Opaque string, bound to identity domain by trusted context |
| EId | Opaque string with type tag `EId` |
| Null | `{"type":"null"}` with no value member |

Inputs are a name-to-tagged-value object. All declared inputs MUST be present,
including nullable inputs explicitly set to null. Missing, extra, duplicate keys,
unknown tags/members, and incompatible values are rejected. Empty opaque IDs are
invalid. Structured record identities and native database selectors are forbidden.

**TI-043.** Bindings MUST require original idempotency key, expected model artifact,
and (for existing records) logical ID and expected revision. Scope/actor cannot be
supplied through inputs. An idempotency key is 1–128 ASCII characters from
`[A-Za-z0-9._:-]`. Key lookup is scoped to trusted environment incarnation and
principal identity domain/ID. Reuse within that scope binds one operation.

**TI-044.** The request fingerprint binds edition/profile, module and artifact,
entity/action, target or creation marker, expected revision or creation marker,
trusted scope/incarnation/actor, and all typed inputs. This is a logical tuple;
bindings MUST version its exact canonical byte encoding before claiming host
conformance. Delimiter concatenation without unambiguous framing is forbidden.
Object key order is insignificant, string contents are not normalized, and IDs
retain nominal types. No hash of raw transport JSON substitutes for this encoding.

**TI-045.** Same key and fingerprint resolves the original operation; different
fingerprint returns a conflict after current authorization checks. Accepted
operations MUST be durably reconciled; infrastructure MUST NOT report a business
rejection after losing evidence of whether commit happened. Pre-admission failures
may be returned without an operation, with admission phase explicitly indicated.

**TI-046.** A binding distinguishes at least `committed`, `rejected`, `pending`,
and `unknown` observations, plus an explicit original-result-expired response.
An original operation reference is included once accepted when observable.
A confirmed rejection after admission binds the original key to that rejected
outcome; it is not re-evaluated against newer data on retry. Rejections MUST NOT
produce business records, business audits, or business events; durable operation
receipts and security logs may still be written.
Unknown means no confirmed outcome, not rollback or permission to replay.
Cancellation/timeout/disconnection cannot establish rejection or undo a commit.

**TI-047.** Receipt expiry MUST NOT make an old key executable again. The host
retains a key tombstone or retires its key scope so stale keys cannot be accepted
as fresh writes. Result retrieval always enforces current access. A committed
operation may exist even if its original caller can no longer see its result.
An original result MUST NOT be silently replaced by the latest record state;
current field-disclosure rules still apply to any retained result snapshot.

## 10. Model artifacts, activation, and evolution

**TI-048.** Validation produces a frozen artifact binding exact UTF-8 source,
source digest, edition/profile, compiler fingerprint, typed IR digest, model name
and version, dependency bindings, resource limits, and required capabilities.
Digests use SHA-256 with explicit domain/version framing. Hosts MUST pin the
manifest/IR serialization contract; source hash alone is not artifact identity.
Compilation MUST NOT execute business actions or change a Schema.

**TI-049.** Published model versions are immutable in their trusted project scope.
Changing comments changes the exact source artifact and requires a new version
once published. An environment activates only a trusted, validated artifact bound
to its Schema mapping, role contract, policy, and safety generation. Caller-supplied
older artifacts cannot select permissive rules.

**TI-050.** Core activation drains new business writes, reconciles or fences
in-flight operations under the host protocol, checks existing data and affected
bindings, and switches the active artifact with compare-and-swap before reopening.
Incompatible changes require explicit reviewed data/Schema migration. No automatic
rollback, online hot swap, cross-environment transaction, or history rewrite is
implied. Old operations retain original artifact references for reconciliation.

**TI-051.** Removal of fields/states/actions, stronger invariants, changed role
bindings, changed predicates, or changed event payloads all require impact review.
No change is declared backward compatible solely because it adds syntax. Events
already committed retain their original schema/artifact; consumers declare the
versions they accept. Restore/recovery MUST preserve or requalify model bindings
and apply Mensoria's current safety/incarnation rules before accepting writes.

## 11. Diagnostics and resource limits

**TI-052.** Diagnostics include stable code, phase, sanitized message, and source
span when attributable. Execution errors include original operation reference when
accepted/observable and a retry/reconcile hint. Requirement labels are optional
public details subject to host disclosure policy. Internal evaluation order does
not authorize revealing a hidden record, role, field, or state.

| Code | Meaning |
| --- | --- |
| TI_SYNTAX | Invalid lexical/grammatical input |
| TI_NAME | Unknown, reserved, duplicate, or ambiguous name |
| TI_TYPE | Invalid type, operator, call, or assignment |
| TI_MODEL | Invalid state/creation/action structure |
| TI_EFFECT | Forbidden effect or dependency |
| TI_LIMIT | Static or runtime budget exceeded |
| TI_INPUT | Invalid request/input encoding |
| TI_ACCESS_DENIED | Platform/business authorization denied or concealed target |
| TI_MODEL_STALE | Expected model does not match active binding |
| TI_REVISION_CONFLICT | Record revision no longer matches |
| TI_STATE | Action source state does not match |
| TI_RECORD_INVALID | Existing data violates the active model |
| TI_REQUIRE | Action requirement failed |
| TI_INVARIANT | Candidate violates an invariant |
| TI_IDEMPOTENCY_CONFLICT | Key is already bound to different inputs |
| TI_OUTCOME_UNKNOWN | No confirmed original commit outcome |
| TI_RESULT_EXPIRED | Original result no longer retained; not permission to replay |
| TI_DEPENDENCY_UNAVAILABLE | Host dependency unavailable; outcome determined separately |

Host disclosure rules MAY coarsen private state/revision failures into access denial.
A private diagnostic retained for operators MUST NOT leak through the public API.

**TI-053.** Implementations publish deterministic static/evaluation budgets and
reject oversized inputs before uncontrolled allocation or stack recursion. The
baseline profile limits are:

| Budget | Maximum |
| --- | --- |
| Source unit | 256 KiB |
| Identifier | 63 ASCII bytes |
| Decoded string value/literal | 16 KiB |
| One opaque ID | 256 UTF-8 bytes |
| Expression nesting | 64 |
| Declarations, excluding nested fields/states | 256 |
| Fields per entity/event; inputs per operation | 64 each |
| States per entity | 64 |
| Requirements/invariants per operation/entity | 64 each |
| Emissions per action | 16 |
| Expanded expression-node evaluations per operation | 100000 |
| Encoded request inputs / aggregate event payloads | 128 KiB each |

One visited expression AST node counts one evaluation step, including repeated
policy-body evaluation; short-circuited nodes are not visited. The evaluator shares
one budget across allow, old/new invariants, requirements, assignments, and events.
Hosts additionally bound records, role sets, transaction duration, output bytes,
and delivery resources. These host bounds MUST be published and enforced; timeout
alone is not evidence of rollback. Different limits require an explicitly named
profile rather than silent changes to this baseline.

## 12. Exclusions and extension discipline

**TI-054.** This edition excludes arithmetic, money/date types, collection
expressions, joins, cross-record policies, user-defined general functions, imports,
loops, arbitrary Rust/native code, SQL, filesystem/network calls, timers,
multi-record actions, quorum/delegation, sagas, and distributed transactions.
An extension MUST specify syntax, types, effects, scope, limits, versioning, failure
semantics, and conformance cases before acceptance. No host capability is enabled
merely because a database or Rust library can perform it.

## Appendix A. Normative grammar

The EBNF uses quoted terminals, `{ X }` for repetition, `[ X ]` for optional syntax,
and `|` for alternatives. IDENT, INT, POSINT, and STRING follow lexical rules above.
Structural and typing constraints in the numbered clauses additionally apply.
A semicolon following a closing declaration brace is not accepted.

```ebnf
source       = "module", IDENT, "version", POSINT, ";", { declaration } ;
declaration  = entity | policy | creation | action | event ;
entity       = "entity", IDENT, "{", { field }, state, { invariant }, "}" ;
field        = "field", IDENT, ":", type, ";" ;
type         = IDENT, [ "?" ] ;
state        = "state", IDENT, "{", state_item, { state_item }, "}" ;
state_item   = IDENT, [ "initial" | "final" ], ";" ;
invariant    = "invariant", IDENT, ":", expression, ";" ;
policy       = "policy", IDENT, "(", IDENT, ":", IDENT, ")", "=",
               expression, ";" ;
creation     = "create", IDENT, "(", [ parameters ], ")", "{", allow,
               { requirement }, { assignment }, "}" ;
action       = "action", IDENT, [ "(", [ parameters ], ")" ], "on", IDENT,
               "{", transition, allow, { requirement }, { assignment },
               { emission }, "}" ;
parameters   = parameter, { ",", parameter } ;
parameter    = IDENT, ":", type ;
transition   = "from", IDENT, "to", IDENT, ";" ;
allow        = "allow", expression, ";" ;
requirement  = "require", expression, "else", STRING, ";" ;
assignment   = "set", IDENT, "=", expression, ";" ;
emission     = "emit", IDENT, "{", { event_value }, "}" ;
event_value  = IDENT, "=", expression, ";" ;
event        = "event", IDENT, "{", { field }, "}" ;
expression   = disjunction ;
disjunction  = conjunction, { "or", conjunction } ;
conjunction  = comparison, { "and", comparison } ;
comparison   = unary, [ compare_op, unary ] ;
compare_op   = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
unary        = { "not" }, primary ;
primary      = INT | STRING | "true" | "false" | "null"
             | reference | call | "(", expression, ")" ;
reference    = name, [ ".", IDENT ] ;
name         = IDENT | "self" | "input" | "actor" ;
call         = IDENT, "(", [ arguments ], ")"
             | "actor", ".", "hasRole", "(", STRING, ")" ;
arguments    = expression, { ",", expression } ;
```

A bare `self` or policy parameter is valid only as a policy-call argument. Bare
`actor`/`input` are never values. Reference/call alternatives are selected by the
following token; unrestricted function syntax does not authorize arbitrary calls.
All target states and state comparisons resolve within the owning entity.

## Appendix B. Conformance evidence

The [conformance matrix](conformance.md) defines required positive and negative
cases. A complete machine-readable corpus and fixed serialization fixtures are
required before implementation conformance is claimed. This draft and its prose
review do not constitute executable conformance evidence.

## Appendix C. Normative logical fingerprint encoding

To remove ambiguity in TI-044, core fingerprint bytes use the following framing.
`U32(n)` is a four-byte unsigned big-endian integer. `F(tag, bytes)` is one ASCII
tag byte followed by `U32(byte_length)` and exactly those bytes. Text is UTF-8
without normalization. Length overflow is rejected.

A typed value is encoded as follows:

- Bool: `F('b', "true")` or `F('b', "false")` using ASCII contents.
- String: `F('s', decoded UTF-8 contents)`.
- Int64: `F('i', canonical ASCII decimal)`.
- UserId: `F('u', opaque UTF-8 contents)`.
- Entity ID: `F('e', F('t', entity type name) || F('v', opaque ID))`.
- Null: `F('n', empty bytes)`; declared nullable type comes from the artifact.

`||` denotes byte concatenation, not a language expression. Input entries are
sorted by ASCII name bytes and encoded as `F('p', F('k', name) || typed_value)`.
The inputs block is `F('x', concatenated entries)`. Duplicate input names are rejected
before sorting. JSON whitespace/escape spelling and object order do not affect it.

The request tuple is the concatenation, in this exact order, of:

1. `F('d', "ti.request/0.1")`;
2. `F('l', edition)` and `F('p', profile)`;
3. `F('s', host scope token)` and `F('g', host incarnation token)`;
4. `F('a', principal identity-domain token)` and `F('u', principal ID token)`;
5. `F('m', module name)` and `F('v', canonical module-version decimal)`;
6. `F('h', exact expected artifact identity token)`;
7. `F('e', entity name)` and `F('c', action name or "Create")`;
8. `F('r', logical record ID)` and `F('q', expected revision token)`, or
   `F('r', empty)` and `F('q', empty)` for creation;
9. the inputs block.

The fingerprint is SHA-256 of that tuple. Host scope tokens MUST unambiguously
bind organization, project, environment, and any required business tenant; their
versioned encoding, identity-domain tokens, revision tokens, and artifact identity
tokens are defined by the binding and cannot be inferred from user input. Ordinary
record IDs and revision tokens MUST NOT be empty. The idempotency key is the lookup
key, not part of its request fingerprint. This digest is an equality mechanism,
not an authorization credential or a public disclosure format.

As a basic framing fixture, `F('i', "3")` has hexadecimal bytes
`69 00 00 00 01 33`. A complete host binding MUST publish full tuple/digest fixtures
using its concrete token encodings before claiming host conformance.
