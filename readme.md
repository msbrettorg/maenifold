# maenifold-rs

## purpose
Rust reimplementation of `maenifold` — a knowledge-graph-backed memory system for LLM agents. This repo provides full CLI and MCP tool parity with the original C# implementation.

## current status
<!-- T-0.2: RTM NFR-3 — README reflects current parity state -->
- **Tools**: 28/28 implemented — zero placeholders, all real handlers
- **Tests**: 500+ passing across 90 test suites, 0 failures
- **CLI parity**: tested side-by-side against C# oracle (130+ oracle comparisons)
- **Sprint 15 remediation**: 27 parity issues found and fixed (search NULL handling, BuildContext format, output formatting, error messages, validation gaps)
- **vec0**: native extension loading verified on macOS and Linux
- **Workflow assets**: 69/69 workflow JSON files (full parity with C#)
- **MCP**: resources/list, resources/read, and resource-change notifications implemented

Only 4 minor cosmetic parity items remain (field ordering in RecentActivity, error wording in Adopt, UpdateAssets header format, ExtractConcepts sort order) — all documented in TODO.md under "Polish".

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
├── tests/                     # Workspace-level oracle parity tests
├── assets/                    # Workflow JSONs, native libs, models, integrations
└── docs/                      # DEVELOPMENT.md (build/test/contribution guide)
```

## compatibility notes
### shared storage / db
- Rust matches the C# `concepts` table contract for mixed-runtime sync (`first_seen`, `occurrence_count`)
- WikiLink filter reads `.wikilink-filter.json` (JSON object format, matching C#)

### markdown / wikilinks
- WikiLink extraction excludes code blocks and inline code
- `pulldown-cmark` parses `[[wikilinks]]` as wiki-link events

### output mode parity
These tools return plain text even with `--json` (matches C# behavior):
`Sync`, `BuildContext`, `Workflow`, `AssumptionLedger`, `ExtractConceptsFromFile`, `FindSimilarConcepts`, `StartWatcher`, `StopWatcher`

### vector / vec0
- Dynamic ONNX model discovery at runtime (scans `assets/models/*.onnx`, selects largest)
- Embedding dimensions read from model tensor shape (no hardcoded constants)
- `FindSimilarConcepts` can reuse C#-generated `vec_concepts` data via `vec0` extension
- `vec0` native libraries under `assets/native/`

## test architecture
- **Boundary tests** (`crates/maenifold-core/tests/`): isolated per-tool tests using `TempDir`
- **CLI contract tests** (`crates/maenifold-cli/tests/`): `assert_cmd`-based JSON output and error validation
- **Oracle parity tests** (`tests/`): side-by-side Rust vs C# output comparison using semantic JSON equality
- **196 test files** across 90 test suites

## validated commands
```bash
cargo test --workspace    # 500+ tests, 0 failures
cargo clippy --workspace --all-targets -- -D warnings  # clean
cargo fmt --all --check   # clean
```

## sprint history highlights
| Sprint | Focus |
|--------|-------|
| 1–3 | Implemented 6 remaining placeholder tools to full handlers |
| 9 | Dynamic ONNX model loading, CI/CD workflows |
| 10 | Port parity completion — workflow JSONs, env var rename, MCP resources |
| 11 | Search core fix — occurrence counting, RRF vector fusion |
| 12 | Structural refactor — decomposed search.rs into sub-modules |
| 13 | Parity audit — search mode paths, MemoryStatus, dead code cleanup |
| 14 | Red-team drift remediation — decay, frontmatter, symlinks, session IDs |
| 15 | CLI parity — 27 format fixes validated against C# oracle |

## known practical notes
- `vec0` native libraries exist under `assets/native/`
- Model assets exist under `assets/models/`
- `assets/workflows/` contains 69 workflow JSON files (full C# parity)
- `assets/integrations/claude-code/` ships hook scripts and quick-start guides alongside the binary
- CI runs on push/PR to `dev` and `main` (`.github/workflows/build.yml`)
