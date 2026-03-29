# Changelog — maenifold-rs

All notable completed work is documented here, organized by sprint.
For detailed retrospectives including lessons learned, see [RETROSPECTIVES.md](RETROSPECTIVES.md).

---

## Sprint 69 — Pipeline Node Configuration Parity (FR-52)

Closed the gap between pipeline node config fields accepted at creation time and what is actually wired, visible, and discoverable at runtime.

### Fixed
- **View config fields** (T-69.2): Pipeline `view` operation now serializes all 7 config fields (`model`, `thoughtLevel`, `allowedTools`, `maxTurns`, `timeout`, `toolMode`, `agentConfig`) for each node type that supports them.
- **Graft guard** (T-69.4): Graft and delete guards now check actual node statuses (`Pending`/`Active`/`Ready`) via `has_remaining_work()` instead of stale frontmatter status string. Completed/abandoned pipelines can now be grafted onto or deleted.
- **Stuck transition** (T-69.5): Restart rejection due to max-attempts now transitions node to `Stuck` status (persisted before error return) with clear error message.

### Added
- **Schema config fields** (T-69.3): MCP Pipeline tool schema uses `oneOf` discriminator per node type, advertising all config fields with correct required arrays. LLMs can now discover `model`, `thoughtLevel`, `allowedTools`, `maxTurns`, `timeout`, `toolMode`, and `agentConfig` via the schema.
- **Wire agent mode** (T-69.6): `AgentConfig.mode` flows end-to-end from pipeline node → `SessionOpts` → ACP `session/set_mode`. Warning log removed.
- **Wire config options** (T-69.7): `AgentConfig.config_options` flows to ACP `session/set_config_option` calls. Built-in keys (`thought_level`, `allowed_tools`, `max_turns`) filtered to avoid double-setting. Warning log removed.
- **ListAgents MCP tool** (T-69.8): New tool enumerates ACP agents from `agents.json` with name, command, args, and mcp_servers. Registered as `ListAgents`/`listagents`/`list_agents`.

### Changed
- **Module extractions**: Tests extracted from `mcp/mod.rs` to `mcp/tests.rs`. Workflow helpers extracted from `runner.rs` to `runner_helpers.rs`. All files under 300 LOC.

---

## Sprint 68 — ACP Client Hardening Phase 2 (FR-50, FR-51)

Continued native ACP client hardening with client capabilities, MCP server passthrough, session persistence, pipeline capability-aware configuration, and usage tracking.

### Added
- **Client capabilities** (T-68.2): `spawn_agent` advertises `ClientCapabilities` with `fs.read=true`, `fs.write=true`, `terminal=false`. `HeadlessClient` implements `read_text_file`/`write_text_file` with CWD jail (`resolve_and_jail` — parent-dir traversal guard + canonical path containment).
- **MCP server passthrough** (T-68.3): `McpServerEntry` enum (Stdio/Http) on `AgentEntry`. `SessionOpts` carries `mcp_servers` through to `NewSessionRequest`. Config loaded from `agents.json`.
- **Session list and load** (T-68.4): `ListSessions`/`LoadSession` `AgentCommand` variants with capability guards (`session_capabilities.list` and `load_session`). `AcpxSessionInfo` maps from ACP `SessionInfo`.
- **Pipeline capability-aware config** (T-68.5): `AgentConfig` struct on `Prompt`/`SequentialThinking`/`Tool` pipeline node variants. Flat node fields (model, thought_level) take precedence via `.or_else()`. Forward-compat warnings for `mode` and `config_options`.
- **ACP feature flags** (T-68.6): Enabled `unstable_session_usage` feature. `UsageUpdate` handled in `session_notification`. `AcpxUsage` type surfaces usage in `AgentResponse`/`AcpxResponse`.

### Changed
- **Module extraction**: `SessionAccumulator` and helpers extracted from `client.rs` to `accumulator.rs`. Classify/run functions extracted from `executor_nodes.rs` to `executor_run.rs`. All files under 300 LOC.

---

## Sprint 67 — Native ACP Client Hardening (FR-50)

Hardened the native ACP client with init capture, session config passthrough, full session-update handling, crash recovery, and prompt timeout/cancel support.

### Added
- **Init capture and capabilities** (T-67.1): `InitInfo` struct captures `InitializeResponse` fields. `AgentHandle` exposes `agent_info()`, `capabilities()`, `auth_methods()` accessors. Protocol version check with mismatch warning.
- **Session config passthrough** (T-67.2): `SessionOpts` carries model, thought_level, allowed_tools, max_turns. Pipeline nodes thread config through `open_agent_session()`. CLI `prompt` command accepts `--model` flag.
- **Session update handling** (T-67.3): `SessionAccumulator` handles all 11 `SessionUpdate` variants. `AgentResponse` now includes `tool_calls` and `thought` fields. `ToolCallRecord` captures tool call metadata.
- **Agent crash recovery** (T-67.4): `AgentHandle::is_alive()` via `AtomicBool` alive flag. `tokio::select!` on io_task detects agent death. `AgentPool::get_or_create()` evicts dead handles and respawns. Agent stderr piped to `tracing::debug!`. Child process reaped via `start_kill()` + `child.wait()`.
- **Prompt timeout and cancel** (T-67.5): `SessionOpts::timeout_seconds` with 300s default. `run_with_timeout` wraps prompt calls. On timeout: `CancelNotification` → `close_session` → error. Pipeline nodes carry optional `timeout` field. `AcpxDriver` timeout parameter wired through.

