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

### FR-43: ACP Protocol Compliance — Prompt Capabilities — Sprint 33

The ACP-UI client SHALL implement all remaining ACP protocol compliance gaps identified by dual-model audit, covering prompt capabilities, content block fidelity, session management, and transport correctness. (21 specs, see CHANGELOG.md for details.)

### FR-44: Multi-Tab Sessions — Sprint 34

The ACP-UI client SHALL support multiple simultaneous agent sessions as tabs, replacing the singleton session model.

- **Tab lifecycle**: Each `createSession` or `resumeSession` opens a new tab. Closing a tab disconnects its ACP bridge.
- **Tab state isolation**: Each tab owns its own messages, tool calls, config options, scroll position, and input draft.
- **Sidebar as launcher**: The sidebar agent list remains always available. Selecting an agent opens a new tab (or focuses an existing tab connected to that agent).
- **Tab strip**: A horizontal tab bar displays all open sessions with agent name, connection status indicator, and close button.
- **Backward compatibility**: Single-tab usage is indistinguishable from the previous singleton model.

### FR-44.1: Tab Reactivity Fix — Sprint 35

The ACP-UI client SHALL fix the Vue reactivity bypass in `createSession()` and `resumeSession()` that prevents tab state transitions from propagating to the UI.

- **Root cause**: Both functions hold a reference to the raw (non-proxied) `TabState` object returned by `createEmptyTabState()`. After inserting it into the reactive `tabs` Map via `.set()`, subsequent mutations (`isConnected = true`, `isConnecting = false`) go through the raw reference, bypassing Vue's reactive proxy. The `tabList` computed and `TabStrip` component never recompute.
- **Symptom**: Tab strip shows "Connecting" indefinitely despite successful connection. Chat is functional because `activeTab` re-evaluates on re-render.
- **Fix**: After `tabs.value.set(tabId, tab)`, re-obtain the tab reference through the reactive proxy: `tab = tabs.value.get(tabId)!`. Apply to both `createSession()` and `resumeSession()` in `session.ts`.

### FR-45: Pipeline Engine — Sprint 35

The system SHALL introduce a Pipeline entity that orchestrates workflows across multiple ACP clients as a directed acyclic graph (DAG). Workflows remain the unit of work for a single ACP client. Pipelines coordinate work across clients with dependency tracking, derived execution state, and session pre-creation.

#### Conceptual Model

- **Workflow** = unit of work for one ACP client. Steps execute sequentially within a single session. One agent, one session, no cross-context leakage. Existing 69 JSON definitions unchanged.
- **Pipeline** = DAG orchestrator across multiple ACP clients. Each node references a workflow and an ACP client. Resumable, restartable, forkable. The pipeline owns session lifecycle; agents are executors handed sessions to work through.

#### FR-45.1: Pipeline Definition Format

The system SHALL support pipeline definitions as JSON with `nodes[]` and `edges[]` arrays.

- **Node schema**: Each node SHALL have `id` (unique string), `workflowId` (reference to an existing workflow JSON asset), and optional `acpClient` (which connected ACP client executes this node).
- **Edge schema**: Each edge SHALL have `source` (node ID), `target` (node ID), and `type` (`success` or `always` for v1).
- **Validation**: The system SHALL reject pipeline definitions containing cycles (topological sort validation at creation time).
- **Backward compatibility**: Existing serial workflow sessions (`queue: [A, B, C]`) SHALL continue to work unchanged. Pipelines are a new entity, not a modification of existing workflow behavior.

#### FR-45.2: Session Pre-Creation

When a pipeline is run, the system SHALL pre-create all workflow sessions upfront.

- **Pre-creation**: On "run," the system SHALL create one workflow session file per pipeline node. Each session contains full frontmatter (step definitions, `pipelineId`, `nodeId`, `workflowId`) and starts with `status: pending`.
- **Inspectability**: All sessions are visible and inspectable before any agent executes. The full execution plan is materialized as files on disk.
- **Predecessor context injection**: Before a downstream node transitions to `ready`, the system SHALL read the `last` frontmatter field from each upstream node's session file and inject the content into the downstream session file as a `## Predecessor Context` section. The executing agent reads this as part of the session content.

#### FR-45.3: Session Frontmatter — `first` and `last` Fields

All session types (workflow, sequential thinking, pipeline) SHALL track `first` and `last` fields in YAML frontmatter.

- **`first`**: Set once on the initial write to the session body. Never updated after creation. Captures the opening context.
- **`last`**: Updated on every append to the session body (step responses, thoughts, conclusions). Always reflects the most recent content written.
- **Scope**: Applies to all `append_h2_section` operations across session types. Both fields store the content as written, including `[[WikiLinks]]`.
- **Use case**: The pipeline engine reads `last` from upstream node sessions for predecessor context injection. Any tool needing "what happened most recently?" reads `last` instead of parsing the markdown body. The `RecentActivity` tool SHALL be updated to use `first`/`last` fields instead of parsing markdown body content.

