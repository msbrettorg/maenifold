<p align="center">
  <img src="docs/branding/maenifold-logo.svg" alt="maenifold">
</p>

<p align="center">
  <strong>The reasoning console.</strong> Knowledge graph · pipeline orchestration · multi-agent coordination.
</p>

<p align="center">
  <a href="https://github.com/msbrettorg/maenifold-rs"><img src="https://img.shields.io/github/license/msbrettorg/maenifold-rs?style=flat-square" alt="MIT License"></a>
</p>

<p align="center">
  <img src="docs/branding/pipeline-screenshot.jpg" alt="18-node research pipeline with branching topology, all nodes completed, in the maenifold desktop pipeline editor" width="800">
</p>

**Status:** Pre-1.0 developer preview. Build from source. Desktop and pipeline features are experimental.

For anyone who spends more time gathering context and re-doing analysis than doing the actual work.

**Product thesis:** Build the reasoning layer first. Useful work follows.

# maenifold

maenifold is a local-first Rust backend plus an Electron desktop app for working with persistent agent memory, ACP sessions, reusable workflow assets, visual pipelines, and gated delivery. The desktop app, the CLI, and external AI clients all hit the same tool surface — agents and humans share the same core domain operations. The goal is simple: one place to reason, orchestrate, execute against the world, and keep context alive across sessions.

Pipelines can mix tool calls, agent sessions, and workflow nodes in a single executable DAG.

The knowledge graph is built from ordinary markdown and `[[WikiLinks]]` — write notes, sync the graph, and the corpus becomes searchable and traversable. The source of truth stays readable because the memory layer is still just files plus a local database.

These pieces connect through shared local state — memory, workflows, pipelines, gate records, and the graph all reference each other.

## Memory & Knowledge Graph

The memory layer is the operational substrate underneath everything else. Pipelines pull context from it. Workflows write conclusions back to it. Agents read from it at session start. Gates reference it for decision history.

- **Graph-aware retrieval, not just search.** `BuildContext` traverses related concepts at configurable depth — direct relations at depth 1, expanded neighborhoods at depth 2+. Community detection clusters the graph into domain neighborhoods automatically without manual tagging.
- **Hybrid search with local embeddings.** Queries fuse semantic similarity and full-text matching through reciprocal rank fusion, with configurable weights. Embeddings run locally via ONNX with no external API calls.
- **Freshness-aware ranking.** Different content types age at different rates — thinking sessions, workflow outputs, and general notes each have their own grace periods. Validated knowledge holds its weight; invalidated assumptions fade faster. The corpus grows over time, but search stays focused on what's still relevant.
- **Memory hygiene is built in.** `AnalyzeConceptCorruption` finds duplicate or variant WikiLinks (`[[auth]]` vs. `[[authentication]]`). `RepairConcepts` consolidates them with dry-run-by-default and semantic similarity thresholds to block unsafe merges.
- **Reasoning artifacts persist.** Thinking sessions, assumption ledgers, and workflow outputs write back into the same memory layer — so future runs start with more context than the last one.

Nearly half of maenifold's MCP tools are memory, search, graph, or repair operations. The knowledge layer is the core of the system, not a feature alongside it.

## Core Concepts

- **MCP** (Model Context Protocol) — the protocol maenifold uses to expose tools. Any MCP-compatible client can connect.
- **ACP** (Agent Client Protocol) — the protocol for multi-turn agent sessions. The Agents tab speaks ACP.
- **WikiLinks** — double-bracket links like `[[concept]]` in markdown files. These become nodes in the knowledge graph.
- **Pipeline** — a directed acyclic graph (DAG) of nodes that execute in dependency order.
- **Gate** — file-authoritative task management. Tasks live as markdown files and progress through stages.
- **Workflow** — a reusable JSON template that defines a structured reasoning process.

The backend centers on `crates/maenifold-core` and exposes MCP tools across eight categories. The desktop app includes dedicated surfaces for Agents, Pipelines, Gates, Workflows, and Graph. Current repo configuration targets `Copilot`, `Goose`, `OpenCode`, `Codex`, and `claude-code` agents. The backend and all persistent state run locally. MIT licensed.