### Changed
- **Module structure**: `acp/native.rs` split into `acp/native/` directory with 5 files: `mod.rs` (248 LOC), `types.rs` (166), `session.rs` (202), `client.rs` (235), `pool.rs` (170). All under 300 LOC limit.

---

## Sprint 63 — Silent Failure Remediation

Remediated 5 instances of the same anti-pattern from the legacy C# codebase: errors that hide from users via stderr logging, `.expect()` panics, `.unwrap_or(false)`, or missing cleanup paths.

### Added
- **Watcher health counters** (T-63.1): `WatcherHealth` struct with `SYNC_FAILURES`, `CONSECUTIVE_FAILURES`, `LAST_FAILURE_MSG` statics. `watcher_health()` accessor wired into `render_system_health()` for MemoryStatus visibility.
- **Frontmatter warning accumulator** (T-63.3): `FRONTMATTER_WARNINGS` mutex with `drain_frontmatter_warnings()` exposing corrupt files. Capped at 100 entries, drain semantics. Wired into `render_system_health()`.
- **Orphan vector cleanup in sync path** (T-63.5): `cleanup_orphan_vec_memory_files()` now runs in both watcher batch and full sync paths with `sqlite_master` table existence guard.

### Changed
- **Regex static compilation** (T-63.2): 3 `once_cell::Lazy<Regex>` statics replace 4 `.expect()` calls in `workflow/runner.rs`. Compiled once at module load.
- **Tag filtering error propagation** (T-63.4): 4 `.unwrap_or(false)` sites replaced with `match` arms that log `[TAG FILTER]` errors with file URI.

---

## Sprint 62 — Gate Generalization

Generalized `maenifold-gate` to support arbitrary development pipelines (SDD, TDD, GSD) instead of hardcoded SWE→Red→Blue stages.

### Added
- **Pipeline definitions**: Correct `sdd.json` (default), `tdd.json` (Red/Green/Refactor), `gsd.json` (Get Shit Done), `lightweight.json`
- **Pipeline-aware parsing**: `parse_task_content_with_pipeline()` derives status from pipeline transitions
- **Named pipeline selection**: `--pipeline tdd` on all commands loads embedded pipelines by name
- **SDD fallback warning**: `gate status` emits `note:` when no `pipeline.json` found
- **Dynamic JSON stages**: `StatusOutput` emits `stages: Vec<StageOutput>` instead of hardcoded `swe`/`red`/`blue` fields
- **pulldown-cmark parser**: Rewrote markdown parser to use AST event stream instead of `starts_with()` matching
- 95 tests (up from ~65)

### Changed
- `TaskStage`/`TaskStatus` enums removed — stages and statuses are strings everywhere
- DB schema relaxed: CHECK constraints removed from `status`/`stage` columns
- Skill renamed from `test-driven-development` to `gate` (pipeline-agnostic)
- `normalize_stage_name()` uses generic first-word extraction, no SDD shortcuts
- No-pipeline public API (`parse_task_content`, `parse_task_file`, `done_override_from_content`) deprecated
- `lib.rs` split into `cli.rs` + `cli/util.rs`; `db.rs` split into `db/mod.rs` + `db/migration.rs`

### Fixed
- `tdd.json` was mislabeled (was actually SDD) — now correctly named `sdd.json`
- `--pipeline` flag wired into all 4 commands (pass, reject, status, sync)
- Dead `OutOfOrderHeading` error variant removed
- `normalize_stage_name` prefix matching tightened to word boundaries

---

## Sprint 58— Pipeline Rewind Implementation

- **FR-40 completion**: Implemented `rewind_pipeline` in handler.rs — replaces stub with working subgraph reset
  - Computes downstream transitive closure via BFS (`reachable_node_ids`)
  - Allocates fresh workflow session IDs for rewound nodes
  - Atomic persistence ordering (workflow sessions before pipeline frontmatter)
  - Guards against rewinding running pipelines
- **RTM fixes**: Updated RTM-160 (rewind test references) and RTM-163 (join uses collision-rejection, not ID remapping)
- **FR-40 fully closed**: All lifecycle controls (pause/resume/cancel/rewind) and graph surgery (graft/join/fork) implemented and tested

## Sprint 57 — Native Workflow & Think Runners (maenifold-acp Rust integration)

### Added
- `runner.rs`: New `run_workflow()` function that starts a workflow session, creates an acpx agent session, and loops through steps — prompting the agent at each step and advancing the workflow
  - Handles terminal statuses (✅/❌), WikiLink injection via `tag_response()`, and step progress parsing
- `runner.rs`: New `run_think()` / `seed_think_session()` / `drive_think_session()` functions that drive SequentialThinking sessions through an agent
  - Supports parent workflow linkage and overflow protection