#### FR-45.4: Workflow Session Status Lifecycle

The system SHALL extend workflow session frontmatter with the following status values:

- **`pending`** — session created, upstream dependencies not yet met.
- **`ready`** — all upstream dependencies met, waiting for dispatch to an ACP client.
- **`active`** — dispatched to an ACP client, currently executing.
- **`completed`** — all steps finished successfully.
- **`abandoned`** — was `active` but the agent disconnected, crashed, or timed out.

Existing session cleanup logic SHALL only clean up sessions in `active` state that have been abandoned (no heartbeat, agent disconnected). `pending` and `ready` sessions SHALL NOT be cleaned. `completed` sessions are permanent audit trail artifacts.

#### FR-45.5: Derived Execution State

Pipeline state SHALL be derived from session files, never stored separately.

- **Status derivation**: A node's status is read from its pre-created workflow session file's frontmatter `status` field.
- **Ready computation**: A node is `ready` when all upstream nodes (per `success` edges) have `status: completed` AND the node's own status is `pending`.
- **Blocked computation**: A node is `blocked` when at least one upstream node (per `success` or `always` edges) is not `completed`.
- **Pure function**: The ready set is computed by a Rust function reading the pipeline definition + session file frontmatter. No database tables, no virtual tables. Dropping and recomputing yields identical results.
- **Rebuildable**: If the pipeline session file is lost, it can be reconstructed from the node session files on disk (each carries `pipelineId` and `nodeId` in frontmatter).

#### FR-45.6: Pipeline MCP Tool

The system SHALL expose pipeline operations via a new `Pipeline` MCP tool, accessible by both the UI and AI agents through the same API surface.

- `pipeline(definition={nodes,edges})` — validate and create a pipeline. Pre-create all node sessions. Return pipeline session ID + initial ready set.
- `pipeline(sessionId=X, view=true)` — return current pipeline state as structured JSON (see response shape below) and a pre-rendered Mermaid diagram string. Suitable for UI rendering and chat display via FR-40.
- `pipeline(sessionId=X, dispatch=true)` — compute ready set, atomically transition ready nodes to `active`, and return them with session IDs. Dispatch IS the claim — no separate status update needed.
- `pipeline(sessionId=X, nodeId=Y, status="completed", conclusion="...")` — mark a node as complete, trigger ready-set recomputation and predecessor context injection for downstream nodes.
- `pipeline(sessionId=X, nodeId=Y, status="abandoned")` — mark a node as abandoned (agent failure).
- `pipeline(sessionId=X, nodeId=Y, restart=true)` — create a new session for an abandoned node, increment attempt count.
- **Single API surface**: The UI (via HTTP `/mcp`), AI agents (via stdio MCP), and the PM (via MCP) SHALL all use the same Pipeline tool. No direct file or database access.
- **Ownership boundary**: The Pipeline tool owns `pending → ready → active` transitions. The Workflow tool owns `active → completed/cancelled`. Neither crosses the boundary.

**`view=true` response shape:**
```json
{
  "pipelineId": "pipeline-1772717599558",
  "status": "active",
  "nodes": [
    {"nodeId": "swe", "workflowId": "tdd-implement", "sessionId": "workflow-1772717601234-swe", "status": "completed", "attempts": 1},
    {"nodeId": "red", "workflowId": "tdd-audit", "sessionId": "workflow-1772717605678-red", "status": "active", "attempts": 1},
    {"nodeId": "blue", "workflowId": "tdd-verify", "sessionId": "workflow-1772717605679-blue", "status": "pending", "attempts": 0}
  ],
  "edges": [
    {"source": "swe", "target": "red", "type": "success"},
    {"source": "red", "target": "blue", "type": "success"}
  ],
  "ready": [],
  "active": ["red"],
  "completed": ["swe"],
  "mermaid": "graph LR\n  swe[swe ✅] --> red[red 🔄]\n  red --> blue[blue ⏳]"
}
```

**`dispatch=true` response shape:**
```json
{
  "dispatched": [
    {"nodeId": "swe", "sessionId": "workflow-1772717601234-swe", "workflowId": "tdd-implement"}
  ],
  "active": [],
  "completed": [],
  "remaining": 2
}
```

