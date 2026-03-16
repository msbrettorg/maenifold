# Product Requirements Document — maenifold-rs

## Overview

maenifold-rs is the Rust reimplementation of the maenifold MCP server — a persistent cognitive manifold backed by a knowledge graph, memory corpus, and hierarchical state machines.

**Current state**: 28/28 tools implemented with C# oracle parity, 500+ tests passing, 15 sprints completed.
For completed requirements (FR-1 through FR-21), see [CHANGELOG.md](CHANGELOG.md).

---

### FR-23: Streamable HTTP Transport ✅ Sprint 26

The system SHALL support MCP Streamable HTTP transport alongside existing stdio transport, selectable via CLI flags.

- **CLI**: `maenifold --http --port <PORT>` starts HTTP server. `maenifold --mcp` continues to start stdio server.
- **Implementation**: Uses `rmcp` crate `transport-streamable-http-server` feature with `axum` web framework.
- **Dual transport**: Both transports can be compiled into the same binary. Only one is active at runtime based on CLI flags.
- **Session management**: HTTP sessions managed per MCP spec with `Mcp-Session-Id` headers.

### FR-31: OpenCode Plugin Runtime Config Helper

The system SHALL package OpenCode local plugin shared runtime helpers inside the plugin directory so relative imports resolve correctly when `~/.config/opencode/plugins` is loaded directly by OpenCode.

- **Shared helper**: `assets/integrations/opencode/plugins/lib/config.ts` SHALL provide `getMaenifoldPluginEnv(name)` and `getMaenifoldPluginFlag(name)` for all OpenCode plugin modules.
- **Resolution order**: The helper SHALL resolve `process.env` first, then fall back to `~/.config/opencode/opencode.json` → `mcp.maenifold.environment`.
- **Fail-open behavior**: Missing config files, malformed JSON/JSONC, or absent keys SHALL return `undefined` without throwing.
- **Plugin-loader safety**: Runtime plugin modules SHALL expose only callable plugin factories at top level so OpenCode's plugin loader does not attempt to invoke non-plugin helper exports.
- **Shared usage**: `persistence.ts`, `confession.ts`, and `compaction.ts` SHALL all consume the shared helper rather than duplicating config resolution logic.

### FR-32: Web UI (`--web` flag) ✅ Sprint 26

The system SHALL support a `--web` CLI flag that launches a browser-accessible web interface on the same port as the MCP HTTP endpoint, combining an ACP chat panel for interacting with AI coding agents and a force-directed knowledge graph visualization.