- `cli.rs`: Added `maenifold workflow <id> <agent> [prompt]` and `maenifold think <agent> <topic>` subcommands for direct invocation
- 21 scoped tests (10 runner + 5 executor + 6 CLI contract)

### Changed
- Native Rust orchestration now replaces the bash `maenifold-acp` script's workflow/think runner logic
  - The pipeline executor actively drives workflow steps instead of passively waiting for agents to self-orchestrate
- `runner.rs`: Workflow steps with `requiresSequentialThinking: true` now automatically spawn thinking submachines
  - Uses correct pre-registration ordering (seed → register → drive) for crash recovery durability
- `executor.rs`: Pipeline executor now calls `run_workflow()` instead of raw `AcpxDriver::prompt()`
  - Nodes without `acp_client` are abandoned immediately
  - Removed `build_driver()` and direct acpx imports

---

## Sprint 56 — FR-40: Async Pipeline Executor (replaces bash bridge)

### Added
- `executor.rs`: Rust-native async pipeline executor using `tokio::task::JoinSet` + `tokio::sync::Semaphore`
  - Dispatch→AcpxDriver::prompt→update_pipeline_node_status loop
  - `CancellationToken` for clean cancel with active node abandonment
  - `AtomicBool` pause flag checked between dispatch cycles
  - 5 scoped unit tests (single node, failure, cancel, concurrency, DAG cascade)

### Changed
- `supervisor.rs`: Stripped process/signal/thread management (~500 LOC removed); replaced with lightweight tokio task tracking (`CancellationToken` + `JoinHandle`)
  - Preserved: orphan detection, mark-stuck, pipeline_has_non_terminal_nodes
  - 9 scoped unit tests
- `handler.rs`: Lifecycle commands (run/pause/resume/cancel) rewired from bash subprocess to executor
  - No PID in run_pipeline response; no SIGSTOP/SIGCONT platform guards
- Gate CLI docs updated: `gate advance` references replaced with `gate pass`/`reject`/`append` in TDD skill and PM agent doc

### Removed
- `maenifold-acp` pipeline subcommand and all pipeline-only bash helpers
  - `run_pipeline`, `pipeline_worker`, `pipeline_launch_node`, `pipeline_wait_for_any`, etc.
  - Preserved: `workflow` and `think` subcommands, all shared helpers

---

## Sprint 55 — FR-42: Configurable Pipeline Engine

### Added
- JSON-driven pipeline definitions (`pipeline.rs`): configurable stages, verdicts, transitions
  - Ships `assets/pipelines/tdd.json` (swe→red→blue) and `lightweight.json` (swe→blue)
  - DAG validation with cycle detection and reachability checks
- `gate pass <ID>` — advances task per pipeline JSON, moves to done/ on terminal pass
- `gate reject <ID>` — rolls back task per pipeline JSON, increments attempts
- `gate append <ID> --agent <role> --notes "..."` — logs agent work without state change
- Task ID column in DB: `T-55.1` resolves to `T-55.1-pipeline-definition-schema.md`
- ID-based `gate status <ID>` lookup
- 10 pipeline integration tests, 84 total tests (up from 45)

### Changed
- CLI is now ID-based: `gate pass T-55.1` instead of `gate advance T-55.1-file.md --stage red --verdict PASS`
- Shared helpers extracted: `load_pipeline_def`, `resolve_file_path`, `append_orchestrator_section`
- `extract_task_id` handles trailing alpha suffixes (T-44.5b)

### Removed
- `gate advance` command and `advance.rs` module
- `--stage`, `--model`, `--verdict` CLI flags
- `advance_tests.rs`

---

## Sprint 54 — Parser Hardening (FR-41)

### Added
- Rework-cycle tolerance: parser handles repeated SWE/Red/Blue sections with last-section-wins semantics
- Implicit verdict support: intermediate sections without verdict = implicit REJECT; final done/ section = implicit PASS
- Emoji verdict parsing: `❌ REJECT` and similar emoji-prefixed verdicts extracted correctly
- No-SWE tolerance: done/ files with only Red/Blue sections (no SWE) accepted as valid
- Location-authoritative fallback: done/ files always report as Done regardless of parse completeness
- Incomplete audit trail note in status output for done/ files with partial stage data
- Case-insensitive CLI verdict parsing: `pass`, `PASS`, `Pass` all accepted
- 20 new tests across parser_rework_tests.rs and integration.rs (45 total, up from 25)

### Changed
- `parser.rs`: prefix heading matching (`starts_with`) replaces exact string match
- `sync.rs`: metadata warnings suppressed for done/ files (historical files lack `**Requirement**`)
- Removed 3 dead error variants: NonCanonicalHeading, DuplicateHeading, MissingVerdict

---

## Sprint 53 — FR-41: File-Authoritative Pipeline Gate Cache — 2026-03-21

### Added
- `gate` CLI tool (`crates/maenifold-gate`) — standalone binary for pipeline task management
  - `gate status` — report pipeline state from authoritative task files
  - `gate advance` — enforce stage transitions (SWE → Red → Blue → Done)
  - `gate sync` — rebuild/reconcile `gate.db` cache from disk
