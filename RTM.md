# Requirements Traceability Matrix — maenifold-rs

For completed requirements (RTM-001 through RTM-114, RTM-147 through RTM-156), see [CHANGELOG.md](CHANGELOG.md).

---

## Hosted Deployment — Dual-Transport, Dual-Backend

### FR-23: Streamable HTTP Transport (`mcp/http.rs`)

- ✅ RTM-126: `maenifold --http --port 8080` starts axum HTTP server | TC-23.1 | CLI
- ✅ RTM-127: HTTP server exposes MCP Streamable HTTP endpoints (POST /mcp, GET /mcp, DELETE /mcp) | TC-23.2 | Integration
- ✅ RTM-128: `maenifold --mcp` continues to start stdio server (unchanged) | TC-23.3 | CLI
- ✅ RTM-129: HTTP sessions tracked via `Mcp-Session-Id` header | TC-23.4 | Integration
- ✅ RTM-130: All 28 tools invokable via HTTP transport | TC-23.5 | Integration

### FR-31: OpenCode Plugin Runtime Config Helper (`assets/integrations/opencode/plugins/`)

- □ RTM-195: `lib/config.ts` resolves `process.env` values before consulting OpenCode config fallback | TC-31.1 | Boundary
- □ RTM-196: Unset plugin env values fall back to `~/.config/opencode/opencode.json` → `mcp.maenifold.environment` | TC-31.2 | Boundary
- □ RTM-197: Missing or malformed OpenCode config fails open without throwing | TC-31.3 | Boundary
- □ RTM-198: `persistence.ts` and `confession.ts` remain plugin-loader safe by exposing only callable plugin factories at module top level | TC-31.4 | Integration
- □ RTM-199: `compaction.ts`, `confession.ts`, and `persistence.ts` share the same helper under the runtime plugin tree | TC-31.5 | Integration
- □ RTM-200: OpenCode plugin tests pass from repo-backed plugin path used by the `~/.config/opencode/plugins` symlink | TC-31.6 | Build+Integration

### NFR-8: Decay & Access Boosting

- □ RTM-201: ReadMemory refreshes last_accessed timestamp in file_content on each successful read (by URI and by title) | TC-NFR8.1 | Boundary
- □ RTM-202: ReadMemory on non-existent identifier does not create or update last_accessed | TC-NFR8.2 | Boundary
- □ RTM-203: ReadMemory with learn=true short-circuit does not update last_accessed | TC-NFR8.3 | Boundary
