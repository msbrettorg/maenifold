---
id: initialize-runtime-dedup
status: done
pipeline: sdd
---

# T-27.7: Remove Redundant initialize_runtime from Resource Handlers

**Requirement:** FR-37 (RTM-236)

## Problem

`current_asset_manager()` in `crates/maenifold-core/src/mcp/resources.rs` calls `initialize_runtime(&config)` on every `resources/list` and `resources/read` request. This opens a SQLite connection and runs DDL statements unnecessarily — `initialize_runtime` is already called at server startup.

## Acceptance Criteria

1. `current_asset_manager()` no longer calls `initialize_runtime()`
2. It still creates `Config::current()` and returns `AssetManager::new(&config)`
3. Server startup paths (`serve_stdio`, `serve_http`, `serve_web`) still call `initialize_runtime` — verify they do
4. `cargo check -p maenifold-core` passes

## Files to Modify

- `crates/maenifold-core/src/mcp/resources.rs` — `current_asset_manager()` function

## Scoped Test Command

```bash
cargo test -p maenifold-core mcp::tests
```

## SWE Remediation

- **What I changed**
  - Removed the redundant `initialize_runtime(&config)` call from `crates/maenifold-core/src/mcp/resources.rs::current_asset_manager()`.
  - Verified startup initialization remains in place by ensuring the HTTP and web server startup paths call `initialize_runtime` before serving, matching the existing stdio startup behavior.
- **Test results**
  - `cargo check -p maenifold-core` ✅ Passed
  - `MAENIFOLD_FULL_SUITE=1 cargo test -p maenifold-core mcp::tests` ✅ Passed
    - Unit tests: 7 passed, 0 failed
    - Integration test binary: 0 matched tests, 0 failed, 878 filtered out
- **Pass/Fail**
  - **PASS**

## Red-team Audit

**Verdict: PASS**

Audited commit: `dc5e7a6` (HEAD → main)

### Check 1 — `current_asset_manager()` no longer calls `initialize_runtime`

✅ **CONFIRMED.** `crates/maenifold-core/src/mcp/resources.rs` lines 62–65:

```rust
pub(crate) fn current_asset_manager() -> Result<AssetManager> {
    let config = Config::current();
    Ok(AssetManager::new(&config))
}
```

No call to `initialize_runtime`. The function is exactly what the spec requires: build a `Config`, return an `AssetManager`.

### Check 2 — All three server startup paths call `initialize_runtime`

✅ **CONFIRMED.**

| Path | File | Line | Call |
|------|------|------|------|
| `serve_stdio` | `mcp/mod.rs` | 190 | `initialize_runtime(&config)?` |
| `serve_http` | `mcp/http.rs` | 14 | `super::initialize_runtime(&config)?` |
| `serve_web` | `mcp/web.rs` | 26 | `super::initialize_runtime(&config)?` |

All three functions call `initialize_runtime` as the first meaningful operation before creating a handler or accepting connections.

### Check 3 — No other `current_asset_manager()` callers depend on init happening there

✅ **CONFIRMED.** The only call sites are:

- `handle_list_resources()` — invoked only by live MCP request dispatch (post-startup).
- `handle_read_resource()` — same; only reached after a client sends a `resources/read` request.
- `resources_tests.rs` — explicitly calls `initialize_runtime(&config)` in its own test setup before touching `current_asset_manager()`. No silent dependency on the old side-effect.

None of these are entry points that could be reached before a server startup path has run.

### Observations (non-blocking)

1. **Pre-existing double-init on stdio and HTTP paths.** `cli.rs::run_mcp()` (line 100) and `cli.rs::run_http()` (line 130) both call `mcp::initialize_runtime` themselves, and then the respective `serve_*` functions call it again internally. This means `initialize_runtime` executes twice on those paths. This is **not introduced by T-27.7** — it pre-dates this change — and it is safe because `initialize_database` uses `CREATE TABLE IF NOT EXISTS` DDL throughout (`db/schema.rs`), making the call idempotent. Still worth tracking as a minor inefficiency in a future cleanup ticket.

2. **`run_web` in cli.rs relies entirely on `serve_web` for init.** Unlike `run_mcp`/`run_http`, `run_web` (cli.rs line 158) does not call `initialize_runtime` itself; it delegates the full call to `serve_web`. This is consistent and correct — not a defect.

3. **`run_inner` (direct `--tool` dispatch) is independent.** Its call to `initialize_runtime` at cli.rs line 180 is unrelated to the MCP resource handler path and is unaffected by this change.

## Blue-team Verification

**Verdict: PASS**

- **Scoped command run**
  - `cargo test -p maenifold-core mcp::tests`
- **Observed result**
  - Lib unit tests ran and passed: `running 8 tests`
  - Lib test summary: `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 73 filtered out`
  - Integration binary also completed without matched failures: `running 0 tests` and `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 878 filtered out`
  - Command exit code: `0`
- **Acceptance criteria spot-check**
  - `crates/maenifold-core/src/mcp/resources.rs::current_asset_manager()` now only builds `Config::current()` and returns `AssetManager::new(&config)`.
  - Startup initialization remains present in:
    - `crates/maenifold-core/src/mcp/mod.rs::serve_stdio`
    - `crates/maenifold-core/src/mcp/http.rs::serve_http`
    - `crates/maenifold-core/src/mcp/web.rs::serve_web`
- **Verified against**
  - Spec: `docs/sprint/todo/initialize-runtime-dedup.md`
  - Audited commit: `dc5e7a6`