- File-authoritative design: `todo/` and `done/` markdown files are the source of truth
- `gate.db` is now a rebuildable cache (not the authority)
- 25 tests (15 unit + 4 advance + 6 integration)

### Changed
- TDD skill (`SKILL.md`) rewritten to use `gate` CLI instead of raw `sqlite3` commands
- PM agent config updated to reference gate CLI
- Scaffold updated: sample files use `T-*.md` naming, `gate.db` removed (auto-generated)
- `setup-template.md` updated for file-authoritative workflow

### Deprecated
- `DISPATCH.md` — replaced by TDD skill + gate CLI
- Raw `sqlite3 gate.db` workflow — use `gate` commands instead

---

## Sprint 52 — Pipeline Active Executor: Supervisor — 2026-03-20

### Added
- **Resident supervisor** (T-52.3): `maenifold-acp pipeline` runs as a managed child process with full lifecycle controls
  - `run`: spawn supervised process with process group isolation
  - `cancel`: signal entire process tree via negative PGID
  - `pause`/`resume`: SIGSTOP/SIGCONT to process group with frontmatter state persistence
  - `status`: PID, uptime, and restart attempts
- **Health monitoring + auto-restart**: monitor thread distinguishes ExitedClean/ExitedCrash/Gone; auto-restarts on crash up to 3 attempts; marks pipeline stuck on exhaustion
- **Orphan detection** (T-52.3 Wave 2c): `supervisor_detect_orphans` scans active/paused pipelines on run, marks untracked sessions stuck

### Fixed
- **Delete best-effort** (T-52.2): distinguish NotFound from IO errors; frontmatter-based active guard prevents deletion of running pipelines
- **Signal delivery check**: `supervisor_pause`/`resume` now error on ESRCH (process gone) instead of silently succeeding
- **Resume pre-condition**: `resume_pipeline` rejects calls when pipeline is not paused
- **Process group termination**: `process_group(0)` + `kill(-(pid))` ensures entire subprocess tree is signaled on cancel
- **Dispatch test flakiness**: all 9 dispatch tests annotated with `#[serial(config)]` to prevent parallel config corruption

### Pipeline Stats
- 95 pipeline tests passing (was 68 at sprint start)
- 11 test gaps closed from Opus blue-team audit (B+ grade)
- 3 red-team audits, 4 findings fixed, 2 deferred (TOCTOU, PID reuse)

---

## Sprint 35 — Upstream UI Rebuild + ACP Compliance — 2026-03-18

### Added
- **Vendored upstream acp-ui** — replaced MSBrett/acp-ui fork submodule with plain files from upstream `formulahendry/acp-ui`
- **Multi-session architecture** (T-46.1–T-46.3): `ref<Map<string, SessionState>>` with per-session `AcpClientBridge` instances; sidebar-based session switching
- **Store decomposition** (T-46.4): `session.ts` split into composables (`useSessionLifecycle`, `useMessageHandler`, `usePermissions`, `useSessionPersistence`, `session-utils`)
- **Mermaid diagram rendering** (T-30.1): `useMermaid` composable with lazy init, dark theme, streaming-safe gating
- **Agent plan support** (T-42.6): `PlanView` component with status icons and priority badges
- **Pinia skill** — Vue state management reference for agents

### Changed
- **20 ACP compliance specs** replayed through TDD pipeline (SWE → red-team → blue-team) against the new upstream-based UI
- **Sync continue-on-error** (T-46.5): `run_sync`/`sync_files` collect per-file errors and continue
- **Session map reactivity** (T-46.6): `reactive()` wrappers + Map reassignment for Vue deep tracking
- **Mermaid streaming fix**: Gate `renderMermaidBlocks` on `!isLoading` to prevent incomplete fences from being permanently marked as processed

### Removed
- MSBrett/acp-ui GitHub fork (deleted)
- `.gitmodules` submodule configuration

### Pipeline Stats
- 20 specs through SWE → Red-team → Blue-team pipeline
- 1 red-team reject (6.6% reject rate)
- ~60 agent dispatches across GPT-5.4/5.2 (SWE/Blue) and Claude Sonnet 4.6 (Red)
- 0 stuck tasks

---

## Sprint 34 — Multi-Tab Sessions — 2026-03-17

### Added
- **Tab state model** (T-44.1): `TabState`, `TabInfo` interfaces and `createEmptyTabState()` factory for per-tab session isolation
- **Multi-session store** (T-44.2): Session store refactored from singleton to `tabs: Map<string, TabState>` + `activeTabId` with backward-compatible computed shims
- **TabStrip component** (T-44.3): Horizontal tab bar with active/connecting/disconnected states, close buttons, and pending-action badges
- **App.vue layout** (T-44.4): TabStrip integrated, sidebar always-available as launcher, per-tab content rendering
- **ChatView per-tab** (T-44.5): Scroll position and input draft preserved per tab, auto-scroll on tab switch
- **SessionList tabs** (T-44.6): Duplicate-tab prevention, focus existing tabs, delete safety for open sessions
- **Tab-aware dialogs** (T-44.7): Permission and auth dialogs show session context; pending-action indicators on non-active tabs

