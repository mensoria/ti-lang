# Ti Language Standard

<p class="ti-lead">A declarative business language for Mensoria.</p>

Ti describes business objects and their legal changes through entities, invariants,
policies, actions, and events. Mensoria supplies identity, persistence, and controlled
transactions to enforce the declared business rules.

<p class="ti-version">Draft 0.1 <span>·</span> ti/0.1 <span>·</span> approval-core/0.1</p>

## Start here

<dl class="ti-reading-path">
<dt><a href="overview.md">Language overview</a></dt>
<dd>Understand the core concepts, notation, and initial scope.</dd>
<dt><a href="approval.md">Approval walkthrough</a></dt>
<dd>Follow a leave request through creation, submission, approval, and withdrawal.</dd>
<dt><a href="01-purpose.md">Read the standard</a></dt>
<dd>Look up the precise syntax, types, authorization, and atomic action semantics.</dd>
</dl>

## Declare a business change

This excerpt from the approval example declares a source state, a destination,
an authorization condition, and the business event to record.

```text
action Approve on LeaveRequest {
    from Pending to Approved;
    allow Reviewer(self);
    emit LeaveApproved {
        requestId = self.id;
        applicant = self.applicant;
        days = self.days;
    }
}
```

Assignments read the old record; events read the candidate. The host commits
business state, operation result, audit, and outbox atomically. See
[Creation and atomic actions](07-actions.md) for the complete rules.

## Reference

| Topic | Chapters |
| --- | --- |
| Syntax and expressions | [Types](04-types.md) · [Expressions](05-expressions.md) · [Full EBNF](a-grammar.md) |
| Execution and failure | [Atomic actions](07-actions.md) · [Events and audit](08-events.md) · [Idempotency and outcomes](09-operations.md) |
| Platform integration | [Mensoria binding](integration.md) · [Model evolution](10-evolution.md) |
| Implementation evidence | [Conformance matrix](conformance.md) · [Delivery stages](roadmap.md) |

## Projects and compiler design

The [package proposal](project-and-packages.md) defines projects, modules, imports,
visibility and locked builds. [Compiler layering](compiler-architecture.md),
[delivery gates](implementation-plan.md) and [package acceptance](package-conformance.md)
distinguish implemented workspace organization from proposed next-edition syntax.

## Document status

This is a proposed standard with a Rust compiler/evaluator and an optional native
Mensoria transaction host. Project loading/imports and offline evaluation are available; next-edition
activation remains a proposal. The first profile
covers single-object, single-stage approval. Publication does not establish runtime
conformance.

English is the normative source. The Chinese translation retains the same clause
identifiers and grammar; both editions describe the same model.
