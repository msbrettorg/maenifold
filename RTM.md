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

- ✅ RTM-204: `maenifold --web` starts axum server serving SPA at `/` and MCP at `/mcp` on same port | TC-32.1 | CLI
- ✅ RTM-205: `--web` implies `--http` behavior — all 28 MCP tools accessible via HTTP transport | TC-32.2 | Integration
- ✅ RTM-206: `web` cargo feature flag gates all web UI dependencies; `web` implies `http` | TC-32.3 | Build
- ✅ RTM-207: ACP agent subprocess spawned from agents.json config via `agent-client-protocol` crate | TC-32.4 | Integration
- ✅ RTM-208: ACP `initialize` + `new_session` handshake completes with spawned agent | TC-32.5 | Integration
- ✅ RTM-209: WebSocket endpoint at `/ws/acp` relays messages between browser and ACP agent stdin/stdout via maenifold relay protocol | TC-32.6 | Integration
- ✅ RTM-210: Agent prompt responses stream to browser via WebSocket as ACP session notifications | TC-32.7 | Integration
- ✅ RTM-211: Static SPA assets embedded in binary via `rust-embed` and served at `/` with correct MIME types | TC-32.8 | Build+Integration
- ✅ RTM-212: `GET /api/graph/nodes` returns concept nodes with name, file count (derived), community ID (nullable), and degree (derived) | TC-32.9 | Integration
- ✅ RTM-213: `GET /api/graph/edges` returns co-occurrence edges with source, target, and weight (integer count) | TC-32.10 | Integration
- ✅ RTM-214: Graph visualization renders force-directed layout from `/api/graph/*` data in the SPA | TC-32.11 | Frontend
- ✅ RTM-215: `agents.json` config file parsed at startup to populate available ACP agents | TC-32.12 | Unit
- ✅ RTM-216: ACP agent spawn failure returns error without panic | TC-32.13 | Unit
- ✅ RTM-217: ACP initialize handshake has 10-second timeout | TC-32.14 | Integration
- ✅ RTM-218: Agent command in agents.json validated (absolute path, exists, executable) | TC-32.15 | Unit

### ~~FR-33: Web UI — Files & History Tabs~~ (DEPRECATED)

- ~~RTM-219~~ through ~~RTM-222~~: Deprecated — chrome-acp proxy protocol removed (FR-39). ACP UI handles sessions natively.

### ~~FR-34: Web UI — Community-Scoped Graph View~~ (DEPRECATED)

- ~~RTM-223~~ through ~~RTM-226~~: Deprecated — GraphView disabled, future graph viz will be an ACP UI panel extension.

### FR-37: MCP Server Hardening

- ✅ RTM-227: call_tool offloads to spawn_blocking | TC-37.1 | Unit
- ✅ RTM-228: list_resources offloads to spawn_blocking | TC-37.2 | Unit
- ✅ RTM-229: read_resource offloads to spawn_blocking | TC-37.3 | Unit
- ✅ RTM-230: HTTP sessions expire after 5 min idle via keep_alive config | TC-37.4 | Integration
- ✅ RTM-231: CORS applied to /mcp endpoint in web mode | TC-37.5 | Integration
- ✅ RTM-232: Mcp-Session-Id exposed in CORS response headers | TC-37.6 | Integration
- ✅ RTM-233: HTTP/web transports do not advertise resources/list_changed | TC-37.7 | Unit
- ✅ RTM-234: MaenifoldError::Message not in is_validation_error | TC-37.8 | Unit
- ✅ RTM-235: replace_section returns error instead of panic on missing heading_end | TC-37.9 | Unit
- ✅ RTM-236: current_asset_manager does not call initialize_runtime | TC-37.10 | Unit

### FR-38: Rust MCP Expert Agent Rewrite

