# Native qualification baseline

These five migrations are exact snapshots from the user's Mensoria prototype,
inspected on 2026-10-30. The source checkout had no committed baseline. Per-file
SHA-256 values in `manifest.json`, rather than an invented Git revision, identify
the tested source. The harness checks them before installation.

The fixture adds users, an epoch field, session families, Ti tables and active
models in an isolated private database. It uses actual record authentication,
data_issuer sessions, native table permissions and transactions on SurrealDB
3.3.0. No production credentials or shared databases are used.

This is the qualified transaction/authentication prefix, not the prototype's
entire later migration stack, transport router, or deployment. Set
`MENSORIA_SOURCE_ROOT` to recheck an external checkout with these exact files;
source drift fails instead of silently changing the qualification baseline.