## Main Surfaces

### Agents

- ACP chat sessions against external agents from one desktop shell.
- Choose the ACP harness, mode, model, and reasoning effort per session, with permission prompts and traffic inspection built into the UI.
- Designed for multi-agent work without changing the backend protocol.
- The reasoning layer needs a way to talk to agents. This is it.

### Pipelines

- Visual DAG editor with runtime status, logs, and inspector panels.
- Node types include `workflow`, `prompt`, `sequentialThinking`, and `tool`.
- A single pipeline can mix different ACP harnesses, modes, models, and reasoning settings across nodes.
- Tool nodes can execute MCP calls directly, so structured retrieval and system actions do not require model-token spend.
- Workflow nodes and graph-backed context let pipelines reuse structured reasoning playbooks instead of rebuilding the same process on every run.
- Supports pipeline surgery: `fork`, `graft`, `join`, and `rewind`.
- **Pipelines double as evaluations.** Fork a completed pipeline, re-run it with a different harness configuration, and compare execution traces. Graft scoring nodes onto a completed run without re-executing upstream work. Every execution trace persists with full provenance — node outputs, timing, agent attribution, thinking session content. No separate eval framework needed; the pipeline executor IS the benchmark runtime.

### Gates

- `gate` is a separate binary for file-authoritative task management.
- Gates provide the local work ledger for staged delivery, review, and rework inside maenifold.
- Think of it as a local issue tracker that agents and humans can both drive.
- Supports staged delivery flows through `status`, `append`, `pass`, `fail`, `history`, and `sync`.
- The same task system can be driven from CLI, MCP, or the desktop UI.
- How the reasoning layer tracks and stages work that matters.

### Workflows

- Browse and filter dozens of bundled workflows from the asset library.
- Each workflow is a structured reasoning template — more than a prompt, less than a full application.
- Author and inspect workflow JSON in the desktop workflow editor.
- Import external workflow JSON and save it back into the local library.
- Reasoning methods you define once and reuse across pipelines and sessions.

### Graph

- Community view for the whole graph, then concept drill-down inside a selected cluster.
- Built from local WikiLinks, not a hidden hosted service.
- The visual surface for the memory and graph layer described above.
- Useful for spotting domain neighborhoods before opening the underlying memory files.

## Architecture

maenifold is MCP-native by design. The server speaks stdio MCP and streamable HTTP, and the pipeline runtime can call external MCP servers over stdio or streamable HTTP/HTTPS. All domain state lives behind MCP instead of being reimplemented in each frontend. The repo is a monorepo: the Rust workspace owns the backend crates, while `apps/desktop/` lives alongside it as an Electron + Vue app and is excluded from the Rust workspace itself.

Because MCP is the only domain interface, there is no capability gap between human and agent. An agent can write memories, create pipelines, dispatch other agents, and pass or fail gate tasks — the same operations a human performs through the desktop app. Pipelines are not static: an agent running inside a pipeline can fork the pipeline it's executing in, and completed upstream work is preserved. Mutation of a live run (graft, rewind, join) is intentionally guarded — cancel or complete first, then reshape.

## Example Flow

Write a note, build a graph, search it, run a pipeline:

1. **Create a memory:** `maenifold tool WriteMemory '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]] because [[SAML]] was too heavy for our use case."}'`
2. **Sync the graph:** `maenifold tool Sync '{}'` — WikiLinks become concept nodes with edges.
3. **Search it:** `maenifold tool SearchMemories '{"query":"authentication","mode":"Hybrid"}'` — returns the note ranked by relevance.
4. **Explore in the desktop app:** Open the Graph tab to see `OAuth2`, `authentication`, and `SAML` clustered together.
5. **Run a pipeline:** Create a DAG in the Pipelines tab with a tool node that calls SearchMemories, feeding results into a prompt node for synthesis.

