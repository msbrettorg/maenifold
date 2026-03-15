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

- ✅ RTM-201: ReadMemory refreshes last_accessed timestamp in file_content on each successful read (by URI and by title) | TC-NFR8.1 | Boundary
- ✅ RTM-202: ReadMemory on non-existent identifier does not create or update last_accessed | TC-NFR8.2 | Boundary
- ✅ RTM-203: ReadMemory with learn=true short-circuit does not update last_accessed | TC-NFR8.3 | Boundary

### FR-32: Web UI (`--web` flag)

- □ RTM-204: `maenifold --web` starts axum server serving SPA at `/` and MCP at `/mcp` on same port | TC-32.1 | CLI
- □ RTM-205: `--web` implies `--http` behavior — all 28 MCP tools accessible via HTTP transport | TC-32.2 | Integration
- □ RTM-206: `web` cargo feature flag gates all web UI dependencies; `web` implies `http` | TC-32.3 | Build
- □ RTM-207: ACP agent subprocess spawned from agents.json config via `agent-client-protocol` crate | TC-32.4 | Integration
- □ RTM-208: ACP `initialize` + `new_session` handshake completes with spawned agent | TC-32.5 | Integration
- □ RTM-209: WebSocket endpoint at `/ws/acp` relays messages between browser and ACP agent stdin/stdout via maenifold relay protocol | TC-32.6 | Integration
- □ RTM-210: Agent prompt responses stream to browser via WebSocket as ACP session notifications | TC-32.7 | Integration
- □ RTM-211: Static SPA assets embedded in binary via `rust-embed` and served at `/` with correct MIME types | TC-32.8 | Build+Integration
- □ RTM-212: `GET /api/graph/nodes` returns concept nodes with name, file count (derived), community ID (nullable), and degree (derived) | TC-32.9 | Integration
- □ RTM-213: `GET /api/graph/edges` returns co-occurrence edges with source, target, and weight (integer count) | TC-32.10 | Integration
- □ RTM-214: Graph visualization renders force-directed layout from `/api/graph/*` data in the SPA | TC-32.11 | Frontend
- □ RTM-215: `agents.json` config file parsed at startup to populate available ACP agents | TC-32.12 | Unit
- □ RTM-216: ACP agent spawn failure returns error without panic | TC-32.13 | Unit
- □ RTM-217: ACP initialize handshake has 10-second timeout | TC-32.14 | Integration
- □ RTM-218: Agent command in agents.json validated (absolute path, exists, executable) | TC-32.15 | Unit

### FR-33: Web UI — Files & History Tabs

- □ RTM-219: `list_dir` WebSocket message returns directory listing from agent cwd | TC-33.1 | Integration
- □ RTM-220: `read_file` WebSocket message returns file contents with MIME type | TC-33.2 | Integration
- □ RTM-221: `list_sessions` forwarded to ACP agent and response relayed to browser | TC-33.3 | Integration
- □ RTM-222: `load_session` replays session history to browser via session_update messages | TC-33.4 | Integration

### FR-34: Web UI — Community-Scoped Graph View

- □ RTM-223: `GET /api/graph/communities` returns community summaries with label, member count, top concepts | TC-34.1 | Integration
- □ RTM-224: Default graph view shows community nodes, not individual concepts | TC-34.2 | Frontend
- □ RTM-225: Clicking a community node drills into its member concepts | TC-34.3 | Frontend
- □ RTM-226: "Back to communities" button returns to overview | TC-34.4 | Frontend