### Changed
- Sidebar no longer toggles between "New Session" / "Disconnect" — always shows launcher
- Multiple simultaneous ACP agent sessions supported (each in its own tab)
- StartupProgress moved from sidebar to main content area

### Architecture
- `AcpClientBridge` instances are per-tab (already instance-capable, now properly utilized)
- Bridge cleanup on tab close with proper watcher disposal
- Foundation for DAG workflow orchestrator tab type (Sprint 35)

---

## Sprint 33 — Full ACP Protocol Compliance — 2026-03-17

### Added
- **Content block refactor** (T-43.1): `ChatMessage.blocks: ContentBlock[]` supporting text, image, audio, resource, and resource_link types with full rendering
- **Session list** (T-43.2): `session/list` RPC with capability detection, cursor pagination, agent↔local session merge, "from agent" indicator
- **Terminal subsystem** (T-43.6): All 5 `terminal/*` methods (create, output, wait_for_exit, kill, release), `TerminalManager`, `TerminalOutputBlock.vue`, Rust backend with ring-buffer output
- **Prompt capabilities** (T-43.7): Client declares `prompt.image/audio/embeddedContext` capabilities; UI gates attachment controls based on agent support
- **Multiple permissions** (T-43.10): Permission requests queued via `Map<string, PendingPermissionEntry>` instead of singleton; sequential UI with queue count
- **Permission fidelity** (T-43.15): Full tool-call context (content, rawInput, rawOutput) displayed in permission dialogs
- **Tool content rendering** (T-43.13): Expandable tool-call cards with text/diff/terminal content blocks
- **Resource link fields** (T-43.14): `title`, `description`, `size` (including bigint) on resource_link content blocks
- **Frame validation** (T-43.16): JSON-RPC validation on both relay (WebSocket) and app (Tauri IPC) inbound/outbound paths
- **ACP extensibility** (T-43.3): `_meta` on all request/response types, `_session/set_model` wire name, capability advertisement

### Changed
- **Per-method timeouts** (T-43.5, T-43.9): Replaced blanket 60s timeout with method-specific values; `session/prompt` gets 30min
- **SDK type alignment** (T-43.4): `SlashCommand` → SDK `AvailableCommand` type; no more flattening
- **Config during generation** (T-43.20): Mode/model/config changes allowed while prompt is generating; optimistic updates with rollback
- **configOptions presence** (T-43.21): Check field presence (`'configOptions' in response`) not array length; empty array ≠ absent
- **session_info targeting** (T-43.18): `session_info_update` scoped by `sessionId`; updates `savedSessions` and `currentSession` independently
- **updatedAt null** (T-43.19): `updatedAt: null` clears the timestamp (sets to 0) instead of `Date.now()`
- **cwd validation** (T-43.8): `session/load` validates absolute path including Windows UNC paths
- **Tool-only turns** (T-43.11): Stub assistant message synthesized when `tool_call` arrives before any text
- **rawInput/rawOutput** (T-43.12): Stored on `ToolCallInfo`, propagated through creation and update paths
- **Clean stdio** (T-43.17): Removed login shell wrapper (`-l` flag) to prevent stdout contamination from shell profiles

### Pipeline Stats
- 21 specs through SWE → Red-team → Blue-team pipeline
- 2 red-team rejects caught and fixed (4.9% reject rate)
- ~50 agent dispatches across GPT-5.4 (SWE/Blue) and Claude Sonnet 4.6 (Red)

---

## Sprint 32 — ACP Protocol Compliance Audit (Initial) — 2026-03-16

### Added
- **Protocol version check** (T-42.1): Shared `performInitialization()` helper, `agentInfo`/`promptCapabilities` stored
- **Agent plan support** (T-42.6): `plan` session update handler, `PlanView.vue` component
- **Session info updates** (T-42.7): `session_info_update` notification handler
- **ConfigOptionPicker** (T-42.4): Generic dropdown for any ACP configOption; legacy pickers gated

### Changed
- **Session setup** (T-42.2): Absolute cwd validation, `loadSession` capability re-check, resume abort guards
- **Prompt turn** (T-42.3): `cancelPermission` on cancel, tool call cancellation, `stopReason` UI
- **Tool call content** (T-42.5): `content` field stored and accumulated on updates; `locations` with line
- **Slash hint** (T-42.8): Hint rendered as placeholder, dead code removed, listener leak fixes
- **Filesystem** (T-42.9): Absolute path validation, `mkdir -p`, null return
- **Transport bridge** (T-42.10): JSON-RPC relay frames, bridge method, content warnings

---

## Sprint 29 — System Tray Icon & Remove Feature Flags — 2026-03-15

### Added
- **System tray icon** (FR-35): macOS menu bar icon with "Open Web UI", "Help", "Quit" menu items when running `--web` or `--http`. Uses `tray-icon` + `winit` for native event loop on main thread; tokio server on background thread. Graceful shutdown via `CancellationToken`.
- **Cognitive Geodesics** tray icon art: generative algorithmic art at `assets/tray-icon.png` — concept nodes warping a knowledge manifold with geodesic particle flows.
- Dependencies: `tray-icon`, `muda`, `winit`, `open`, `image`, `objc2-core-foundation` (macOS)

