# Mensoria integration and ownership

Status: optional native transaction host implemented and locally qualified.
The existing DataFacade source adapter compiles; full platform/HTTP acceptance
remains separate. See [implementation](implementation.md).
Language authority: [Ti Standard Draft 0.1](ti-language-standard.md).

## Placement

Ti is Mensoria's declarative business language. Model validation, versioning, and
action execution form a business semantics module in the shared Rust application
core. A separate service, database kernel, or scheduler is not required.

```text
Studio / SDK / application
          |
Ti validation + immutable model artifact + Action API
          |
Identity / Policy / Schema / controlled command execution
          |
Record + Operation + Outbox + audit
          |
SurrealDB environment database
```

| Owner | Responsibility |
| --- | --- |
| Ti compiler | Syntax, types, pure expressions, state/action semantics, artifact |
| Ti model service | Versions, dependencies, compatibility, explicit activation |
| Ti action executor | Business preconditions, candidate state, invariants, event payloads |
| Mensoria Identity/Policy | Trusted actor, scope, current platform permission |
| Mensoria Schema | Logical fields, codecs, migration and physical mappings |
| Mensoria command infrastructure | Safety admission, revision checks, atomic commit, operation/key reconciliation |
| Mensoria Runtime | Existing outbox delivery and task execution mechanisms |
| Mensoria Studio | Editing, validation feedback, state diagram, activation review, operation inspection |

## Existing foundation versus new work

The inspected Mensoria prototype documents controlled record commands, revision
checks, persistent operations, outbox writes, and unknown-outcome reconciliation.
Those mechanisms are inputs to this proposal. Their historical/local qualification
does not qualify Ti actions or a new deployment.

New work includes model artifacts and activation, expression validation/evaluation,
action authorization and state transitions, invariant enforcement, atomic action
commands, semantic business events, and preventing alternate-write bypasses.
Existing CRUD support is not evidence that these behaviors already work.

## Binding and authority

Reuse trusted organization/project/environment binding and record identities.
Never accept a physical database or native record token from Ti source or action
inputs. Platform scope is distinct from an application's optional business tenant.
The latter requires explicit schema and policy design; Ti does not invent one.

Business authorization narrows platform permission. It cannot grant access to
records or fields denied by platform policy. A scoped role contract must define
which identity domain and authority supplies business roles; platform membership
must never silently become application-user privilege.

Revalidate according to Mensoria's current session, admission, revocation, and
result-release rules. Do not weaken them to a forever-valid request snapshot.
Schema migration and model activation are maintenance/control operations; ordinary
Action API traffic must not receive DDL or root capabilities.

## Prevent bypasses

MVP Ti-managed collections allow public writes only through actions, including
creation and draft editing. Generic CRUD, batch APIs, SQL mutation, and import
surfaces must reject direct writes to them unless a separately reviewed protocol
enforces equivalent semantics. Reads retain current platform authorization.

A language-level `allow` is insufficient if another endpoint can directly update
`status`. Enforcement must exist at the common command/storage capability boundary,
not only in Studio. Privileged repair is separate, explicitly authorized, audited,
and followed by model validation.

## SQL relationship

Mensoria SQL is a data query/mutation front end; Ti describes legal business
changes. Both reuse the existing application core. Ti need not compile through
SQL. The inspected SQL specification is a proposed design, not an enabled endpoint.

SQL reads may expose authorized Ti business fields through published bindings.
SQL writes must obey the managed-collection restriction. Ti expression semantics
are separately versioned; do not silently inherit SQL NULL or SurrealQL truthiness.

## Action request proposal

An action request carries logical target ID, action name, typed inputs, expected
record revision, an expected model artifact token, and the original idempotency
key. These are conceptual fields, not a new committed HTTP contract.

The host resolves the active artifact and rejects stale model expectations for
new operations. Same-key reconciliation refers to the original artifact and
outcome under current result-access rules. Atomic persistence must include state,
operation outcome, audit, and outbox; external effects happen after commit.

## Design provenance

This proposal was informed by ArchStudio's Mensoria architecture, component,
data-service and SQL design documents, and the prototype README inspected on
2026-10-29. Those sources remain authoritative for platform contracts. This
repository owns the Ti draft; it does not independently redefine platform behavior.
