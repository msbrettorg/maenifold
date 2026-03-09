# Product Requirements Document — maenifold-rs Tool Parity

## Overview
Complete the Rust reimplementation of maenifold by implementing the 6 remaining placeholder tools to full C# oracle parity.

## Functional Requirements

### FR-1: Adopt Tool
The system SHALL provide an `Adopt` tool that reads a role, color, or perspective JSON asset by type and identifier, returning the full asset content formatted as an LLM adoption instruction.
- **Parameters**: `type` (role|color|perspective), `identifier` (asset ID), `learn` (bool)
- **Behavior**: Two-step lookup — fast path by filename, fallback by scanning JSON `id` fields. Returns structured adoption prompt with embedded JSON.
- **Parity**: Output SHALL match C# `AdoptTools.cs` (99 lines)

### FR-2: AnalyzeConceptCorruption Tool
The system SHALL provide an `AnalyzeConceptCorruption` tool that scans all memory files for WikiLink concept variants matching a given concept family, classifies them by pattern type, and suggests repair commands.
- **Parameters**: `conceptFamily` (string), `maxResults` (int, default 50), `learn` (bool)
- **Behavior**: Scans WikiLinks, normalizes concepts, checks containment of family string. Classifies variants into: File Paths, Singular/Plural, Compound (hyphenated), With Suffix, Other. Generates suggested `RepairConcepts` commands for safe merges.
- **Parity**: Output SHALL match C# `ConceptAnalyzer.cs` (~140 lines)

### FR-3: UpdateAssets Tool
The system SHALL provide an `UpdateAssets` tool that compares packaged assets against runtime assets, identifying added/updated files, with optional dry-run mode.
- **Parameters**: `dryRun` (bool, default true), `learn` (bool)
- **Behavior**: Recursive directory comparison using file size + SHA256 hash. In non-dry-run mode, copies changed files. Returns markdown summary with counts and file lists.
- **Parity**: Output SHALL match C# `AssetManager.cs` update logic (~165 lines)

### FR-4: RecentActivity Tool
The system SHALL provide a `RecentActivity` tool that queries the `file_content` database table for recently indexed files, filtered by type and timespan, with optional content inclusion.
- **Parameters**: `limit` (int, default 10), `filter` (thinking|memory|assumptions|all), `timespan` (duration string), `includeContent` (bool), `learn` (bool)
- **Behavior**: Queries `file_content` table ordered by `last_indexed DESC`. Classifies files by path pattern (sequential/workflow/thinking/memory). Extracts H2 sections from content. Formats activity report with thought/step counts and snippets.
- **Parity**: Output SHALL match C# `RecentActivityTools.cs` + `RecentActivity.Reader.cs` + `RecentActivity.Formatter.cs` (~297 lines total)

### FR-5: RunFullBenchmark Tool
The system SHALL provide a `RunFullBenchmark` tool that measures performance across 5 benchmark categories: CTE vs N+1 graph traversal, search (3 modes), sync, complex traversal, and system health.
- **Parameters**: `iterations` (int, default 5), `maxTestFiles` (int, default 1000), `includeDeepTraversal` (bool, default true), `learn` (bool)
- **Behavior**: Runs timed iterations of each benchmark using `std::time::Instant`. Reports min/max/avg/median timings. System health reports DB table counts.
- **Parity**: Output SHALL match C# `PerformanceBenchmark.cs` + `PerformanceBenchmark.Graph.cs` (~365 lines total)

### FR-6: RepairConcepts Tool
The system SHALL provide a `RepairConcepts` tool that replaces concept variants with a canonical form across all memory files, with two modes: WikiLink replacement and WikiLink creation, plus semantic safety validation.
- **Parameters**: `conceptsToReplace` (comma/semicolon/pipe-separated), `canonicalConcept` (string), `folder` (optional), `dryRun` (bool, default true), `createWikiLinks` (bool, default false), `minSemanticSimilarity` (float, default 0.7), `learn` (bool)
- **Behavior Mode A (replacement)**: Regex-find `[[variant]]` WikiLinks, replace with `[[canonical]]`. **Mode B (creation)**: AST-walk markdown paragraphs/headings/lists, find plain-text variant mentions, wrap in `[[canonical]]`, skip code blocks/existing links. **Safety**: If `minSemanticSimilarity > 0`, query vec0 embeddings for cosine similarity; block if any variant below threshold.
- **Parity**: Output SHALL match C# `ConceptRepairTool.cs` (~639 lines)

### FR-7: Dynamic Model Loading
The system SHALL discover ONNX embedding models at runtime by scanning `assets/models/*.onnx`, selecting the largest file when multiple models are present.
- **Discovery**: Glob `*.onnx` in assets/models, sort by file size descending, load first match
- **Dimensions**: Read output tensor shape from loaded ONNX session — last dimension = embedding size (no hardcoded constant)
- **Tokenizer**: Prefer `tokenizer.json` (HuggingFace standard) in same directory; fall back to `vocab.txt` + `tokenizer_config.json` (BERT WordPiece manual construction)
- **Max tokens**: Read `model_max_length` from tokenizer config; default 512 if absent
- **No model**: Graceful degradation — all non-semantic features work, embedding calls return `Err`
- **Cross-compat**: Must work with models placed by C# runtime sharing same `~/maenifold/assets/models/` directory