### Changed
- **Feature flags removed** (FR-36): Removed `http` and `web` cargo feature flags from all crates. All deps compile unconditionally. `cargo build --release -p maenifold-cli` works without `--features`. All 10 `#[cfg(feature = "...")]` gates removed.
- `Makefile`: `--features web` removed from build command
- `main.rs` restructured: dispatches `--web`/`--http` to tray event loop, `--mcp` stays headless

---

## Sprint 28 — Replace chrome-acp with ACP UI — 2026-03-15

### Changed
- **Web UI replaced**: chrome-acp (minimal React) → ACP UI (feature-complete Vue 3 SPA)
- New features from ACP UI: multi-agent sessions, agent thinking view, slash commands, permission dialogs, model picker, mode selector, traffic monitor, session management
- **Relay simplified**: chrome-acp proxy protocol → transparent JSON-RPC passthrough (browser sends raw ACP JSON-RPC over WebSocket, relay pipes to agent stdin/stdout)
- `agent-client-protocol` crate dependency removed — relay is now pure subprocess + pipe management
- `manager.rs` (ACP SDK wrapper) deleted — relay handles subprocess lifecycle directly

### Added
- `vendor/acp-ui` submodule (fork of formulahendry/acp-ui, Tauri shell stripped)
- `src/lib/relay.ts` — WebSocket adapter replacing Tauri invoke/listen
- stderr forwarding as `{"type":"stderr","data":"..."}` for ACP UI startup progress display

### Removed
- `vendor/chrome-acp` submodule
- `crates/maenifold-core/src/acp/manager.rs`
- `agent-client-protocol` dependency from Cargo.toml

---

## Sprint 27 — MCP Server Hardening — 2026-03-15

### Fixed
- **CRITICAL**: Tool/resource handlers offloaded to `spawn_blocking` — no longer block async runtime (filesystem, SQLite, ONNX inference)
- **CRITICAL**: HTTP sessions expire after 5 min idle (`keep_alive: 300s`) — no more unbounded session accumulation
- CORS applied to `/mcp` endpoint in web mode (was only on API sub-router)
- `Mcp-Session-Id` header exposed in CORS responses for browser MCP clients
- HTTP/web transports no longer falsely advertise `resources/list_changed` capability
- `MaenifoldError::Message` removed from validation error classifier — internal errors no longer misclassified
- Panic in `replace_section` replaced with proper error return (`expect` → `ok_or_else`)
- Redundant `initialize_runtime` call removed from resource request hot path

### Changed
- `rust-mcp-expert.agent.md` fully rewritten for rmcp 1.1.0 (was written for v0.8+, contained 18+ non-compilable examples)

### Requirements Completed
- FR-37 (RTM-227–236): MCP Server Hardening
- FR-38 (RTM-237–239): Rust MCP Expert Agent Rewrite

---

## Sprint 26 — Web UI Full Feature Parity — 2025-07-23

### Added
- 22 UI components promoted from chrome-extension to shared package (chain-of-thought, plan, checkpoint, suggestion, artifact, image, sources, task, loader, open-in-chat, queue, web-preview, canvas, controls, panel, toolbar, node, edge, connection, inline-citation, confirmation, context)
- 3 shared UI primitives promoted (alert, carousel, progress)
- ChatInterface.tsx wired with ChainOfThought, Plan, Checkpoint, Suggestions, Artifact, Task, Sources
- New shared deps: @xyflow/react, embla-carousel-react, @radix-ui/react-progress, tokenlens

### Fixed
- **SECURITY**: CORS restricted to localhost origins (replaced `CorsLayer::permissive()`)
- **SECURITY**: Relay MPSC channel bounded to 256 (was unbounded — OOM risk)
- **SECURITY**: Exec validation uses `libc::access(X_OK)` instead of mode bitmask
- **SECURITY**: `agent-client-protocol` pinned to specific git rev (was floating HEAD)
- Consolidated duplicate shimmer.tsx and reasoning.tsx between extension and shared
- 10 pre-existing TypeScript errors in shared package fixed

### Requirements Completed
- FR-32 (RTM-204–218): Web UI — full component parity with chrome extension
- FR-32-SEC: 4 security fixes from red-team audit

---

## Sprint 23 — Streamable HTTP Transport — 2026-03-14

### Added
- MCP Streamable HTTP transport via `maenifold --http [--port <PORT>]`
- `serve_http(port)` using rmcp `StreamableHttpService` + axum 0.8 router
- `LocalSessionManager` for `Mcp-Session-Id` session lifecycle
- Permissive CORS for browser-based MCP clients
- Graceful shutdown via `CancellationToken` + SIGINT handler
- `--http` / `--port` CLI flags (mutually exclusive with `--mcp`, default port 8080)
- 3 HTTP transport integration tests (initialize, session validation, tool invocation)
- HTTP deps gated behind `http` Cargo feature flag

### Requirements Completed
- FR-23 (RTM-126–130): Streamable HTTP Transport

---

## Sprint 22 — WikiLink Filter in Sync — 2026-03-13