The graph accumulates context over time. Pipelines and workflows can pull from it automatically, so reasoning improves as the corpus grows.

### In Practice

A `deal-prep` pipeline pulls account data from a CRM, deployment status from workplace intelligence, and licensing docs — six tool nodes in parallel, zero model tokens on the data gathering. It routes results through a structured analysis workflow, synthesizes findings in a persistent thinking session, and generates a slide deck. The output is a deliverable for a customer meeting, not code.

A research pipeline works differently: start with a hypothesis, run experiment nodes, then graft new experiments onto the living DAG as results suggest new directions. An 18-node memory subsystem benchmark grew this way — each graft extending the investigation without rerunning completed upstream work.

The same pipeline structure serves any domain where the work is: gather context, apply a method, produce an output, and optionally evolve the process as you learn.

```text
maenifold-rs/
├── crates/
│   ├── maenifold-core/              # Library — tools, search, graph, ACP, pipeline engine
│   ├── maenifold-cli/               # Binary — CLI + MCP server
│   ├── maenifold-gate/              # Binary — file-authoritative task management
│   └── maenifold-pipeline-driver/   # Binary — headless pipeline executor
├── apps/desktop/                    # Electron + Vue 3 desktop application
├── assets/                          # Workflows, roles, colors, perspectives, integrations
└── docs/                            # Architecture, guides, research
```

## Prerequisites

- Rust 1.75+ (stable toolchain)
- Node 18+ and npm (for the desktop app)
- `apps/desktop/` uses Electron + Vite for the desktop shell
- macOS ARM64, Linux x64, or Windows x64

## Quick Start

**Fastest first win — CLI in three commands:**

Build the CLI from source:

```bash
cargo build --release -p maenifold-cli -p maenifold-pipeline-driver -p maenifold-gate
```

Three binaries land in `./target/release/`: `maenifold` (CLI + MCP server), `maenifold-pipeline-driver` (headless pipeline executor), and `gate` (task management). All three must be on your PATH for full functionality. The easiest way is `./rebuild.sh`, which builds, symlinks, restarts services, and packages the desktop app.

Minimal MCP client config:

```json
{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}
```

This JSON goes in whatever MCP client config your agent host expects (e.g., `.copilot/config.json`, `claude_desktop_config.json`).

CLI examples using the current direct-tool syntax:

```bash
maenifold tool WriteMemory '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]]"}'
maenifold tool SearchMemories '{"query":"authentication","mode":"Hybrid"}'
maenifold tool BuildContext '{"conceptName":"authentication","depth":2}'
```

On a fresh install with no memory files, SearchMemories returns an empty result set — that's expected.

**Desktop app (Electron + Vue):**

```bash
cd apps/desktop
npm install && npm run dev:electron
```

A desktop window should open with tabs for Agents, Pipelines, Gates, Workflows, and Graph across the top.

For HTTP mode, the backend also supports `maenifold --http` and exposes streamable MCP at `/mcp`.

## Tools

maenifold exposes roughly 30 MCP tools across eight categories:

- **Memory** — `WriteMemory`, `ReadMemory`, `EditMemory`, `DeleteMemory`, `MoveMemory`, `ListMemories`
- **Search** — `SearchMemories`, `ExtractConceptsFromFile`, `FindSimilarConcepts`
- **Graph** — `Sync`, `BuildContext`, `Visualize`
- **Repair** — `RepairConcepts`, `AnalyzeConceptCorruption`
- **Reasoning** — `SequentialThinking`, `Workflow`, `AssumptionLedger`
- **Pipeline / ACP** — `Pipeline`, `ListAgents`, `AcpExec`
- **Assets** — `Adopt`, `ListAssets`, `UpdateAssets`, `ReadMcpResource`
- **System** — `GetConfig`, `GetHelp`, `RecentActivity`, `MemoryStatus`, `RunFullBenchmark`

Tool reference docs live under [`assets/usage/tools/`](assets/usage/tools/). The canonical registry is [`crates/maenifold-core/src/registry.rs`](crates/maenifold-core/src/registry.rs).