### FR-8: Dynamic Vec0 Schema
The system SHALL adapt vec0 virtual table dimensions to match the loaded model's embedding size.
- **Metadata**: Store `(model_filename, embedding_dimensions)` in a `model_metadata` table
- **Change detection**: On model load, compare current model against stored metadata
- **Migration**: If dimensions changed, drop and recreate `vec_concepts` and `vec_memory_files` tables with new `FLOAT[N]` dimension, then trigger full embedding re-sync
- **No model**: Skip vec0 table creation entirely

### FR-9: CI/CD Pipeline
The system SHALL have GitHub Actions workflows for continuous integration and multi-platform releases.
- **build.yml**: Trigger on push/PR to `dev` and `main`. Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` on ubuntu-latest
- **release.yml**: Trigger on `v*` tags. Matrix build for `osx-arm64`, `linux-x64`, `win-x64`. Produce ZIP artifacts with binary + assets (excluding ONNX model). Upload to GitHub Releases

## Non-Functional Requirements

### NFR-1: C# Oracle Parity
All 6 tools SHALL produce semantically equivalent JSON output to the C# oracle when invoked with identical parameters against identical data, verified by the existing parity test framework.

### NFR-2: Test Coverage
Each tool SHALL have tests at all 3 layers: boundary (in-process), CLI (binary), and parity (oracle comparison).

### NFR-3: No Regressions
Implementation SHALL NOT break any existing passing tests. `cargo test` SHALL remain green.

### NFR-4: Code Quality
All code SHALL pass `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`.

---

## Sprint 10 — Port Parity Completion

### FR-10: Workflow Asset Parity
The system SHALL ship the same set of workflow JSON files as the C# reference implementation.
- **Gap**: Rust has 41 workflows; C# has 69. 28 scientific-method-family workflows are absent.
- **Behavior**: `Workflow` tool SHALL find and execute any workflow available in the C# release.

### FR-11: MCP Resource Parity
The system SHALL expose native MCP `resources` capability matching the C# server.
- **resources/list**: Enumerate asset catalogs (workflows, roles, colors, perspectives)
- **resources/read**: Read individual asset by `asset://{type}/{id}` URI
- **notifications/resources/list_changed**: Emit when asset files change on disk
- **Parity**: Match C# `AssetResources.cs` (197 lines) + `AssetWatcherTools.cs` notification path

### FR-12: MCP Auto-Start Watchers
The `--mcp` CLI mode SHALL auto-start the incremental sync watcher and asset watcher on startup, matching C# `Program.cs` lines 33-41.

### FR-13: Oracle Parity Tests for All Tools
Every tool in the registry SHALL have oracle-comparison tests in `tests/compat/` verifying output matches the C# binary.
- **Gap**: Adopt, AnalyzeConceptCorruption, UpdateAssets, RecentActivity, RunFullBenchmark, RepairConcepts have boundary/CLI tests but no oracle comparison tests.

### FR-14: CI Parity Gate
`build.yml` SHALL run `cargo test` at workspace root (not just `-p maenifold-core -p maenifold-cli`), ensuring oracle parity tests are gated in CI.

### FR-15: Config Env Var Parity
The system SHALL read all C# configuration environment variables with matching names and defaults.
- **Naming**: `MAENIFOLD_SESSION_ABANDON_MINUTES` (not `ABANDONMENT`), `MAENIFOLD_AUTO_SYNC` (not `ENABLE_INCREMENTAL_SYNC`)
- **Missing vars**: `MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY`, `MAENIFOLD_INCREMENTAL_VACUUM_MINUTES`, `MAENIFOLD_SESSION_CLEANUP`, `MAENIFOLD_SYNC_LOGGING`, `MAENIFOLD_EMBEDDING_LOGS`, `MAENIFOLD_VECTOR_LOGS`, `MAENIFOLD_WATCHER_BUFFER`, `MAENIFOLD_SQLITE_BUSY_TIMEOUT` (as runtime env var, not const)

### FR-16: Stale Documentation Remediation
- `readme.md` SHALL be updated to reflect 28/28 implemented tools and current status
- `TODO.md` status fields SHALL be updated from `pending` to `done` for completed items
- A `docs/` directory SHALL exist with at minimum: DEVELOPMENT.md (Rust build/test guide)

### FR-17: RepairConcepts Mode B Verification
RepairConcepts boundary tests SHALL NOT accept `"not yet implemented"` as a passing condition. Mode B (WikiLink creation), semantic validation, code-block exclusion, and double-wrap prevention SHALL be verified with real assertions.

### FR-18: Sprint 9 RTM Coverage
Tests SHALL exist for RTM-052 (tokenizer.json preferred), RTM-054 (max tokens default 512), RTM-058 (vec0 runtime dimensions), RTM-059 (vec0 recreation on dimension change).

### FR-19: WikiLink Filter Format Parity
The system SHALL read `.wikilink-filter.json` (JSON format) matching C# `WikiLinkFilter.cs`, not `.wikilink-filter` (line-based text). A shared `~/maenifold/memory/` root MUST behave identically under both runtimes.
- **C# format**: `.wikilink-filter.json` — JSON object where keys are concept names and values are block reasons
- **Rust current**: `.wikilink-filter` — line-based text with `concept|reason` per line
- **Migration**: Read JSON format as primary. Optionally fall back to line-based format for backward compat, then deprecate.

### FR-20: Asset Catalog Isolation
`preferred_catalog_assets_root()` in `assets.rs` SHALL NOT fall through to a sibling C# repo's assets. The function either returns the Rust repo's own assets or `None`. Cross-repo masking during development/testing must be eliminated so missing Rust assets surface as real errors.
