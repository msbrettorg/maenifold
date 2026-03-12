<p align="center">
  <img src="docs/branding/maenifold-logo.svg" alt="maenifold">
</p>

<p align="center">
  Domain expertise that compounds. Open. Local. Yours.
</p>

<p align="center">
  <a href="https://github.com/msbrettorg/maenifold/releases/latest"><img src="https://img.shields.io/github/v/release/msbrettorg/maenifold?style=flat-square" alt="Latest Release"></a>
  <a href="https://github.com/msbrettorg/maenifold/blob/main/LICENSE"><img src="https://img.shields.io/github/license/msbrettorg/maenifold?style=flat-square" alt="MIT License"></a>
</p>

---

**Persistent hierarchical state machines executing across ephemeral agents on a shared cognitive manifold, where each HSM transition produces graph artifacts that reshape the manifold for all future transitions by all threads — human and agent alike.**

Agents think in chains of thought — maenifold captures the important bits as `[[WikiLinks]]`, builds a graph of just those concepts and how they relate, and feeds it back into the context window. The filler is stripped. The signal compounds. You see the same graph in Obsidian. Every agent on your machine is a thread in the same mind.

## Quick Start

```bash
# Build from source
cargo build --release -p maenifold-cli

# Or download from GitHub Releases
# https://github.com/msbrettorg/maenifold/releases/latest
```

**CLI**

```bash
maenifold --tool WriteMemory --payload '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]]"}'
maenifold --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid"}'
maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2}'
```

**MCP** (Claude Code, Claude Desktop, Codex, etc.)

```json
{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}
```

