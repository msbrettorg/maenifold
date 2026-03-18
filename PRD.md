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

### ~~FR-44.1: Tab Reactivity Fix~~ (DEPRECATED — superseded by FR-44.12)

Interim fix using `shallowRef + triggerRef`. Addressed immediate tab strip failures but created a maintenance burden: 17+ mutation sites require manual `triggerRef` calls. Superseded by FR-44.12's architectural approach.

### FR-44.12: Multi-Tab State Architecture — Sprint 35

The ACP-UI client SHALL restructure multi-tab state management so that per-tab mutations are naturally reactive without manual `triggerRef` calls.

- **Problem**: Sprint 34 wrapped the OG single-session state in `shallowRef<Map<string, TabState>>`. This requires manual `triggerRef(tabs)` at every mutation site. Research (T-44.5b, T-44.10) identified 17+ gaps causing stale permission dialogs, broken auto-scroll during streaming, and component state bleed across tabs.
- **Solution**: Per-tab reactive state (each tab independently reactive), keyed `<ChatView :key="activeTabId">`, per-tab UI state in store, deep scroll watch on active tab's messages, and content-version-based unseen output detection.
- **Success criteria**: Zero `triggerRef` calls remain in `session.ts` after restructuring.

### FR-45: Pipeline Engine — Sprint 35

The system SHALL introduce a Pipeline entity that orchestrates workflows across multiple ACP clients as a directed acyclic graph (DAG). Workflows remain the unit of work for a single ACP client. Pipelines coordinate work across clients with dependency tracking, derived execution state, and session pre-creation.

#### Conceptual Model

- **Workflow** = unit of work for one ACP client. Steps execute sequentially within a single session. One agent, one session, no cross-context leakage. Existing 69 JSON definitions unchanged.
- **Pipeline** = DAG orchestrator across multiple ACP clients. Each node references a workflow and an ACP client. Resumable, restartable, forkable. The pipeline owns session lifecycle; agents are executors handed sessions to work through.

#### FR-45.1: Pipeline Definition Format

The system SHALL support pipeline definitions as JSON with `nodes[]` and `edges[]` arrays.

