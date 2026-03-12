# maenifold-rs

## purpose
Rust implementation of `maenifold` — a knowledge-graph-backed memory system for LLM agents. This repo is self-contained and validated with Rust-native tests.

## current status
<!-- T-0.2: RTM NFR-3 — README reflects current parity state -->
- **Tools**: 28/28 implemented — zero placeholders, all real handlers
- **Tests**: extensive Rust-native boundary, CLI, and workspace integration coverage
- **Validation**: no external oracle or sibling repository required
- **Sprint 15 remediation**: 27 parity issues found and fixed (search NULL handling, BuildContext format, output formatting, error messages, validation gaps)
- **vec0**: native extension loading verified on macOS and Linux
- **Workflow assets**: 69 bundled workflow JSON files
- **MCP**: resources/list, resources/read, and resource-change notifications implemented

Current follow-up work is tracked directly in `TODO.md`.

## implemented tool surface
All 28 registered tools resolve to real handlers:
- `WriteMemory`
- `ReadMemory`
- `EditMemory`
- `DeleteMemory`
- `MoveMemory`
- `SearchMemories`
- `ExtractConceptsFromFile`
- `Sync`
- `BuildContext`
- `Visualize`
- `SequentialThinking`
- `Workflow`
- `RecentActivity`
- `MemoryStatus`
- `GetConfig`
- `GetHelp`
- `ListMemories`
- `UpdateAssets`
- `RepairConcepts`
- `AnalyzeConceptCorruption`
- `FindSimilarConcepts`
- `StartWatcher`
- `StopWatcher`
- `RunFullBenchmark`
- `Adopt`
- `AssumptionLedger`
- `ListAssets`
- `ReadMcpResource`

## workspace structure
```
maenifold-rs/
├── crates/
│   ├── maenifold-core/       # Library crate — all tool logic
│   │   └── src/
│   │       ├── assets/        # Adopt, UpdateAssets, ListAssets, asset manager
│   │       ├── benchmark/     # RunFullBenchmark (5 categories)
│   │       ├── community/     # Louvain community detection, NMI metrics
│   │       ├── db/            # SQLite connection, schema, vec0 extension
│   │       ├── decay/         # Access-decay scoring and metadata
│   │       ├── graph/         # BuildContext, Visualize, graph traversal
│   │       ├── markdown/      # WikiLink extraction, frontmatter, storage
│   │       ├── mcp/           # MCP stdio server, resources/list, resources/read
│   │       ├── memory/        # Write/Read/Edit/Delete/Move/List memory
│   │       ├── repair/        # RepairConcepts, AnalyzeConceptCorruption
│   │       ├── search/        # SearchMemories (Hybrid/Semantic/FullText)
│   │       ├── sessions/      # AssumptionLedger, session management
│   │       ├── similar/       # FindSimilarConcepts, vector queries
│   │       ├── sync/          # Sync, incremental sync, graph state, watchers
│   │       ├── thinking/      # SequentialThinking sessions
│   │       ├── tools/         # RecentActivity, MemoryStatus, GetConfig, GetHelp
│   │       ├── util/          # JSON helpers, path validation
│   │       ├── vector/        # ONNX model discovery, embedding inference
│   │       ├── workflow/      # Workflow engine, queue, submachine
│   │       ├── config.rs
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       └── registry.rs    # Tool dispatch (28 tools → handlers)
│   └── maenifold-cli/         # Binary crate — CLI + MCP entrypoint
├── tests/                     # Workspace-level Rust integration tests
├── assets/                    # Workflow JSONs, native libs, models, integrations
└── docs/                      # DEVELOPMENT.md (build/test/contribution guide)
```

## compatibility notes
### shared storage / db
- `concepts` persists `first_seen` and `occurrence_count`
- WikiLink filter reads `.wikilink-filter.json`

### markdown / wikilinks
- WikiLink extraction excludes code blocks and inline code
- `pulldown-cmark` parses `[[wikilinks]]` as wiki-link events

### output mode notes
These tools return plain text even with `--json`:
`Sync`, `BuildContext`, `Workflow`, `AssumptionLedger`, `ExtractConceptsFromFile`, `FindSimilarConcepts`, `StartWatcher`, `StopWatcher`

### vector / vec0
- Dynamic ONNX model discovery at runtime (scans `assets/models/*.onnx`, selects largest)
- Embedding dimensions read from model tensor shape (no hardcoded constants)
- `vec0` native libraries under `assets/native/`

## test architecture
- **Boundary tests** (`crates/maenifold-core/tests/`): isolated per-tool tests using `TempDir`
- **CLI contract tests** (`crates/maenifold-cli/tests/`): `assert_cmd`-based JSON output and error validation
- **Workspace integration tests** (`tests/`): Rust-native end-to-end coverage across crate boundaries

## validated commands
```bash
cargo test --workspace    # Rust-native workspace validation
cargo clippy --workspace --all-targets -- -D warnings  # clean
cargo fmt --all --check   # clean
```

## sprint history highlights
| Sprint | Focus |
|--------|-------|
| 1–3 | Implemented 6 remaining placeholder tools to full handlers |
| 9 | Dynamic ONNX model loading, CI/CD workflows |
| 10 | Workflow assets, env var cleanup, MCP resources |
| 11 | Search core fix — occurrence counting, RRF vector fusion |
| 12 | Structural refactor — decomposed search.rs into sub-modules |
| 13 | Search mode paths, MemoryStatus, dead code cleanup |
| 14 | Red-team drift remediation — decay, frontmatter, symlinks, session IDs |
| 15 | CLI behavior remediation — 27 format and validation fixes |

## known practical notes
- `vec0` native libraries exist under `assets/native/`
- Model assets exist under `assets/models/`
- `assets/workflows/` contains 69 workflow JSON files
- `assets/integrations/claude-code/` ships hook scripts and quick-start guides alongside the binary
- CI runs on push/PR to `dev` and `main` (`.github/workflows/build.yml`)