- ✅ RTM-237: Agent file references rmcp 1.1.0 | TC-38.1 | Review
- ✅ RTM-238: All code examples compile against rmcp 1.1.0 | TC-38.2 | Review
- ✅ RTM-239: Transport section covers stdio + streamable HTTP correctly | TC-38.3 | Review

### FR-39: Replace chrome-acp with ACP UI

- ✅ RTM-240: chrome-acp submodule removed, acp-ui submodule added at vendor/acp-ui | TC-39.1 | Build
- ✅ RTM-241: Zero @tauri-apps imports in vendor/acp-ui/src/ | TC-39.2 | Build
- ✅ RTM-242: src/lib/relay.ts WebSocket adapter replaces tauri.ts | TC-39.3 | Build
- ✅ RTM-243: localStorage replaces @tauri-apps/plugin-store for sessions and prefs | TC-39.4 | Build
- ✅ RTM-244: Relay passes raw JSON-RPC between browser WebSocket and agent stdin/stdout | TC-39.5 | Integration
- ✅ RTM-245: ?agent= query parameter enforcement preserved on /ws/acp | TC-39.6 | Integration
- ✅ RTM-246: Agent stderr forwarded as {"type":"stderr","data":"..."} messages | TC-39.7 | Integration
- ✅ RTM-247: agent-client-protocol crate dependency removed | TC-39.8 | Build
- ✅ RTM-248: build.rs points at vendor/acp-ui/dist | TC-39.9 | Build
- ✅ RTM-249: make web-build produces working binary with embedded ACP UI SPA | TC-39.10 | Build
- ✅ RTM-250: ACP UI renders in browser with agent dropdown, sessions, chat, model picker | TC-39.11 | Frontend
- ✅ RTM-251: Full ACP round-trip: select agent → connect → prompt → response rendered | TC-39.12 | Integration

## FR-35: System Tray Icon (Sprint 29)

- ✅ RTM-260: tray-icon + winit + muda crates added to maenifold-core | Build
- ✅ RTM-261: Tray icon appears in macOS menu bar when running --web | Functional
- ✅ RTM-262: Tray icon appears in macOS menu bar when running --http | Functional
- ✅ RTM-263: "Open Web UI" menu item opens browser to localhost:port | Functional
- ✅ RTM-264: "Help" menu item opens GitHub issues page | Functional
- ✅ RTM-265: "Quit" menu item triggers graceful server shutdown via CancellationToken | Functional
- ✅ RTM-266: --mcp mode remains headless — no tray icon, no native event loop | Functional
- ✅ RTM-267: Main thread runs native event loop (macOS requirement); tokio on background thread | Architecture
- ✅ RTM-268: Tray icon disappears on shutdown | Functional
- ✅ RTM-269: Cognitive Geodesics icon asset at assets/tray-icon.png | Asset

## FR-36: Remove Cargo Feature Flags (Sprint 29)

- ✅ RTM-270: [features] section removed from maenifold-core Cargo.toml | Build
- ✅ RTM-271: [features] section removed from maenifold-cli Cargo.toml | Build
- ✅ RTM-272: All #[cfg(feature = "...")] gates removed from crates/ | Code
- ✅ RTM-273: --features flag removed from Makefile | Build
- ✅ RTM-274: cargo build --release -p maenifold-cli succeeds without --features | Build
- ✅ RTM-275: All previously feature-gated deps compile unconditionally | Build

## ACP Protocol Compliance — Sprint 32

### FR-42.1: Initialization Compliance (`stores/session.ts`)

- □ RTM-276: `initResponse.protocolVersion` checked; mismatch disconnects with error | TC-42.1.1 | Unit
- □ RTM-277: `agentInfo` stored from `initResponse` and accessible in store | TC-42.1.2 | Unit
- □ RTM-278: `promptCapabilities` stored from `initResponse.agentCapabilities` | TC-42.1.3 | Unit
- □ RTM-279: Shared `performInitialization()` replaces duplicate initialize blocks | TC-42.1.4 | Refactor