- **Node schema**: Pipeline nodes SHALL support two types via a tagged union. Each node SHALL have `nodeId` (unique string, derived as `{workflowId}-{N}` or `{stepId}-{N}`), `type` (`"step"` or `"workflow"`), `acpClient` (name of a registered ACP agent from `agents.json`), and `label` (friendly display name, derived from the workflow/step name). **Step nodes** SHALL have a `step` field with the same schema as a workflow step: `{id, name, description, metadata?, requiresSequentialThinking?}`. **Workflow nodes** SHALL have a `workflowId` field referencing `assets/workflows/{id}.json`. Both types compile to the same runtime representation: an ordered list of steps (1 for step nodes, N for workflow nodes).
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
- **`always` edge semantics**: An `always` edge is satisfied when the upstream node is `completed`, `abandoned`, or `stuck`.
- **Blocked computation**: A node is `blocked` when at least one upstream node on a `success` edge is `abandoned` or `stuck`. Nodes with unsatisfied but not terminal dependencies remain `pending`.
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
  "definitionId": "tdd-review-loop",
  "status": "active",
  "createdAt": "2026-03-18T12:00:00Z",
  "nodes": [
    {"nodeId": "tdd-implement-1", "workflowId": "tdd-implement", "sessionId": "workflow-1772717601234-tdd-implement-1", "status": "completed", "attempts": 1, "startedAt": "2026-03-18T12:05:00Z", "completedAt": "2026-03-18T12:45:00Z", "agentId": "OpenCode"},
    {"nodeId": "tdd-audit-1", "workflowId": "tdd-audit", "sessionId": "workflow-1772717605678-tdd-audit-1", "status": "active", "attempts": 1, "startedAt": "2026-03-18T12:46:00Z", "agentId": "Copilot"},
    {"nodeId": "tdd-verify-1", "workflowId": "tdd-verify", "sessionId": "workflow-1772717605679-tdd-verify-1", "status": "pending", "attempts": 0}
  ],
  "edges": [
    {"source": "tdd-implement-1", "target": "tdd-audit-1", "type": "success"},
    {"source": "tdd-audit-1", "target": "tdd-verify-1", "type": "success"}
  ],
  "ready": [],
  "active": ["tdd-audit-1"],
  "completed": ["tdd-implement-1"],
  "mermaid": "graph LR\n  tdd-implement-1[TDD Implementation ✅] --> tdd-audit-1[TDD Audit 🔄]\n  tdd-audit-1 --> tdd-verify-1[TDD Verification ⏳]"
}
```

**`dispatch=true` response shape:**
```json
{
  "dispatched": [
    {"nodeId": "tdd-implement-1", "sessionId": "workflow-1772717601234-tdd-implement-1", "workflowId": "tdd-implement", "acpClient": "OpenCode"}
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
definitionId: tdd-review-loop
definitionVersion: "1.0.0"
createdAt: "2026-03-18T12:00:00Z"
definition:
  nodes:
    - nodeId: tdd-implement-1
      workflowId: tdd-implement
      acpClient: OpenCode
      label: TDD Implementation
    - nodeId: tdd-audit-1
      workflowId: tdd-audit
      acpClient: Copilot
      label: TDD Audit
    - nodeId: tdd-verify-1
      workflowId: tdd-verify
      acpClient: OpenCode
      label: TDD Verification
  edges:
    - source: tdd-implement-1
      target: tdd-audit-1
      type: success
    - source: tdd-audit-1
      target: tdd-verify-1
      type: success
nodeStates:
  tdd-implement-1:
    sessionId: workflow-1772717601234-tdd-implement-1
    status: completed
    attempts: 1
    startedAt: "2026-03-18T12:05:00Z"
    completedAt: "2026-03-18T12:45:00Z"
    agentId: OpenCode
    predecessorContextInjected: false
    submachines: []
  tdd-audit-1:
    sessionId: workflow-1772717605678-tdd-audit-1
    status: active
    attempts: 1
    startedAt: "2026-03-18T12:46:00Z"
    agentId: Copilot
    predecessorContextInjected: true
    submachines: []
  tdd-verify-1:
    sessionId: workflow-1772717605679-tdd-verify-1
    status: pending
    attempts: 0
    predecessorContextInjected: false
    submachines: []
maxConcurrency: 3
first: "Pipeline created with 3 nodes: tdd-implement-1, tdd-audit-1, tdd-verify-1"
last: "Node tdd-implement-1 completed"
```

**Worked example — TDD pipeline:**
1. PM: `pipeline(definitionId="tdd-review-loop")` → loads template from `assets/pipelines/tdd-review-loop.json`, creates 3 session files, returns `pipeline-17727...` + ready set `["tdd-implement-1"]`
2. PM: `pipeline(sessionId=X, dispatch=true)` → atomically claims `tdd-implement-1` as active, returns session ID + acpClient `"OpenCode"`
3. PM hands OpenCode agent the session ID → agent calls `workflow(sessionId=tdd-implement-1-session, response=...)` for each step
4. Agent completes → `workflow(sessionId=tdd-implement-1-session, status="completed", conclusion="...")`
5. PM: `pipeline(sessionId=X, dispatch=true)` → tdd-implement-1's `last` injected into tdd-audit-1's `## Predecessor Context`, tdd-audit-1 claimed as active via Copilot
6. Repeat for tdd-audit-1 → tdd-verify-1 → pipeline complete

#### FR-45.7: Pipeline Dispatch Model

The v1 dispatch model SHALL be pull-based (cooperative scheduling).

- **Pull-based**: A supervisor (PM agent, UI user, or automation) calls `pipeline(dispatch=true)` to get the ready set, then manually hands session IDs + prompts to ACP clients.
- **No auto-dispatch**: The pipeline engine does NOT push work to ACP clients automatically in v1. There is no ACP client module in the codebase. Auto-dispatch is a future enhancement.
- **Dispatch-as-claim**: `dispatch=true` atomically transitions ready nodes to `active` and returns them. There is no separate "mark as active" step. This prevents double-dispatch when multiple callers poll simultaneously.
- **Concurrency**: An optional `maxConcurrency` field on the pipeline definition limits how many nodes can be `active` simultaneously. The `dispatch` response respects this limit.
- **`always` edge semantics**: An `always` edge allows the downstream node to become ready when the upstream node is in `completed`, `abandoned`, or `stuck` state. This enables fault-tolerant pipelines where some nodes are optional and terminal upstream failures should not strand downstream cleanup or follow-up work.

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

#### FR-45.11: Pipeline Designer UI — Sprint 37

The system SHALL provide a Vue Flow-based pipeline designer that replaces the workflow tree viewer.

- **Vue Flow integration**: The acp-ui SHALL integrate `@vue-flow/core` for interactive DAG rendering with drag-and-drop node placement.
- **Replaces workflow tree**: The 🌳 Workflows sidebar toggle SHALL be replaced by a pipeline designer view.
- **Create pipelines**: Users SHALL be able to create new pipeline definitions by adding nodes (selecting a workflow + ACP client), drawing edges between them, and saving to `assets/pipelines/`.
- **Edit pipelines**: Users SHALL be able to load, modify, and re-save existing pipeline definitions.
- **Node creation UX**: Adding a node SHALL present a workflow picker (from `assets/workflows/`) and an ACP client picker (from `agents.json`). The `nodeId` and `label` SHALL be auto-derived from the selected workflow.
- **Edge creation UX**: Users SHALL draw edges between nodes and select the edge type (`success` or `always`).
- **Node positions**: The designer SHALL persist node positions in the pipeline definition's `ui.nodes` field.
- **Execution history**: The designer SHALL display past pipeline execution sessions with status, timing, and node state detail.

#### FR-45.12: Pipeline Definition Storage — Sprint 37

The system SHALL support persistent, reusable pipeline templates stored as JSON assets.

- **Storage location**: Pipeline definitions SHALL be stored as JSON files in `assets/pipelines/`, mirroring the existing `assets/workflows/` pattern.
- **Asset kind**: A new `AssetKind::Pipeline` SHALL be added to the asset system, enabling discovery via `ListAssets(type="pipeline")` and `ReadMcpResource("asset://pipeline/{id}")`.
- **Naming convention**: Pipeline definition IDs SHALL use kebab-case slugs. The filename SHALL be `{id}.json`.
- **Schema**: Each pipeline definition SHALL contain `id`, `name`, `nodes[]`, `edges[]`, and optional fields: `description`, `version` (semver string), `maxConcurrency`, and `ui` (node positions).
- **Node schema**: See FR-45.1 for the updated node schema. Nodes are a tagged union: `type: "step"` (inline step definition) or `type: "workflow"` (references a workflow asset).
- **Template vs instance**: Pipeline definitions (templates) are immutable design-time assets. Pipeline sessions (instances) are mutable runtime state. The definition SHALL be snapshotted into the session frontmatter at creation time.
- **`definitionId` parameter**: The Pipeline MCP tool SHALL accept an optional `definitionId` parameter to load a saved template instead of requiring an inline `definition`. Both modes SHALL remain supported.
- **Workflow reference validation**: At creation time, the system SHALL validate that all `workflowId` references resolve to existing workflow assets.

**Pipeline definition example:**
```json
{
  "id": "tdd-review-loop",
  "name": "TDD Review Loop",
  "description": "Clarify scope → implement → audit → verify",
  "version": "1.0.0",
  "maxConcurrency": 3,
  "nodes": [
    {
      "nodeId": "clarify-scope-1",
      "type": "step",
      "acpClient": "Copilot",
      "label": "Clarify Scope",
      "step": {
        "id": "clarify-scope",
        "name": "Clarify Scope",
        "description": "Restate the task, identify constraints, produce a concise implementation brief.",
        "requiresSequentialThinking": false
      }
    },
    { "nodeId": "tdd-implement-1", "type": "workflow", "workflowId": "tdd-implement", "acpClient": "OpenCode", "label": "TDD Implementation" },
    { "nodeId": "tdd-audit-1", "type": "workflow", "workflowId": "tdd-audit", "acpClient": "Copilot", "label": "TDD Audit" },
    { "nodeId": "tdd-verify-1", "type": "workflow", "workflowId": "tdd-verify", "acpClient": "OpenCode", "label": "TDD Verification" }
  ],
  "edges": [
    { "source": "clarify-scope-1", "target": "tdd-implement-1", "type": "success" },
    { "source": "tdd-implement-1", "target": "tdd-audit-1", "type": "success" },
    { "source": "tdd-audit-1", "target": "tdd-verify-1", "type": "always" }
  ],
  "ui": {
    "nodes": {
      "clarify-scope-1": { "x": 0, "y": 0 },
      "tdd-implement-1": { "x": 300, "y": 0 },
      "tdd-audit-1": { "x": 600, "y": 0 },
      "tdd-verify-1": { "x": 900, "y": 0 }
    }
  }
}
```

#### FR-45.13: Pipeline Execution Tracking — Sprint 37

The system SHALL extend pipeline session frontmatter with richer execution metadata.

- **Definition provenance**: Pipeline sessions SHALL record `definitionId` and `definitionVersion` when created from a saved template.
- **Pipeline-level timing**: Sessions SHALL track `createdAt` and `completedAt` timestamps.
- **Per-node timing**: `NodeState` SHALL be extended with `startedAt` (set on dispatch), `completedAt` (set on completion/abandonment), and `agentId` (copied from the node's `acpClient` at dispatch time).
- **Predecessor context tracking**: `NodeState` SHALL include a `predecessorContextInjected` boolean indicating whether upstream context was injected into this node's session.
- **Submachine tracking**: `NodeState` SHALL include a `submachines` array of `SubmachineRecord` entries, each containing `sessionId`, `sessionType`, `stepIndex` (which workflow step spawned it), `stepName`, `startedAt`, `completedAt`, and `status`.
- **Submachine write-through**: The existing `register_workflow_submachine` and `maybe_resume_workflow_submachine` functions SHALL propagate submachine lifecycle events to the parent pipeline session's `nodeStates` (best-effort, non-blocking).

#### FR-45.14: Pipeline REST API — Sprint 37

The system SHALL expose REST endpoints for pipeline definition CRUD and execution history.

- `GET /api/pipelines/definitions` — list all pipeline definitions (summary: id, name, description, nodeCount, edgeCount).
- `GET /api/pipelines/definitions/:id` — read a full pipeline definition.
- `POST /api/pipelines/definitions` — create a new pipeline definition.
- `PUT /api/pipelines/definitions/:id` — update an existing pipeline definition.
- `DELETE /api/pipelines/definitions/:id` — delete a pipeline definition.
- `GET /api/pipelines/sessions` — list pipeline execution sessions (filterable by status, definitionId).
- `GET /api/pipelines/sessions/:id` — read full pipeline session detail (reuses `view_pipeline_session` logic, includes mermaid diagram).
- `POST /api/pipelines/sessions/:id/run` — start active execution of a pipeline session. Returns 202 with `runId`.
- `GET /api/pipelines/sessions/:id/events` — cursor-tracked SSE stream of pipeline execution events. Supports `Last-Event-ID` header for reconnection without data loss. Falls back to polling via `?since={cursor}` query parameter.

#### FR-45.15: Pipeline Active Executor — Sprint 37

The pipeline engine SHALL support active, autonomous execution of pipelines without requiring an external PM agent or manual dispatch loop. When a user clicks "Run Pipeline" in the UI, the engine SHALL execute the full DAG end-to-end.

- **`PipelineEngine`**: A new async executor component SHALL be added to the axum server. It SHALL be constructed once at server startup and shared via `ApiState`.
- **`AgentProcess` extraction**: The existing `RelayProcess` in `acp/relay.rs` SHALL be refactored into a shared `acp/process.rs` module (`AgentProcess`). Both the WebSocket relay handler and the pipeline executor SHALL use this shared process abstraction. Zero behavior change to the existing relay.
- **Execution trigger**: `POST /api/pipelines/sessions/:id/run` SHALL spawn a detached `tokio::task` that runs the execution loop. The HTTP handler SHALL return 202 immediately.
- **Execution loop**: The executor SHALL:
  1. Load pipeline state from the session file
  2. Compute ready nodes via `compute_ready_set()` (existing pure function)
  3. Claim ready nodes up to `maxConcurrency` (using a `tokio::Semaphore`)
  4. Spawn a `tokio::task` per claimed node
  5. Each node task spawns an `AgentProcess` for the target `acpClient`, sends step prompts, collects responses
  6. On completion, persist node state and notify the main loop via `mpsc::channel`
  7. Recompute ready set and repeat until pipeline is terminal
- **Dual execution mode**: Nodes with `acpClient` set SHALL execute via `AgentProcess` (external ACP agent). Nodes with `acpClient` unset SHALL execute in-process by calling workflow tool handlers directly.
- **Cursor-tracked event streaming**: Pipeline execution events SHALL be persisted with monotonic sequence numbers. The `/events` SSE endpoint SHALL support `Last-Event-ID` header for reconnection without data loss. A polling fallback SHALL be available via `GET /api/pipelines/sessions/:id/events?since={cursor}` for clients that cannot use SSE. This follows the AgentBoss pattern: events are cursor-tracked, so disconnects never lose data.
- **Event types**: `NodeStarted`, `NodeCompleted`, `NodeFailed`, `NodeOutput` (streaming agent text), `PipelineCompleted`, `PipelineStuck`.
- **Long-running support**: Pipelines may run for hours or days. The executor SHALL survive server restarts by reading persisted state from session files. On restart, incomplete pipelines can be resumed via `POST /run` which re-enters the execution loop from current state.
- **Backward compatibility**: The existing `Pipeline` MCP tool (`dispatch=true` / manual status updates) SHALL continue to work unchanged for AI-agent-driven pipelines. The active executor is an additional execution path that reuses the same underlying state functions.

### NFR-9: Non-Text Content Blocks

The system SHOULD log warnings when receiving non-text content blocks instead of silently dropping them. Full rendering of image/audio/resource content blocks is deferred to a future sprint.