### Added
- `is_concept_blocked()` API on WikiLinkFilter — single-concept filter check with exact + prefix matching
- WikiLink filter wired into `upsert_processed_file` for full sync (`run_sync`)
- WikiLink filter wired into `sync_single_file` for incremental sync
- 9 boundary tests covering RTM-151 through RTM-156
- Incremental sync reports filtered concept count in output

### Fixed
- **CRITICAL**: Prefix normalization bug in WikiLink filter — blocked prefixes were matching unrelated concepts

### Requirements Completed
- FR-19 (RTM-151–156): WikiLink filter applied during sync

---

## Sprint 21 — Vec0 Table Creation + Verification Parity — 2026-03-12

### Added
- `ensure_vec0_tables` uses 4-level dimension fallback chain (runtime → metadata → table introspection → DEFAULT 384)
- `vec0_extension_available` verifies table accessibility with real query, not just extension load
- Sync verification report queries actual `vec_concepts`/`vec_memory_files` row counts
- Phase timing telemetry in `run_sync()` — 5 Instant checkpoints for file processing, graph rebuild, embeddings, community detection, total

### Requirements Completed
- FR-8 (RTM-058, RTM-060, RTM-149, RTM-150): Dynamic vec0 schema parity

---

## Sprint 20 — Vector Sync Diagnostics — 2026-03-11

### Added
- ONNX inference error details surfaced for concept embedding failures (actionable diagnostics)
- Model discovery resolves ONNX models via canonicalized executable path (symlink support)
- Model discovery checks shared runtime root `~/maenifold/assets/models/` for C# cross-compat

### Requirements Completed
- FR-7 (RTM-147, RTM-148): Dynamic model loading — symlink + shared root discovery

---

## Sprint 15 — CLI Parity Remediation — 2026-03-11

