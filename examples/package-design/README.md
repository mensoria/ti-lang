# Package design fixture

Status: runnable project compiler example for `ti/0.2` / `approval-package/0.2`.
The legacy single-file CLI still rejects its headers/imports. Project commands
load/compile it explicitly; no database activation or host artifact is produced.

`leave/` is the root model package; `approval-events/` is a sibling library.
Both packages and all seven source modules are checked, including unused imports.
The checked-in `leave/Ti.lock` is generated from their actual source bytes.
From the repository root:

```sh
cargo run --locked -- check --manifest examples/package-design/leave/Ti.toml --workspace-root examples/package-design
cargo run --locked -- compile --manifest examples/package-design/leave/Ti.toml --workspace-root examples/package-design
cargo run --locked -- eval --manifest examples/package-design/leave/Ti.toml --workspace-root examples/package-design examples/create-request.json
```

After intentionally editing sources, update the lock explicitly:

```sh
cargo run --locked -- lock --manifest examples/package-design/leave/Ti.toml --workspace-root examples/package-design
```

Normal check/compile/eval never repair Ti.lock. Without allowing the common
workspace parent, the sibling dependency is outside the default package root.
The unused DecisionRecorded import neither emits an event nor registers host work.

Expected linking:

- Local modules: `leave`, `leave.model`, `leave.policies`, `leave.actions`, `leave.events`.
- Dependency modules: `approval_events`, `approval_events.events`.
- Entity identity: `(acme.leave, 0.1.0, leave.model, entity, LeaveRequest)`.
- `LeaveRequestId` refers to that same defining entity despite imports.
- Root exposes one entity and its complete Create/EditDraft/Submit/Approve/Reject/Withdraw group.
- `CanReview` is an alias for the original `Reviewer` policy, not a second policy.
- Importing DecisionRecorded performs no event emission and creates no collection.
- The dependency has no model revision and cannot activate itself.

Proposal: [English](../../docs/project-and-packages.md),
[Chinese](../../docs/project-and-packages.zh-CN.md).
The runnable 0.1 approval example remains [approval.ti](../approval.ti).
