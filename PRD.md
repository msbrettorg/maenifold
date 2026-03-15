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
