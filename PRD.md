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

### FR-32: Web UI (`--web` flag) ✅ Sprint 26

The system SHALL support a `--web` CLI flag that launches a browser-accessible web interface on the same port as the MCP HTTP endpoint, combining an ACP chat panel for interacting with AI coding agents and a force-directed knowledge graph visualization.

### ~~FR-33: Web UI — Files & History Tabs~~ (DEPRECATED)

### ~~FR-34: Web UI — Community-Scoped Graph View~~ (DEPRECATED)

### FR-35: System Tray Icon ✅ Sprint 29

### FR-36: Remove Cargo Feature Flags ✅ Sprint 29

### FR-37: MCP Server Hardening ✅ Sprint 27

### FR-38: Rust MCP Expert Agent Rewrite ✅ Sprint 27

### FR-39: Replace chrome-acp with ACP UI ✅ Sprint 28

### FR-40: Pipeline Active Executor ✅ Sprint 51 + Sprint 56 + Sprint 57 + Sprint 58

The Pipeline tool SHALL become an autonomous DAG execution engine that drives ACP agents via `acpx` subprocesses, inverting the current passive state-tracker model.

- **Graph backend**: Pipeline DAG SHALL use `petgraph::StableGraph` for graph structure, cycle detection, topological ordering, and graph surgery (graft/join/fork).
- **AcpX driver**: The system SHALL spawn and manage `acpx` subprocesses via `tokio::process::Command`, using `--format json` for structured output parsing.
- **Execution engine**: On `run`, maenifold SHALL autonomously compute ready sets, dispatch nodes to ACP agents bounded by `maxConcurrency` (via `tokio::sync::Semaphore` + `JoinSet`), collect results, cascade state, and repeat until the DAG completes. The executor runs as a Rust-native async task (`executor.rs`) — the original bash bridge (`maenifold-acp pipeline`) was removed in Sprint 56.
- **Lifecycle controls**: The system SHALL support `run`, `pause` (stop dispatching new nodes), `resume`, `cancel` (abort active via `CancellationToken`, abandon remaining), and `rewind` (reset subgraph to Pending and re-execute). Pause/resume use an `AtomicBool` flag checked between dispatch cycles.
- **HSM integration**: Each pipeline node SHALL execute its workflow via the existing Workflow tool, with SequentialThinking submachines spawned when `requiresSequentialThinking=true`. Nesting: Pipeline → Workflow → SequentialThinking.
- **Graph surgery**: The system SHALL support `graft` (add nodes/edges), `join` (merge DAGs), and `fork` (clone from a node) operations on pipeline DAGs, leveraging `StableGraph` stable indices.
- **CRUD**: The system SHALL support `create`, `view`, `delete` pipeline operations via MCP for agents and UI.
- **State persistence**: Pipeline state SHALL persist as markdown + YAML frontmatter (existing pattern). No database migration.

### FR-41: File-Authoritative Pipeline Gate Cache ✅ Sprint 53 + Sprint 54

CLI tool (`gate`) making todo/done markdown files authoritative for pipeline task state, with gate.db as a rebuildable cache. Replaces raw sqlite3 gate.db commands.

- **Authoritative state**: Pipeline task state SHALL live entirely in markdown artifacts under `todo/` and `done/`.
- **Cache semantics**: `gate.db` SHALL be treated as a derived cache/index used to avoid rescanning thousands of task files during normal pipeline operations.
- **Advance enforcement**: `gate advance` SHALL validate the requested transition against the task file, append the canonical stage section, and only then update the DB cache.
- **Sync recovery**: `gate sync` SHALL rebuild or reconcile `gate.db` from on-disk task state.
- **Status inspection**: `gate status` SHALL report effective task state derived from disk, with optional cache cross-check metadata.
- **Parser hardening** (Sprint 54): Parser SHALL tolerate rework cycles, out-of-order headings, emoji verdicts, implicit verdicts, no-SWE done files, and case-insensitive CLI input.
- **Location-authoritative fallback**: Files in `done/` SHALL be treated as done regardless of incomplete audit trail, with a diagnostic note preserved in status output.

### FR-42: Configurable Pipeline Engine

Replace the hardcoded SWE→Red→Blue pipeline with a JSON-driven process engine. Simplify the CLI to three state-changing commands with role-based separation enforced by agent tool configuration.

#### Pipeline Definition

- The system SHALL load pipeline stage definitions from a JSON file (e.g., `pipeline.json`).
- Each stage SHALL declare: name, valid verdicts (pass/reject/fail), transition targets on each verdict, and optionally allowed/denied agent roles.
- The JSON definition SHALL be validated as a DAG on load. Cycles SHALL be rejected.
- Predefined pipelines SHALL be shipped (e.g., `tdd.json`: swe→red→blue; `lightweight.json`: swe→blue).
- `--pipeline <path>` SHALL override the default; if omitted, gate SHALL look for `pipeline.json` in the root.