- **CLI**: `maenifold --web [--port <PORT>]` starts HTTP server with MCP endpoints at `/mcp` AND serves the web UI SPA at `/`. Superset of `--http`. Mutually exclusive with `--mcp`.
- **Feature flag**: Gated behind `web` cargo feature flag to keep the base binary lean. `web` implies `http`.
- **ACP integration**: WebSocket relay at `/ws/acp` provides transparent JSON-RPC passthrough to ACP agents spawned as subprocesses (stdin/stdout). No protocol translation — browser sends raw ACP JSON-RPC, relay pipes to agent.
- **Embedded SPA**: Frontend based on [ACP UI](https://github.com/formulahendry/acp-ui) Vue 3 SPA (fork at `vendor/acp-ui`), extracted from Tauri shell, compiled to static assets and embedded in the binary via `rust-embed`. Features: multi-agent, sessions, agent thinking, slash commands, permissions, model picker, traffic monitor.
- **Graph visualization**: Force-directed graph view of the knowledge graph (concepts as nodes, co-occurrence as edges, communities as clusters), exposed via REST API endpoints at `/api/graph/*`.
- **Agent configuration**: `agents.json` config file (in maenifold config dir) defining available ACP agents with command, args, and env vars. Supports fetching the [ACP registry](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json).

### ~~FR-33: Web UI — Files & History Tabs~~ (DEPRECATED)

Deprecated: ACP UI (FR-39) provides native session management. The chrome-acp proxy protocol that FR-33 targeted has been removed. File browsing is handled by the ACP agent itself.

### ~~FR-34: Web UI — Community-Scoped Graph View~~ (DEPRECATED)

Deprecated: GraphView was disabled due to performance issues (30s forced refresh). Future graph visualization will be implemented as an ACP UI Vue panel extension, not as the chrome-acp React component this FR targeted.

### FR-35: System Tray Icon ✅ Sprint 29

The system SHALL display a system tray icon (macOS menu bar / Windows system tray / Linux notification area) when running in `--web` or `--http` mode.

- **Menu items**: The tray menu SHALL include: "Open Web UI" (launches default browser to `http://localhost:{port}`), "Help" (opens `https://github.com/ma-collective/maenifold-rs/issues`), and "Quit" (graceful shutdown).
- **Lifecycle**: The tray icon appears when the server starts and disappears on shutdown.
- **Implementation**: Uses the `tray-icon` crate. The native event loop runs on the main thread; the tokio runtime and server run on a background thread.
- **Applicability**: Only active in `--web` and `--http` modes. Stdio mode (`--mcp`) is headless.

### FR-36: Remove Cargo Feature Flags ✅ Sprint 29

The system SHALL compile all functionality unconditionally — no cargo feature flags.

- **Rationale**: All features (`http`, `web`) ship in every binary. Feature flags add build matrix complexity, conditional compilation bugs, and "forgot `--features`" friction for zero benefit.
- **Scope**: Remove `http` and `web` feature flags from all `Cargo.toml` files. Remove all `#[cfg(feature = "...")]` gates. All deps compile unconditionally.
- **CLI unchanged**: `--mcp`, `--http`, `--web` remain as runtime mode selectors — they choose behavior, not compilation scope.

### FR-37: MCP Server Hardening ✅ Sprint 27

The MCP server SHALL be hardened against async correctness, resource leak, protocol compliance, and reliability issues identified by dual-model audit.

- **Async correctness**: `call_tool`, `list_resources`, and `read_resource` handlers SHALL offload blocking work (filesystem, SQLite, ONNX) to `tokio::task::spawn_blocking` instead of executing synchronously on the async runtime.
- **Session lifetime**: HTTP and web transports SHALL configure `LocalSessionManager` with a finite `keep_alive` (5 minutes) so abandoned sessions are reaped automatically.
- **CORS completeness**: `serve_web` SHALL apply CORS to the top-level router (including `/mcp`), and `localhost_cors()` SHALL expose `Mcp-Session-Id` in response headers.
- **Resource notification truthfulness**: HTTP and web transports SHALL either implement `resources/list_changed` notifications or stop advertising the capability.
- **Error classification**: `MaenifoldError::Message` SHALL be removed from `is_validation_error` to prevent internal errors from being misclassified as user input errors.
- **Panic elimination**: `replace_section` in `editing.rs` SHALL replace `expect("heading end set with section start")` with a proper `ok_or_else` error return.
- **Runtime initialization dedup**: `current_asset_manager()` SHALL not re-run `initialize_runtime` on every resource request.

### FR-38: Rust MCP Expert Agent Rewrite ✅ Sprint 27

The `rust-mcp-expert.agent.md` instruction file SHALL be rewritten to accurately reflect rmcp v1.1.0 APIs, transports, macros, and patterns. All code examples SHALL be compilable against the current SDK.

### FR-39: Replace chrome-acp with ACP UI ✅ Sprint 28

The web UI SPA SHALL be replaced from chrome-acp (React, minimal) to ACP UI (Vue 3, feature-complete).

- **Submodule swap**: Remove `vendor/chrome-acp` git submodule, add `vendor/acp-ui` (fork of [formulahendry/acp-ui](https://github.com/formulahendry/acp-ui)).
- **Tauri extraction**: Strip all `@tauri-apps/*` dependencies from the Vue SPA. Replace Tauri `invoke()`/`listen()` with a WebSocket adapter connecting to the Rust relay. Replace `plugin-store` with `localStorage`. Replace native folder picker with text input.
- **Relay simplification**: Replace chrome-acp proxy protocol (connect/new_session/prompt message types) with transparent JSON-RPC passthrough. Browser sends raw ACP JSON-RPC over WebSocket, relay pipes to agent stdin. Agent stdout pipes back to browser. Remove `agent-client-protocol` crate dependency.
- **Build system**: Update `build.rs` and `Makefile` to build and embed `vendor/acp-ui/dist` instead of `vendor/chrome-acp/packages/proxy-server/public`.
- **Features preserved**: `?agent=` query parameter enforcement, `/api/agents` REST endpoint, stderr forwarding for startup progress, agent lifecycle tied to WebSocket connection.

### FR-40: Mermaid Diagram Rendering in Chat

The web UI SHALL render Mermaid diagram code blocks in assistant chat messages as interactive SVG diagrams.

- **Detection**: Code blocks with language tag `mermaid` (` ```mermaid `) in rendered chat messages SHALL be detected and replaced with rendered SVG output.
- **Rendering**: Uses the `mermaid` npm package for client-side SVG rendering. No server-side dependency.
- **Lazy loading**: The mermaid library SHALL be loaded dynamically (`import('mermaid')`) only when the first mermaid code block is detected, to avoid bloating initial page load.
- **Dark theme**: Mermaid SHALL be initialized with `{ theme: 'dark' }` to match the application's dark UI theme.
- **Error handling**: Invalid mermaid syntax SHALL display the original code block with an error indicator rather than crashing or showing blank content.
- **Supported diagram types**: All mermaid-supported diagram types (flowchart, sequence, class, state, ER, gantt, pie, etc.) SHALL render correctly.

### FR-41: Native Desktop App via Tauri — Sprint 31

The system SHALL embed the ACP UI frontend in a native Tauri v2 webview window, replacing the browser-served SPA approach.

- **Single binary**: `maenifold --app` SHALL open a native desktop window (Tauri webview) instead of serving assets to a browser.
- **Workspace integration**: A new `crates/maenifold-app` crate SHALL own the Tauri runtime, config, and build script.
- **Tray integration**: The Tauri app SHALL provide system tray functionality (Open Window, Open at Login, Help, Quit) using Tauri's native tray APIs, replacing the current winit/tray-icon implementation for `--app` mode.
- **Frontend embedding**: The Vue frontend dist/ from `vendor/acp-ui` SHALL be embedded at compile time via Tauri's asset codegen.
- **Mode separation**: `--mcp` remains headless. `--http` retains axum-served UI for remote/browser access. `--app` is native desktop only.
- **Window behavior**: Close button hides to tray. Tray click shows window. Dock icon visible only when window is visible.

### FR-42: ACP Protocol Compliance — Sprint 32

The ACP-UI client SHALL fully comply with the Agent Client Protocol specification across all protocol areas. This encompasses initialization, session setup, prompt turn lifecycle, session configuration, slash commands, tool calls, content blocks, agent plans, session listing, file system methods, and transport compliance.

#### FR-42.1: Initialization Compliance

The client SHALL:
- Check the `protocolVersion` returned by the agent in the `initialize` response and disconnect with a clear error if the version is unsupported.
- Store `agentInfo` (name, title, version) from the `initialize` response for display and diagnostics.
- Store `agentCapabilities.promptCapabilities` and gate image/audio/resource UI features accordingly.
- Extract a shared `performInitialization()` helper to eliminate duplicate `initialize` blocks in `createSession` and `resumeSession`.

#### FR-42.2: Session Setup Compliance

The client SHALL:
- Validate that `cwd` is an absolute path before calling `session/new`. The `'.'` fallback MUST be removed.
- Re-check `agentCapabilities.loadSession` from the live `initialize` response before calling `session/load` in `resumeSession`.
- Add abort guards and `isConnecting` flag to `resumeSession` matching the `createSession` pattern.

#### FR-42.3: Prompt Turn Compliance

The client SHALL:
- Call `cancelPermission()` from within `cancelOperation()` when a permission request is pending (MUST per spec).
- Preemptively mark all non-finished tool calls as `cancelled` when sending `session/cancel`.
- Differentiate `stopReason` values (`refusal`, `max_tokens`, `max_turn_requests`) in the UI with appropriate feedback.

#### FR-42.4: Session Config Options

The client SHALL:
- Render `ConfigOptionPicker` components for each `configOption` returned by the agent.
- Suppress legacy `ModePicker`/`ModelPicker` when `configOptions` is present and non-empty.
- Implement `unstable_setSessionConfigOption` on `AcpClientBridge` to call `session/set_config_option`.
- Filter unrecognized option `type` values (only render `select`).

#### FR-42.5: Tool Call Content

The client SHALL:
- Store the `content` field from `tool_call` and `tool_call_update` notifications.
- Accumulate content from successive `tool_call_update` notifications.
- Store `locations` updates from `tool_call_update`.
- Add `line` to the `locations` type definition.

#### FR-42.6: Agent Plan Support

The client SHALL:
- Handle `plan` session update notifications and store plan entries in reactive state.
- Render plan entries with `content`, `priority` badge, and `status` indicator.

#### FR-42.7: Session Info Updates

The client SHALL:
- Handle `session_info_update` notifications to update session title and timestamp.
- Remove the client-side first-message title hack in `sendPrompt`.

#### FR-42.8: Slash Command Hint

The client SHALL:
- Render the `hint` field from slash commands as textarea placeholder text after command selection.
- Remove the dead code branch in `handleCommandSelect`.

#### FR-42.9: File System Compliance

The client SHALL:
- Validate that `path` is absolute before executing `fs/read_text_file` and `fs/write_text_file`.
- Create parent directories before writing files (`mkdir -p` equivalent).
- Return `null` (not `{}`) from successful `writeTextFile` calls.

#### FR-42.10: Transport & Extensibility Compliance

The client SHALL:
- Convert WebSocket relay out-of-band frames to valid JSON-RPC `_relay/*` notifications.
- Fix event listener leaks in `ModePicker.vue` and `ModelPicker.vue` (add `onUnmounted` cleanup).

### NFR-9: Non-Text Content Blocks

The system SHOULD log warnings when receiving non-text content blocks instead of silently dropping them. Full rendering of image/audio/resource content blocks is deferred to a future sprint.