### Fixed
- 27 CLI parity fixes from oracle comparison across all tool categories:
  - SearchMemories NULL distance crash (Hybrid/Semantic/Tags modes)
  - BuildContext format + depth traversal matches C# oracle
  - FindSimilarConcepts displays weighted_similarity
  - Removed 'Invalid argument:' prefix from error messages
  - CLI help tool groupings match C# (MCP group, no Adopt:/Assets: prefixes)
  - Missing `--payload` now rejected (was silently using `{}`)
  - Empty title + integer folder type validation
  - Nested WikiLink regex rejects malformed brackets
  - RepairConcepts blocks when vec0 unavailable
  - Workflow step JSON pretty-printed
  - RepairConcepts output format matches C#
  - ExtractConcepts zero-result message matches C#
  - ReadMemory shows N/A for missing dates, timestamps in local time
  - `--help` exits 1 (matches C#)
  - IO error mapped to correct JSON error code
- Vec0 extension loading deduplicated and reliable

### Requirements Completed
- NFR-1: CLI output matches C# oracle across all 28 tools

---

## Sprint 14 — Red-Team Drift Remediation — 2026-03-10

### Fixed
- Decay fallback: uses creation date when `last_accessed` is None (was returning 1.0)
- Graceful frontmatter parse: malformed YAML returns empty frontmatter, not Err
- CRLF line endings supported in frontmatter detection
- `co_occurrence_count` counts unique files, not sum of weights (fixes community detection)
- Symlink canonicalization added to path validation (security fix)
- NMI threshold env var renamed to `MAENIFOLD_COMMUNITY_NMI_THRESHOLD` (C# parity)
- Removed co_occurrence_count fallback in community edge weights
- Long titles truncated to 255 chars instead of rejected
- Semantic validation returns warning text, not Err (matches C# API contract)
- Legacy session IDs accepted (suffix-free format for cross-runtime compatibility)
- Session ID suffix uses random entropy (security hardening)

### Requirements Completed
- 11 red-team findings remediated with full traceability

---

## Sprint 13 — Parity Audit Remediation — 2026-03-10

### Added
- Separate search mode paths: Semantic (vector-only), FullText (text-only, normalized), Hybrid (RRF fusion)
- MemoryStatus returns full stats: folder count, file count, total size, graph stats, DB size
- `ToolResponse.WithNextSteps` and `WithHint` helpers wired into 26 tool return paths

### Fixed
- PreFusion multiplier: 3x (was 5x) matching C# `PreFusionMultiplier`
- FullText score normalization produces scores in [0.0, 1.0] range

### Removed
- 9 dead code functions behind `#[allow(dead_code)]` — evaluated and resolved

### Requirements Completed
- FR-6 (search parity), FR-7 (MemoryStatus), FR-8 (ToolResponse helpers)

---

## Sprint 12 — Structural Refactor / Facade Preservation — 2026-03-09

### Changed
- Extracted `workflow_ops.rs`, `mcp_resources.rs`, `graph_preview.rs`, `graph_analyzer.rs`, `similar.rs` from monolithic modules
- Extracted `sync.rs` from `search.rs` (sync/incremental-sync logic)
- Extracted `repair.rs` from `search.rs` (RepairConcepts, AnalyzeConceptCorruption)
- Extracted `similar.rs` from `search.rs` (FindSimilarConcepts)
- Extracted `decay.rs` from `memory.rs` (shared decay/date/path helpers)
- All public API facades preserved — zero behavioral change

### Requirements Completed
- NFR-1: search.rs decomposed from 3069 lines into focused modules

---

## Sprint 11 — Search Core Fix + MCP Resources — 2026-03-10

### Added
- MCP `resources/list` + `resources/read` capability
- MCP resource-change notifications via notify crate
- Auto-start sync watcher in `--mcp` mode
- Oracle parity tests for Adopt, UpdateAssets, RecentActivity, RunFullBenchmark, RepairConcepts, AnalyzeConceptCorruption

### Fixed
- SearchMemories text scoring: occurrence counting + title bonus (was binary matching)
- Real vector search: two-pipeline RRF architecture replacing `content.contains()` heuristic
- Vector search folder prefix overmatching
- Vector search tag filtering moved before RRF fusion
- `vec_memory_files` graceful degradation when table missing

### Requirements Completed
- FR-11 (RTM-068–071): MCP resource parity
- FR-12 (RTM-072–073): MCP auto-start watchers
- FR-13 (RTM-074–079): Oracle parity tests for all tools
- FR-21 (RTM-107–114): SearchMemories scoring parity

---

## Sprint 10 — Port Parity Completion — 2026-03-09

### Added
- 28 missing workflow JSONs copied (69/69 parity with C#)
- CI runs workspace-root tests including oracle parity
- docs/DEVELOPMENT.md created
- Bundled integration assets (claude-code hooks, README, QUICK_START, SKILL)

### Fixed
- Config env vars renamed to match C# (`MAENIFOLD_SESSION_ABANDON_MINUTES`, `MAENIFOLD_AUTO_SYNC`, + 8 missing vars added)
- RepairConcepts Mode B tests use real assertions (28/28 pass, no stub-acceptance)
- 4 Sprint 9 RTM boundary tests added (tokenizer.json, max tokens, vec0 dims, vec0 recreation)
- WikiLink filter reads `.wikilink-filter.json` (JSON format, matches C#)
- Asset catalog cross-repo masking removed
- 6 MCP stdio test failures fixed (framing mismatch)
- 12 oracle parity test failures fixed (6 fixed, 6 documented as ignored drift)

### Requirements Completed
- FR-10 (RTM-066–067): Workflow asset parity (69/69)
- FR-14 (RTM-080): CI parity gate
- FR-15 (RTM-081–090): Config env var parity (10 vars)
- FR-16 (RTM-091–093): Documentation remediation
- FR-17 (RTM-094–097): RepairConcepts Mode B verification
- FR-18 (RTM-098–101): Sprint 9 RTM coverage
- FR-19 (RTM-102–104): WikiLink filter format parity
- FR-20 (RTM-105–106): Asset catalog isolation

---

## Sprint 9 — Dynamic Model Loading + CI/CD — 2026-03-08

### Added
- Dynamic ONNX model discovery: scan `assets/models/*.onnx`, select largest, read dimensions from tensor
- `tokenizer.json` preferred (HuggingFace standard), fallback to `vocab.txt` + config
- `model_metadata` table for tracking model filename + dimensions
- Vec0 tables created with runtime dimensions; recreated on model change
- `build.yml`: fmt + clippy + test on push/PR to dev/main
- `release.yml`: matrix build for osx-arm64, linux-x64, win-x64

### Requirements Completed
- FR-7 (RTM-050–056): Dynamic model loading
- FR-8 (RTM-057–060): Dynamic vec0 schema
- FR-9 (RTM-061–065): CI/CD pipeline

---

## Sprints 1–3 — Core Tool Parity — 2026-03-07

### Added
- **Adopt** tool: role/color/perspective asset reading with two-step lookup (filename → JSON id scan)
- **AnalyzeConceptCorruption** tool: WikiLink variant scanning with 5-category classification + repair command generation
- **UpdateAssets** tool: recursive directory comparison (size + SHA256), dry-run mode, file copy
- **RecentActivity** tool: file_content DB query with type filtering, timespan, H2 section extraction
- **RunFullBenchmark** tool: 5 benchmark categories (CTE vs N+1, search modes, sync, complex traversal, system health)
- **RepairConcepts** tool: Mode A (WikiLink replacement), Mode B (WikiLink creation via pulldown-cmark AST), semantic validation via vec0 embeddings
- Boundary, CLI, and parity tests for all 6 tools
- Pre-existing clippy errors fixed (23 errors), `normalize_concept` + `extract_h2_sections` helpers added

### Requirements Completed
- FR-1 (RTM-001–007): Adopt tool
- FR-2 (RTM-008–013): AnalyzeConceptCorruption tool
- FR-3 (RTM-014–019): UpdateAssets tool
- FR-4 (RTM-020–030): RecentActivity tool
- FR-5 (RTM-031–037): RunFullBenchmark tool
- FR-6 (RTM-038–046): RepairConcepts tool
- NFR-4 (RTM-047–049): Clippy baseline

---

## Cumulative Status

| Metric | Value |
|--------|-------|
| Tools implemented | 28/28 (zero placeholders) |
| Tests passing | 500+ |
| Transports | stdio + Streamable HTTP |
| CLI parity | Matches C# oracle (27 format fixes in Sprint 15) |
| Sprints completed | 16 (Sprints 1–3, 9–15, 20–23) |
