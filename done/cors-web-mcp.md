---
id: T-80.43
title: Fix CORS in Web Mode + Expose Mcp-Session-Id
status: done
pipeline: sdd
---

# T-27.3: Fix CORS in Web Mode + Expose Mcp-Session-Id

**Requirement:** FR-37 (RTM-231, RTM-232)

## Problem

1. In `serve_web()`, CORS is only applied to the API sub-router, not the `/mcp` nest_service. Browser MCP clients get blocked.
2. `localhost_cors()` does not expose `Mcp-Session-Id` in response headers. Browser JS cannot read it from cross-origin responses.

## Acceptance Criteria

1. In `web.rs`, CORS layer is applied to the top-level router (wrapping both `/mcp` and the API routes)
2. In `mod.rs`, `localhost_cors()` adds `.expose_headers(["Mcp-Session-Id"])` to the CorsLayer
3. `cargo check -p maenifold-core` passes

## Files to Modify

- `crates/maenifold-core/src/mcp/web.rs` — move `.layer(cors)` to top-level router
- `crates/maenifold-core/src/mcp/mod.rs` — add `expose_headers` to `localhost_cors()`

## Scoped Test Command

```bash
cargo test -p maenifold-core mcp::tests
```

## SWE Remediation

- Changed `crates/maenifold-core/src/mcp/web.rs` so `serve_web()` applies `localhost_cors()` at the top-level router, covering both the `/mcp` `nest_service` and the merged API/static routes.
- Changed `crates/maenifold-core/src/mcp/mod.rs` so `localhost_cors()` exposes the `Mcp-Session-Id` response header for browser clients.

### Test Results

- `cargo check -p maenifold-core` — **PASS**
- `cargo test -p maenifold-core mcp::tests` — **FAIL**
  - The command built and the `mcp::tests` unit tests passed (`8 passed`), but the run still exited non-zero because the repository's `integration_tests` binary enforces a policy that rejects this filter as a prohibited full-suite invocation:
    - `POLICY VIOLATION: Full test suite runs are prohibited.`
    - `Use targeted tests: cargo test -p maenifold-core --test integration_tests -- <module>::<test_name>`

### Overall

- **FAIL** — implementation is complete and `cargo check` passed, but the spec’s scoped test command exits non-zero in the current repo due to the existing integration test policy guard.

---

## Red-team Audit

**Auditor:** red-team agent  
**Verdict: PASS**

### Criterion 1 — CORS layer wraps the top-level router (including `/mcp`)

✅ **PASS**

In `web.rs`, `.layer(cors)` is called on the fully-assembled top-level `axum::Router` *after* both `.nest_service("/mcp", service)` and `.merge(api)` have been applied:

```rust
let router = axum::Router::new()
    .nest_service("/mcp", service)
    .merge(api)
    .layer(cors);   // ← wraps the entire router, including /mcp
```

In axum, `Router::layer` wraps every route already registered on the router, so the `CorsLayer` is applied uniformly to `/mcp/**`, the merged API paths, and the SPA fallback. There is no gap.

### Criterion 2 — `localhost_cors()` exposes `Mcp-Session-Id`

✅ **PASS**

In `mod.rs`, the builder chain terminates with:

```rust
CorsLayer::new()
    .allow_origin(origin_predicate)
    .allow_methods(AllowMethods::any())
    .allow_headers(AllowHeaders::any())
    .expose_headers(["Mcp-Session-Id"])
```

The `expose_headers` call is present and uses the exact header name required by the spec. `tower_http`'s `CorsLayer` propagates this to the `Access-Control-Expose-Headers` response header so browser JS can read `Mcp-Session-Id` from cross-origin responses.

### Criterion 3 — `cargo check -p maenifold-core` passes

✅ **PASS**

```
warning: function `format_source_files_for_display` is never used
warning: variant `Http` is never constructed
warning: `maenifold-core` (lib) generated 2 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.35s
```

Exit code 0. Both warnings are pre-existing dead-code notices unrelated to this change.

---

### Supplementary Observations (non-blocking)

**[Low] Overly permissive `allow_methods` / `allow_headers` for localhost**  
`AllowMethods::any()` and `AllowHeaders::any()` allow any HTTP method and any request header from localhost origins. Since the origin predicate is already restricted to `http://localhost:*`, `http://127.0.0.1:*`, and `http://[::1]:*`, the practical attack surface is narrow (exploiting this requires a malicious page already running on the developer's localhost). No immediate risk, but narrowing these to the methods and headers actually needed by the MCP protocol (e.g., `GET, POST, DELETE` and `Content-Type, Accept, Mcp-Session-Id`) would reduce unnecessary surface area in future. The spec does not require this tightening, so it is noted informally.

**[Informational] `localhost_cors()` is `pub(crate)` and guarded by `any(feature = "http", feature = "web")`**  
The shared factory is correctly scoped: both feature gates that need HTTP-level CORS can reach it, but it is not exposed outside the crate. Good hygiene.

**[Informational] `http.rs` feature gate**  
`localhost_cors()` is available under `feature = "http"` but this audit was not asked to verify `http.rs` usage. If the HTTP-only mode also serves browser clients, it should apply the same layer. This is outside the stated scope and is flagged only for awareness.

---

All three stated acceptance criteria are satisfied. No spec violations, no security regressions.

## Blue-team Verification

**Verdict: PASS**

- Command run: `cargo test -p maenifold-core mcp::tests`
- Result: exit code 0
- Scoped evidence:
  - `mcp::tests` unit tests: `8 passed; 0 failed`
  - filtered integration test binary also completed cleanly: `0 passed; 0 failed; 878 filtered out`

### Command Output Summary

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.51s
Running unittests src/lib.rs (target/debug/deps/maenifold_core-100ffec8141f5414)

running 8 tests
test mcp::tests::t_27_5_message_errors_are_not_validation_errors ... ok
test mcp::tests::t_27_5_message_resource_errors_map_to_internal_error ... ok
test mcp::tests::t_5_2_write_memory_schema_requires_title_and_content ... ok
test mcp::tests::t_5_1_handler_server_info_matches_expected ... ok
test mcp::tests::t_27_4_http_handler_does_not_advertise_resource_list_changed ... ok
test mcp::tests::t_5_2_registry_names_match_canonical_names ... ok
test mcp::tests::t_5_1_handler_builds_28_rmcp_tools ... ok
test mcp::tests::t_5_2_registry_mapping_covers_all_tools ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 73 filtered out; finished in 0.00s

Running tests/main.rs (target/debug/deps/integration_tests-0e00f9883bef02ed)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 878 filtered out; finished in 0.00s
```
