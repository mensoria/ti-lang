# Ti Lang

**A declarative business language for Mensoria.**

[简体中文](README.zh-CN.md) · [Documentation website](https://mensoria.github.io/ti-lang/)

Ti describes business entities, invariants, authorization conditions, state
transitions, and events. Mensoria supplies identity, persistence, controlled
transactions, operation reconciliation, and event delivery.

The name **Ti** evokes titanium, whose chemical symbol is Ti. It is a brand
metaphor, not a technical guarantee.

## Status

Design draft, 2026-10-29. This repository contains a proposed language model and
illustrative source. There is no parser, compiler, executor, or runnable example
yet. Syntax, file extension, and integration contracts are provisional.

## First use case

Single-object, single-stage approval: create a request, edit its draft, submit,
approve, reject, or withdraw. Actions are the only public write entry points for
Ti-managed entities.

The initial goal is to make permitted business changes explicit and enforceable
across every public write path. It is not to replace Rust, SQL, or the database.

## Design

- [Ti Language Standard — Draft 0.1](docs/ti-language-standard.md): normative syntax and semantics
- [Conformance matrix](docs/conformance.md): required evidence, not executed tests
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

Implementation licensing has not been selected. No upstream Rip code has been
copied into this repository.

## Bilingual book and publication

The online book provides 21 chapters per edition, complete English and Simplified
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