Both interfaces have full feature parity. CLI filters intermediate results and preserves context ([why this matters](https://www.anthropic.com/engineering/code-execution-with-mcp)). MCP auto-syncs the graph during interactive sessions.

## How It Works

**`[[WikiLinks]]` are the primitive.** Each one is a compressed semantic unit — `[[authentication]]`, `[[commitment-discounts]]`, `[[null-reference-exception]]` — carrying meaning in its name alone. When agents tag concepts in their reasoning, those tags become graph nodes. Co-occurring WikiLinks become edges. Structure emerges from use.

**Memory is for humans.** Readable markdown with full prose, citations, and context. Open a file, read it, audit what your agents know.

**The graph is for navigation.** A navigable structure of concept names and relationships — the semantic skeleton of everything agents and humans have learned together, stripped of filler. Community detection clusters reasoning domains. Decay weights surface what's fresh. At session start, the graph is injected into the context window as a concept map — the attentional gate between persistent memory and ephemeral working memory.

**The graph IS the mind's persistent memory.** Not a database the agent queries and hopes for the best. The compressed, clustered, decay-weighted concept map *is* what primes every session. The context window is ephemeral working memory — the graph is what survives. Agents traverse deeper only when they need the full document.

**One graph. Every thread.** You in Obsidian, Claude Code, VS Code, Codex, cron jobs — every interface is a thread of attention in the same mind. What one thread learns, every thread knows. Knowledge compounds across clients, sessions, domains, and time.

### Capabilities

- **Hybrid search** — semantic vectors + full-text with RRF fusion
- **Sequential thinking** — persistent hierarchical state machines with revision, branching; any agent can resume any thought
- **69 workflows** — composable HSMs from deductive reasoning to multi-agent sprints
- **Memory lifecycle** — decay, consolidation, repair modeled on cognitive neuroscience
- **16 roles, 7 thinking colors, 12 perspectives** — composable cognitive assets
- **Community detection** — Louvain algorithm identifies reasoning domains during sync
- **Decay weighting** — ACT-R power-law recency bias across search, context, and similarity

Six layers: WikiLinks → Graph → Search → Session State → Reasoning → Orchestration.

## Platforms

| Platform | Binary | Notes |
|----------|--------|-------|
| macOS | osx-arm64, osx-x64 | Apple Silicon or Intel |
| Linux | linux-x64, linux-arm64 | x64 or ARM64 |
| Windows | win-x64 | x64 only |

Native Rust binary. Vector embeddings via ONNX Runtime. vec0 SQLite extension bundled per platform. No external dependencies.

## Tools

All 28 tools are registered in `registry.rs` and resolve to real handlers.

| Category | Tool | Description |
|----------|------|-------------|
| **Memory** | `WriteMemory` | Create a new memory file with `[[WikiLinks]]` |
| | `ReadMemory` | Read a memory file by URI or title |
| | `EditMemory` | Append, prepend, find-replace, or replace section |
| | `DeleteMemory` | Delete a memory file (requires confirmation) |
| | `MoveMemory` | Rename or relocate a memory file |
| | `ListMemories` | List files and folders under the memory root |
| | `GetHelp` | Retrieve detailed help for a specific tool |
| **Search** | `SearchMemories` | Hybrid/semantic/full-text search across memory files |
| | `ExtractConceptsFromFile` | Extract `[[WikiLink]]` concepts from a file |
| | `FindSimilarConcepts` | Find semantically similar concepts via vector embeddings |
| **Graph** | `BuildContext` | Traverse the concept graph around a node |
| | `Visualize` | Generate a Mermaid diagram of concept relationships |
| | `Sync` | Synchronize WikiLink concepts into the graph database |
| **Repair** | `RepairConcepts` | Replace concept variants with a canonical form |
| | `AnalyzeConceptCorruption` | Find duplicate/corrupted WikiLink variants |
| **Reasoning** | `SequentialThinking` | Structured thinking with branching and persistence |
| | `Workflow` | Run structured workflows with submachine support |
| | `AssumptionLedger` | Track assumptions linked to workflow context |
| **Assets** | `Adopt` | Load a role, color, or perspective as an adoption instruction |
| | `UpdateAssets` | Compare and sync packaged assets to runtime |
| | `ListAssets` | List available asset catalogs |
| | `ReadMcpResource` | Read an MCP resource by URI |
| **System** | `RecentActivity` | Query recently indexed files with optional content |
| | `MemoryStatus` | High-level memory system statistics |
| | `GetConfig` | Display configuration settings and resolved paths |
| | `RunFullBenchmark` | Performance benchmarks across 5 categories |
| | `StartWatcher` | Start background file-system watcher for incremental sync |
| | `StopWatcher` | Stop the background watcher |

## Skills

| Skill | What You Get |
|-------|--------------|
| **[Maenifold](integrations/claude-code/plugin-maenifold/skills/maenifold/)** | 28 tools, 6 composable layers, sequential thinking, 69 workflows |
| **[Product Manager](integrations/claude-code/plugin-product-team/skills/product-manager/)** | Multi-agent orchestration, graph context injection, quality gates, sprint traceability |

## Integrations

| Integration | Purpose |
|-------------|---------|
| **[Claude Code](integrations/claude-code/plugin-maenifold/)** | MCP server, graph-of-thought hooks, skill auto-loading |

## Documentation

- **[Complete Guide](docs/README.md)** — Architecture, philosophy, detailed examples
- **[Bootstrap Guide](docs/BOOTSTRAP.md)** — From empty graph to domain expertise
- **[Scripting Guide](docs/SCRIPTING.md)** — CLI patterns, Graph-RAG, HYDE, FLARE
- **[Context Engineering](docs/research/context-engineering.md)** — Theoretical foundations
- **[Security Model](docs/SECURITY_MODEL.md)** — STRIDE analysis and data flow
- **[Testing Philosophy](docs/TESTING_PHILOSOPHY.md)** — No mocks, real systems, real behavior
- **[Development Guide](docs/DEVELOPMENT.md)** — Build, test, and contribution guide

## Configuration

All settings are read from environment variables with sensible defaults.

| Variable | Default | Description |
|----------|---------|-------------|
| `MAENIFOLD_ROOT` | `~/maenifold` | Root directory for all data |
| `MAENIFOLD_DATABASE_PATH` | `$ROOT/memory.db` | SQLite database location |
| `MAENIFOLD_AUTO_SYNC` | `true` | Enable incremental sync watcher on MCP start |
| `MAENIFOLD_SQLITE_BUSY_TIMEOUT` | `5000` | SQLite busy timeout in milliseconds |
| `MAENIFOLD_SYNC_LEASE_MS` | `10000` | Sync lease duration (1000–60000 ms) |
| `MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY` | `40` | Run OPTIMIZE after N sync cycles |
| `MAENIFOLD_INCREMENTAL_VACUUM_MINUTES` | `720` | Auto-vacuum interval |
| `MAENIFOLD_SESSION_CLEANUP` | `true` | Enable abandoned session cleanup |
| `MAENIFOLD_SESSION_ABANDON_MINUTES` | — | Minutes before a session is considered abandoned |
| `MAENIFOLD_WATCHER_BUFFER` | `65536` | File-system watcher buffer size |
| `MAENIFOLD_DECAY_FUNCTION` | `power-law` | Access-decay scoring function |
| `MAENIFOLD_DECAY_HALF_LIFE_DAYS` | `30` | Half-life for decay scoring |
| `MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT` | `28` | Grace period before decay applies |
| `MAENIFOLD_SYNC_LOGGING` | `true` | Log sync operations |
| `MAENIFOLD_EMBEDDING_LOGS` | `false` | Log embedding inference |
| `MAENIFOLD_VECTOR_LOGS` | `false` | Log vector search queries |

## Development

See [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the full guide.

```bash
cargo build --workspace                                  # build
cargo fmt --all --check                                  # format check
cargo clippy --workspace --all-targets -- -D warnings    # lint
cargo test                                               # full test suite
```

### Workspace Structure

```
maenifold-rs/
├── crates/
│   ├── maenifold-core/       # Library crate — all tool logic (20 modules, ~17.6K LOC)
│   └── maenifold-cli/        # Binary crate — CLI + MCP entrypoint
├── integrations/claude-code/  # Claude Code plugin packages
├── tests/                     # Workspace-level integration tests
├── assets/                    # Workflows, roles, colors, perspectives, models, native libs
└── docs/                      # Development guide
```

CI runs `fmt` → `clippy` → `build` → `test` on every push/PR to `dev` and `main`. Releases are built for macOS ARM64, Linux x64, and Windows x64 on `v*` tags.

## License

MIT. Contributions welcome at [github.com/msbrettorg/maenifold](https://github.com/msbrettorg/maenifold).