### FR-42.2: Session Setup Compliance (`stores/session.ts`, `App.vue`)

- □ RTM-280: `cwd` validated as absolute path before `session/new`; `'.'` fallback removed | TC-42.2.1 | Unit
- □ RTM-281: `resumeSession` re-checks live `loadSession` capability from `initResponse` | TC-42.2.2 | Unit
- □ RTM-282: `resumeSession` has abort guards and `isConnecting` flag | TC-42.2.3 | Unit

### FR-42.3: Prompt Turn Compliance (`stores/session.ts`)

- □ RTM-283: `cancelOperation()` calls `cancelPermission()` when permission pending | TC-42.3.1 | Unit
- □ RTM-284: `cancelOperation()` marks non-finished tool calls as `cancelled` | TC-42.3.2 | Unit
- □ RTM-285: `stopReason` values produce differentiated UI feedback | TC-42.3.3 | UI

### FR-42.4: Session Config Options (`stores/session.ts`, `ChatView.vue`, `acp-bridge.ts`)

- □ RTM-286: `ConfigOptionPicker` rendered for each `configOption` in `ChatView.vue` | TC-42.4.1 | UI
- □ RTM-287: Legacy pickers suppressed when `configOptions` present | TC-42.4.2 | UI
- □ RTM-288: `unstable_setSessionConfigOption` implemented on `AcpClientBridge` | TC-42.4.3 | Unit
- □ RTM-289: Unrecognized option `type` values filtered (only `select` rendered) | TC-42.4.4 | Unit

### FR-42.5: Tool Call Content (`stores/session.ts`, `types.ts`)

- □ RTM-290: `ToolCallInfo` type includes `content` field | TC-42.5.1 | Type
- □ RTM-291: `tool_call` handler stores `content` from notification | TC-42.5.2 | Unit
- □ RTM-292: `tool_call_update` handler accumulates `content` and updates `locations` | TC-42.5.3 | Unit

### FR-42.6: Agent Plan Support (`stores/session.ts`, `PlanView.vue`)

- □ RTM-293: `plan` session update stored in reactive `currentPlan` state | TC-42.6.1 | Unit
- □ RTM-294: `PlanView.vue` renders entries with content, priority, status | TC-42.6.2 | UI

### FR-42.7: Session Info Updates (`stores/session.ts`)

- □ RTM-295: `session_info_update` handler updates session title and timestamp | TC-42.7.1 | Unit
- □ RTM-296: Client-side first-message title hack removed from `sendPrompt` | TC-42.7.2 | Refactor

### FR-42.8: Slash Command Hint (`ChatView.vue`, `CommandPalette.vue`)

- □ RTM-297: `hint` rendered as textarea placeholder after command selection | TC-42.8.1 | UI
- □ RTM-298: Dead code branch in `handleCommandSelect` removed | TC-42.8.2 | Refactor

### FR-42.9: File System Compliance (`acp-bridge.ts`)

- □ RTM-299: `fs/read_text_file` and `fs/write_text_file` validate absolute path | TC-42.9.1 | Unit
- □ RTM-300: `writeTextFile` creates parent directories before writing | TC-42.9.2 | Unit
- □ RTM-301: `writeTextFile` returns `null` on success | TC-42.9.3 | Unit

### FR-42.10: Transport & Extensibility (`relay.rs`, `ModePicker.vue`, `ModelPicker.vue`)

- □ RTM-302: WebSocket relay stderr/error frames are `_relay/*` JSON-RPC notifications | TC-42.10.1 | Unit
- □ RTM-303: `ModePicker.vue` removes click listener on unmount | TC-42.10.2 | Unit
- □ RTM-304: `ModelPicker.vue` removes click listener on unmount | TC-42.10.3 | Unit

### NFR-9: Non-Text Content Blocks (`stores/session.ts`)

- □ RTM-305: Non-text content blocks logged as warnings instead of silently dropped | TC-NFR9.1 | Unit
