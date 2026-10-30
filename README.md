# Ti Lang

**A declarative business language for Mensoria.**

[简体中文](README.zh-CN.md) · [Documentation website](https://mensoria.github.io/ti-lang/)

Ti describes business entities, invariants, authorization conditions, state
transitions, and events. Mensoria supplies identity, persistence, controlled
transactions, operation reconciliation, and event delivery.

The name **Ti** evokes titanium, whose chemical symbol is Ti. It is a brand
metaphor, not a technical guarantee.

## Status

Draft 0.1, 2026-10-30. Rust compiler, evaluator, strict wire codec, frozen
artifacts and an optional Mensoria transaction host are implemented. All 44
numbered regression scenarios are executable against a pinned native backend.
See [implementation and qualification boundaries](docs/implementation.md); this
is not a full standard or production conformance claim.

## First use case

Single-object, single-stage approval: create a request, edit its draft, submit,
approve, reject, or withdraw. Actions are the only public write entry points for
Ti-managed entities.

The initial goal is to make permitted business changes explicit and enforceable
across every public write path. It is not to replace Rust, SQL, or the database.

## Design

- [Ti Language Standard — Draft 0.1](docs/ti-language-standard.md): normative syntax and semantics
- [Conformance matrix](docs/conformance.md): required evidence with numbered baseline regression checks
- [Core model and minimum language](docs/language.md)
- [Mensoria integration and ownership](docs/mensoria-integration.md)
- [Delivery stages and acceptance](docs/roadmap.md)
- [Illustrative approval model](examples/approval.ti)

The standard owns draft semantics; the overview and example are informative. Changes
to the example must remain consistent with those semantics. The integration
proposal does not supersede Mensoria's identity, Schema, or operation contracts.

## Scope

Version one targets bounded expressions, explicit action inputs, one business
record per action, optimistic concurrency, durable events, and versioned models.

General-purpose functions, arbitrary SQL, network or filesystem access, loops,
user-supplied executable code, multi-object actions, timers, and long-running
workflow orchestration are outside the first version.

Implementation licensing has not been selected.

## Bilingual book and publication

The online book provides 25 chapters per edition, complete English and Simplified
Chinese manuscripts, chapter-aligned language switching, and search.
The English standard is normative; translated clauses retain the same identifiers.

```sh
python3 scripts/bootstrap_mdbook.py
python3 scripts/build_book.py
python3 -m http.server 8000 --bind 127.0.0.1 --directory site
```

Read the [authoring and publication guide](docs/publishing.md) for source ownership,
verified tool installation, checks, and GitHub Pages setup. Website publication
is separate from language implementation conformance.

## Run the reference core

```sh
cargo run --locked -- check examples/approval.ti
cargo run --locked -- compile examples/approval.ti
cargo run --locked -- eval examples/approval.ti examples/create-request.json
```

See [implementation and limits](docs/implementation.md). Evaluation returns a
candidate and events only; it does not authenticate or commit to Mensoria DB.

## Project and compiler design

Read the [package/module proposal](docs/project-and-packages.md),
[compiler architecture](docs/compiler-architecture.md),
[delivery plan](docs/implementation-plan.md) and
[package acceptance](docs/package-conformance.md). The implementation has six
workspace packages. Project loading, explicit locking, `use`/`pub use` and offline
evaluation are implemented; next-edition deployment remains separate.
