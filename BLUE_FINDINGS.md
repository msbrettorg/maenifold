# BLUE_FINDINGS — T-303.51

## Canonical pub path targeted

- Targeted path: `maenifold_core::mcp::stdio_pool::StdioPool::start`
- Rationale: `maenifold_core::mcp` publicly exposes `pub mod stdio_pool`, and the existing external integration tests already import `StdioPool` from `maenifold_core::mcp::stdio_pool`.

## Current red-phase defect confirmation

- `crates/maenifold-core/src/mcp/stdio_pool.rs:65` still declares `pub(crate) async fn start<S>(...)`.
- `cargo test -p maenifold-core --no-run` currently fails with `error[E0624]: associated function start is private`.
- The pre-existing external integration tests that hit the same defect today are:
  - `crates/maenifold-core/tests/prompt_aggregation_contract.rs:25`
  - `crates/maenifold-core/tests/gateway_integration.rs:100`
  - `crates/maenifold-core/tests/gateway_integration.rs:129`
  - `crates/maenifold-core/tests/gateway_integration.rs:147`
  - `crates/maenifold-core/tests/gateway_integration.rs:181`

## Minimal SWE fix

- The smallest correct SWE change is the single-line visibility edit at `crates/maenifold-core/src/mcp/stdio_pool.rs:65`:
  - from: `pub(crate) async fn start<S>(...)`
  - to: `pub async fn start<S>(...)`
- Expected API impact: no signature change, no behavioral change, no new symbols beyond exposing the already-used associated function on the existing public `StdioPool` type.

## Blue-team blocker discovered while executing the assignment

- `crates/maenifold-core/Cargo.toml:5` sets `autotests = false`.
- Because of that package setting, a new file under `crates/maenifold-core/tests/` is **not** compiled unless it is also added as an explicit `[[test]]` target in `crates/maenifold-core/Cargo.toml`.
- The assignment and diff constraint allowed only three new files and forbade production-source edits; they did not permit the required manifest edit.
- Evidence:
  - New file created: `crates/maenifold-core/tests/stdio_pool_visibility_smoke.rs`
  - `cargo test -p maenifold-core --test stdio_pool_visibility_smoke --no-run` returned:

```text
error: no test target named `stdio_pool_visibility_smoke` in `maenifold-core` package
help: available test targets:
    gateway_config_rediscovery
    gateway_integration
    gateway_streamable_http
    gateway_streamable_http_cors
    list_memories_dataless
    prompt_aggregation_contract
    save_workflow_asset_dispatch
```

- Result: the new smoke file exists at the requested path, but it is currently inert until SWE/PM authorizes the matching `[[test]]` manifest entry.