#### ID-Based CLI

- All state-changing commands SHALL accept a task ID (e.g., `T-54.1`), NOT a filename.
- The database SHALL map task IDs to file paths, current stage, location, and history.
- `gate status` and `gate status <ID>` SHALL resolve from the database. `gate sync` rebuilds the mapping.

#### Command Separation

- `gate append <ID> --notes "..." --agent <role>` — Logs work to the task file. Does NOT change pipeline state. Available to subagents.
- `gate pass <ID> --notes "..."` — Advances the task to the next stage per the pipeline definition. Moves file to `done/` on terminal pass. Orchestrator-only.
- `gate reject <ID> --notes "..."` — Rolls the task back per the pipeline definition (e.g., to pending). Orchestrator-only.
- `gate advance` SHALL be removed. Its responsibilities are split across `pass`, `reject`, and `append`.

#### Authorization Model

- Authorization SHALL be enforced by agent tool configuration, NOT by the binary.
- Subagent configs SHALL include `gate append` only. `gate pass` and `gate reject` SHALL be excluded from subagent tool lists.
- `pass` and `reject` SHALL implicitly record `orchestrator` as the agent role.
- The binary SHALL NOT implement authentication, tokens, or cryptographic verification.

## FR-43: Pipeline UI Frontend Components

The standalone pipeline-ui application (`/Users/brett/src/ma-collective/pipeline-ui/`) SHALL provide a visual DAG editor and runner for maenifold pipelines, communicating exclusively via MCP Streamable HTTP to the maenifold server.

FR-43.1: The application SHALL include a PipelineInspector component that shows context-sensitive configuration (design mode) or read-only status with action buttons (server mode) for the selected node or edge.

FR-43.2: The application SHALL include a PipelineRunner component with lifecycle control buttons (create/run/pause/resume/cancel/delete), an event log tab, and a mermaid diagram tab.

FR-43.3: The application SHALL include a PipelineSidebar component that lists all pipelines from the server, supports creating new pipelines, loading existing ones, and deleting them.

FR-43.4: The application SHALL include a PipelineView shell component that composes the sidebar, composer, inspector, and runner into a four-panel layout, integrated into the main App.vue via a toggle button.

FR-44: The PipelineComposer SHALL provide a node creation toolbar in design mode enabling users to add nodes with workflow ID, optional ACP client, and connect them with edges.

FR-45: The pipeline-ui test suite SHALL use ONLY real integration tests against a live maenifold --http server. No mocked fetch, no mocked composables, no fake responses. Per TESTING_PHILOSOPHY.md.

FR-46: The pipeline-ui SHALL include functional smoke tests that verify UI components render, respond to user interaction, and display real data from the MCP server.

### FR-47: Pipeline Dataflow Architecture

The Pipeline tool SHALL support three distinct node types with per-node prompts and predecessor context propagation, replacing the current flat prompt model where every node receives the same run-level prompt.

- **Node types**: `PipelineNode` SHALL be a serde-tagged enum (`#[serde(tag = "type")]`) with three variants: `Workflow` (existing, extended with per-node `prompt`), `Prompt` (new — single prompt to ACP agent), and `SequentialThinking` (new — drives a thinking session to completion).
- **Backwards compatibility**: Existing pipeline JSON (`{ "id", "workflowId", "acpClient" }`) without a `type` field SHALL deserialize as a Workflow node via `#[default]` on the `PipelineNodeKind` enum.
- **Per-node prompt**: Each node SHALL carry its own prompt/topic. The run-level prompt becomes optional ambient context. Prompt assembly order: predecessor context → ambient prompt → node prompt → (workflow step text for workflow nodes only).
- **Output capture**: `NodeState` SHALL include `output: Option<String>`. On node completion, the executor SHALL capture the agent's conclusion/response into `NodeState.output` for downstream consumption.
- **Predecessor context**: The executor SHALL build predecessor context from upstream `NodeState.output` values (not from session files). The existing `predecessor_context_content()` pattern is preserved but reads from in-memory output.
- **Type-based dispatch**: `dispatch.rs` SHALL branch on node type — `activate_workflow_node_session` for workflow nodes only. Prompt and SequentialThinking nodes SHALL create appropriate sessions without workflow activation.
- **Type-based execution**: `executor.rs` SHALL branch on node type — workflow → `run_workflow()`, prompt → single ACP agent prompt, sequentialThinking → `drive_think_session()`.
- **Validation**: `validate_definition()` SHALL enforce type-specific required fields (workflow needs `workflowId`, prompt needs `prompt`+`agent`, sequentialThinking needs `topic`+`agent`).
- **Mermaid display**: `view.rs` SHALL render different node shapes/styles per type in the Mermaid DAG diagram.