## Assets

The repo ships with dozens of bundled workflows, roles, colors, and perspectives:

- **Workflows** in `assets/workflows/`
- **Roles** in `assets/roles/`
- **Colors** in `assets/colors/`
- **Perspectives** in `assets/perspectives/`

These assets are consumed by the workflow engine, desktop authoring tools, and MCP-facing adoption flows. They are reusable templates for structured reasoning, not just connector configuration.

## Integrations

Bundled integration assets live under [`assets/integrations/`](assets/integrations/):

- [`assets/integrations/claude-code/`](assets/integrations/claude-code/) — Claude Code plugin assets
- [`assets/integrations/copilot/`](assets/integrations/copilot/) — Copilot plugin assets
- [`assets/integrations/opencode/`](assets/integrations/opencode/) — OpenCode agent and plugin assets
- [`assets/integrations/skills/`](assets/integrations/skills/) — reusable skills, including maenifold, gate, ACP, and Pinia support

## What's Next

- **Future direction:** self-evolving pipelines that learn from execution traces and improve how they gather context, route reasoning, and shape the graph over time.
- **Future direction:** move maenifold closer to a no-code environment for agent orchestration, with workflows, pipelines, and the graph working together through shared local state.
- **Future direction:** pipeline-as-evaluation — use fork, graft, and execution trace comparison to benchmark harness quality without a separate eval framework.

## Development

The Rust backend and the desktop app evolve together, but they build separately. Start with the narrowest relevant target:

```bash
cargo build --workspace
make quality-lint
cargo test -p maenifold-core
MAENIFOLD_FULL_SUITE=1 cargo test
```

Desktop work happens under `apps/desktop/` with its own Node/Electron toolchain. See [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the full build, test, and quality workflow.

Release automation currently packages macOS ARM64, Linux x64, and Windows x64. There are no release tags in this repository today, so treat source builds as the supported path.

Issues and pull requests are welcome on [GitHub](https://github.com/msbrettorg/maenifold-rs).

## Troubleshooting

- **`maenifold` not found after build:** The binary is at `./target/release/maenifold`. Either add it to your PATH or use the full path in MCP configs.
- **Pipeline `run` fails with "driver binary not found":** The pipeline executor spawns a separate `maenifold-pipeline-driver` process and expects it next to the `maenifold` binary. Run `./rebuild.sh` to symlink all binaries, or manually: `sudo ln -sf $(pwd)/target/release/maenifold-pipeline-driver /usr/local/bin/maenifold-pipeline-driver`.
- **Pipeline tool nodes abandon immediately:** External MCP tool nodes (e.g. `azure-mcp-server.kusto`) resolve their server config from `maenifold.json`. Ensure the server is listed in the top-level `mcpServers` array and that the pipeline driver binary is installed (see above).
- **SearchMemories returns empty results:** Expected on a fresh install with no memory files. Write a memory first with `WriteMemory`.
- **Desktop dev shell doesn't start Electron:** Run `npm run dev:electron` from `apps/desktop/`. The plain `npm run dev` script starts only the Vite renderer; the Electron host is wired behind the `dev:electron` lifecycle event.
- **MCP client can't spawn the server:** Ensure the `command` in your MCP config points to an absolute path or a binary on your PATH.

## Documentation

- [`docs/README.md`](docs/README.md)
- [`docs/BOOTSTRAP.md`](docs/BOOTSTRAP.md)
- [`docs/SCRIPTING.md`](docs/SCRIPTING.md)
- [`docs/acp-client-architecture.md`](docs/acp-client-architecture.md)
- [`docs/SECURITY_MODEL.md`](docs/SECURITY_MODEL.md)
- [`docs/TESTING_PHILOSOPHY.md`](docs/TESTING_PHILOSOPHY.md)
- [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md)

## License

MIT. Repository: [msbrettorg/maenifold-rs](https://github.com/msbrettorg/maenifold-rs)
