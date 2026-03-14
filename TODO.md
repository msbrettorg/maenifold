# Backlog — maenifold-rs

For completed work (Sprints 1–3, 9–15, 20–22), see [CHANGELOG.md](CHANGELOG.md).

---

## Sprint 17 — Streamable HTTP Transport

### T-17.1: Add rmcp HTTP transport feature + axum dependency | RTM: FR-23

Update workspace `Cargo.toml`: add `transport-streamable-http-server` to rmcp features (gated behind `http` feature flag). Add `axum = "0.7"` and `tower-http = { version = "0.6", features = ["cors"] }` dependencies.

- **Files**: `Cargo.toml`, `crates/maenifold-core/Cargo.toml`
- **Status**: pending

### T-17.2: Implement `serve_http()` in MCP server | RTM: FR-23

Add `MaenifoldMcpServer::serve_http(port: u16)` alongside existing `serve_stdio()`. Uses `StreamableHttpService` from rmcp mounted on axum router. CORS permissive. Session tracking via `Mcp-Session-Id` headers. Same `MaenifoldHandler` used for both transports.

- **Files**: `crates/maenifold-core/src/mcp/http.rs` (new), `crates/maenifold-core/src/mcp/mod.rs`
- **Status**: pending
- **Depends on**: T-17.1

### T-17.3: CLI `--http` flag and `--port` option | RTM: FR-23

Add `--http` and `--port <PORT>` CLI arguments. `--http` starts `serve_http()`. `--mcp` unchanged. Mutually exclusive flags. Default port 8080.

- **Files**: `crates/maenifold-cli/src/main.rs` or `crates/maenifold-core/src/cli.rs`
- **Status**: pending
- **Depends on**: T-17.2

### T-17.5: HTTP transport tests | RTM: FR-23

Integration tests: start HTTP server on random port, send MCP initialize + tool/call requests via reqwest, verify JSON-RPC responses. Test session creation and cleanup.

- **Files**: `crates/maenifold-core/tests/http_transport_tests.rs` (new)
- **Status**: pending
- **Depends on**: T-17.2

---

### T-24.1: Add OpenCode shared runtime config helper + loader-safe exports | RTM: FR-31 (RTM-195 through RTM-200)

Implement `assets/integrations/opencode/plugins/lib/config.ts` and wire `persistence.ts`, `confession.ts`, and `compaction.ts` to consume it under the runtime plugin tree used by the `~/.config/opencode/plugins` symlink.
AC-1: `getMaenifoldPluginEnv(name)` returns `process.env[name]` when explicitly set
AC-2: Unset env values fall back to `~/.config/opencode/opencode.json` → `mcp.maenifold.environment`
AC-3: `getMaenifoldPluginFlag(name)` parses string booleans and returns `true`, `false`, or `undefined` without throwing
AC-4: Missing or malformed OpenCode config fails open and preserves plugin execution
AC-5: `persistence.ts` and `confession.ts` expose only callable plugin factories at module top level while keeping test seams reachable via attached properties
AC-6: `compaction.ts`, `confession.ts`, and `persistence.ts` all resolve shared helper imports successfully from the repo-backed plugin symlink path
AC-7: Focused OpenCode plugin tests pass from `assets/integrations/opencode/tests/plugins`

- **Status**: pending
- **Depends on**: none

---

## Maintenance — Traceability & Test Gaps

### T-GRAPH-DECAY-002: Access-boosting boundary tests for ReadMemory | RTM: NFR-8

Boundary tests validating that ReadMemory correctly maintains the `last_accessed` timestamp in the `file_content` table for the [[decay-weighting]] system. Covers positive paths (URI-based read, title-based read, sequential reads) and negative paths (missing identifier, learn-mode short-circuit).

- **Files**: `crates/maenifold-core/tests/access_boosting_reads.rs`, `crates/maenifold-core/tests/access_boosting/support.rs`
- **Status**: complete
