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

### FR-40: Pipeline Active Executor — Sprint 51

The Pipeline tool SHALL become an autonomous DAG execution engine that drives ACP agents via `acpx` subprocesses, inverting the current passive state-tracker model.

- **Graph backend**: Pipeline DAG SHALL use `petgraph::StableGraph` for graph structure, cycle detection, topological ordering, and graph surgery (graft/join/fork).
- **AcpX driver**: The system SHALL spawn and manage `acpx` subprocesses via `tokio::process::Command`, using `--format json` for structured output parsing.
- **Execution engine**: On `run`, maenifold SHALL autonomously compute ready sets, dispatch nodes to ACP agents bounded by `maxConcurrency` (via `tokio::sync::Semaphore` + `JoinSet`), collect results, cascade state, and repeat until the DAG completes.
- **Lifecycle controls**: The system SHALL support `run`, `pause` (drain active, stop dispatching), `resume`, `cancel` (abort active, abandon remaining), and `rewind` (reset subgraph to Pending and re-execute).
- **HSM integration**: Each pipeline node SHALL execute its workflow via the existing Workflow tool, with SequentialThinking submachines spawned when `requiresSequentialThinking=true`. Nesting: Pipeline → Workflow → SequentialThinking.
- **Graph surgery**: The system SHALL support `graft` (add nodes/edges), `join` (merge DAGs), and `fork` (clone from a node) operations on pipeline DAGs, leveraging `StableGraph` stable indices.
- **CRUD**: The system SHALL support `create`, `view`, `delete` pipeline operations via MCP for agents and UI.
- **State persistence**: Pipeline state SHALL persist as markdown + YAML frontmatter (existing pattern). No database migration.

### FR-41: File-Authoritative Pipeline Gate Cache ✅ Sprint 53

CLI tool (`gate`) making todo/done markdown files authoritative for pipeline task state, with pipeline.db as a rebuildable cache. Replaces raw sqlite3 pipeline.db commands.

- **Authoritative state**: Pipeline task state SHALL live entirely in markdown artifacts under `todo/` and `done/`.
- **Cache semantics**: `pipeline.db` SHALL be treated as a derived cache/index used to avoid rescanning thousands of task files during normal pipeline operations.
- **Advance enforcement**: `gate advance` SHALL validate the requested transition against the task file, append the canonical stage section, and only then update the DB cache.
- **Sync recovery**: `gate sync` SHALL rebuild or reconcile `pipeline.db` from on-disk task state.
- **Status inspection**: `gate status` SHALL report effective task state derived from disk, with optional cache cross-check metadata.
