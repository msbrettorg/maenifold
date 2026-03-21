# web-cli-flag

**Requirement**: FR-32
**Test Cases**: TC-32.1, TC-32.2
**Component**: crates/maenifold-core/src/cli.rs

## Acceptance Criteria

1. `maenifold --web` starts the axum HTTP server serving the SPA at `/` and MCP at `/mcp` on the same port
2. `--web` accepts `--port <PORT>` (default 8080), same as `--http`
3. The existing `--port` guard in `cli.rs` SHALL be updated to accept `--port` when either `--http` OR `--web` is specified
4. `--web` is mutually exclusive with both `--mcp` AND `--http` (error if either or both are also specified)
5. `--web` implies all `--http` behavior — all 28 MCP tools remain accessible via HTTP transport at `/mcp`
6. When `web` feature is not compiled in, `--web` prints "Error: Web UI is not enabled in this build" and exits with code 1
7. On startup, prints: `maenifold web UI listening on http://127.0.0.1:{port}/`
8. `--help` output SHALL include `maenifold --web [--port <PORT>]` alongside existing modes

## Verdicts

- TC-32.1 (--web starts SPA + MCP server): ADD — test does not exist yet
- TC-32.2 (--web implies --http behavior): ADD — test does not exist yet