**Pipeline session file format:**
```yaml
title: Pipeline Session pipeline-1772717599558
permalink: pipeline-1772717599558
type: pipeline
status: active
definition:
  nodes:
    - id: swe
      workflowId: tdd-implement
    - id: red
      workflowId: tdd-audit
    - id: blue
      workflowId: tdd-verify
  edges:
    - source: swe
      target: red
      type: success
    - source: red
      target: blue
      type: success
nodeStates:
  swe:
    sessionId: workflow-1772717601234-swe
    status: completed
    attempts: 1
  red:
    sessionId: workflow-1772717605678-red
    status: active
    attempts: 1
  blue:
    sessionId: workflow-1772717605679-blue
    status: pending
    attempts: 0
maxConcurrency: 8
first: "Pipeline created with 3 nodes: swe, red, blue"
last: "Node swe completed"
```

**Worked example — TDD pipeline:**
1. PM: `pipeline(definition={nodes:[swe,red,blue], edges:[swe→red, red→blue]})` → creates 3 session files, returns `pipeline-17727...` + ready set `["swe"]`
2. PM: `pipeline(sessionId=X, dispatch=true)` → atomically claims `swe` as active, returns session ID
3. PM hands SWE agent the session ID → agent calls `workflow(sessionId=swe-session, response=...)` for each step
4. Agent completes → `workflow(sessionId=swe-session, status="completed", conclusion="...")`
5. PM: `pipeline(sessionId=X, dispatch=true)` → swe's `last` injected into red's `## Predecessor Context`, red claimed as active
6. Repeat for red → blue → pipeline complete

#### FR-45.7: Pipeline Dispatch Model

The v1 dispatch model SHALL be pull-based (cooperative scheduling).

- **Pull-based**: A supervisor (PM agent, UI user, or automation) calls `pipeline(dispatch=true)` to get the ready set, then manually hands session IDs + prompts to ACP clients.
- **No auto-dispatch**: The pipeline engine does NOT push work to ACP clients automatically in v1. There is no ACP client module in the codebase. Auto-dispatch is a future enhancement.
- **Dispatch-as-claim**: `dispatch=true` atomically transitions ready nodes to `active` and returns them. There is no separate "mark as active" step. This prevents double-dispatch when multiple callers poll simultaneously.
- **Concurrency**: An optional `maxConcurrency` field on the pipeline definition limits how many nodes can be `active` simultaneously. The `dispatch` response respects this limit.
- **`always` edge semantics**: An `always` edge allows the downstream node to become ready when the upstream node is in `completed` OR `abandoned` state. This enables fault-tolerant pipelines where some nodes are optional.

#### FR-45.8: Pipeline Resume and Restart

Pipelines SHALL support lifecycle operations beyond initial execution.

- **Resume**: A pipeline with `active` or `ready` nodes can be resumed after interruption. The engine reads session files, derives current state, and returns the ready set. No state is lost because session files persist.
- **Restart**: An `abandoned` node can be retried by creating a new workflow session for the same node (new session file, same workflow definition, same predecessor context). The prior session file remains as audit trail. Attempt count is tracked in the pipeline session frontmatter.
- **Max attempts**: Nodes SHALL have a configurable `maxAttempts` (default 3). After reaching max attempts, the node transitions to a `stuck` status and blocks downstream nodes.
- **Immutable topology**: Pipeline topology is immutable after creation in v1. No appending nodes or rewiring edges at runtime. To change topology, create a new pipeline.
- **Status gating**: The Workflow tool SHALL gate `continue_workflow_session` on the session's `status` field. Sessions in `pending` state SHALL reject continuation attempts. Sessions in `completed` state SHALL reject re-opening.

#### FR-45.9: Pipeline Fork (v2 — deferred)

_Deferred to Sprint 36._ Fork creates a new session for a completed or abandoned node targeting a different ACP client, enabling A/B comparisons across agents. Forks are observational — they do not affect canonical downstream scheduling unless explicitly promoted.

#### FR-45.10: Verdict-Based Edge Routing (v2 — deferred)

_Deferred to Sprint 36._ The v1 engine supports `success` and `always` edge types only. Future enhancements:

- `reject` edges that route based on a structured `verdict` field in the node's workflow conclusion.
- `optional` edges where upstream failure does not block downstream.
- Conditional edges based on conclusion content.

#### FR-45.11: Vue Flow DAG Tab (v2 — deferred)

_Deferred to Sprint 36._ The v1 engine is fully functional via MCP tool calls. Visual rendering in a Vue Flow tab requires:

- Vue Flow dependency integration in acp-ui.
- New tab type alongside chat tabs.
- Real-time status notification channel (does not exist yet).
- Node dispatch interaction UX.

For Sprint 35, pipeline state can be inspected via `pipeline(view=true)` and optionally rendered as Mermaid diagrams using existing FR-40 support.

### NFR-9: Non-Text Content Blocks

The system SHOULD log warnings when receiving non-text content blocks instead of silently dropping them. Full rendering of image/audio/resource content blocks is deferred to a future sprint.
