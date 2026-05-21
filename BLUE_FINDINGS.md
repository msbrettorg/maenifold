# BLUE FINDINGS — T-303.50.2

## Owning crate and registration path

- Crate: `crates/maenifold-core`
- Graph resource facade: `crates/maenifold-core/src/mcp/resources/graph.rs`
- Corpus resource registration: `crates/maenifold-core/src/mcp/resources/graph_corpus.rs`
- Top-level resource module wiring: `crates/maenifold-core/src/mcp/resources/mod.rs`

`graph.rs` routes all `graph://` reads into `graph_corpus::read_or_route_to_concept(...)` first, then
falls back to `graph_community::read_community_resource(...)`. `graph_corpus.rs` is therefore the file
the SWE stage must extend to register and render `graph://corpus/primer`.

## Test pattern mirrored

- Existing in-crate resource tests are wired from `src/mcp/resources/mod.rs` via `#[cfg(test)] mod ...`.
- Shared fixture setup comes from `src/mcp/resources/resources_tests_common.rs`.
- I mirrored that pattern instead of adding a new `tests/*.rs` target because `Cargo.toml` has
  `autotests = false`, so a new integration test binary would require a non-test `Cargo.toml` change.

## Primer payload shape encoded in tests

Canonical spec text describes markdown lines, not JSON:

- Child spec lines 48-54: markdown output block with one `**[[anchor]]** ...` line per community.
- Umbrella / design spec lines 181-199 and user spec lines 59-73: `graph://corpus/primer` is
  "~2-3KB markdown" with the same line-oriented shape.

Assigned task acceptance, however, asked for a JSON body plus `application/json` mime type. To reconcile
that contradiction without inventing a different rendered form, the tests encode the primer as:

```json
{
  "lines": [
    "**[[gate]]** [[task-pm]] [[red-team]] [[blue-team]] [[confession-report]]",
    "**[[mcp]]** [[gateway]] [[resources]] [[tools]] [[sampling]]"
  ]
}
```

This preserves the spec's exact markdown line shape inside a compact JSON object, gives the empty-corpus
case a valid object (`{"lines":[]}`), and keeps compatibility with the assignment's `application/json`
expectation.

## Size budget cited in tests

- Child spec line 38: target output `≤ 4KB`, hard cap `8KB`.
- The large-fixture unit test asserts the serialized payload stays at or under `8192` bytes.

## Harness gap found

There is no existing standalone primer renderer function to unit-test directly. The current graph resource
surface exposes only route-level entry points:

- `handle_read_resource(...)` in `src/mcp/resources/mod.rs`
- `graph_corpus::read_or_route_to_concept(...)` in `src/mcp/resources/graph_corpus.rs`

Smallest SWE follow-up to enable a true renderer unit test:

1. Extract a focused helper in `graph_corpus.rs` (or a new `primer.rs`) such as
   `render_corpus_primer(connection, limit) -> serde_json::Value` or `-> String`.
2. Have `graph://corpus/primer` registration call that helper.
3. Point the current red tests at that helper once it exists; until then they fail at the missing route.

## Additional verification blocker

`cargo test -p maenifold-core --no-run` does **not** currently pass on this branch because unrelated
existing integration tests fail to compile before my new tests run:

- `tests/prompt_aggregation_contract.rs`
- `tests/gateway_integration.rs`

Failure: they call private `StdioPool::start(...)` (`E0624`). I did not modify those files.
