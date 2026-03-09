# Backlog — maenifold-rs Tool Parity

## Sprint 1 — Easy Wins (3 parallel pipelines)

### T-1.1: Implement Adopt handler | RTM: FR-1
Replace placeholder in `registry.rs` with real handler. Read asset by type+identifier using existing `AssetKind::parse` and `AssetManager::read_asset`. Two-step lookup: fast path by filename, fallback by scanning JSON `id` fields. Return structured LLM adoption instruction with embedded JSON. Match C# `AdoptTools.cs`.
- **Files**: `crates/maenifold-core/src/tools.rs` (or `assets.rs`), `crates/maenifold-core/src/registry.rs`
- **Status**: pending

### T-1.2: Implement AnalyzeConceptCorruption handler | RTM: FR-2
Replace placeholder in `registry.rs` with real handler. Scan all memory files for WikiLinks matching concept family. Classify variants into pattern types (file paths, singular/plural, compound, suffix, other). Generate suggested RepairConcepts commands. Match C# `ConceptAnalyzer.cs`.
- **Files**: `crates/maenifold-core/src/search.rs`, `crates/maenifold-core/src/registry.rs`
- **Status**: pending

### T-1.3: Implement UpdateAssets handler | RTM: FR-3
Replace placeholder in `registry.rs` with real handler. Extend `AssetManager` with comparison logic: recursive directory walk, file size + SHA256 hash comparison, optional file copy. Return markdown summary with added/updated file lists. Match C# `AssetManager.cs` update logic.
- **Files**: `crates/maenifold-core/src/assets.rs`, `crates/maenifold-core/src/registry.rs`
- **Status**: pending

### T-1.4: Boundary tests for Sprint 1 tools | RTM: FR-1, FR-2, FR-3
Write boundary tests for Adopt, AnalyzeConceptCorruption, UpdateAssets in `crates/maenifold-core/tests/`. Use `isolated_config()` pattern with TempDir. Cover TC-1.1 through TC-1.7, TC-2.1 through TC-2.6, TC-3.1 through TC-3.6.
- **Files**: `crates/maenifold-core/tests/` (new or existing test files)
- **Status**: pending
- **Depends on**: T-1.1, T-1.2, T-1.3

### T-1.5: CLI tests for Sprint 1 tools | RTM: FR-1, FR-2, FR-3
Write CLI contract tests for Adopt, AnalyzeConceptCorruption, UpdateAssets using `assert_cmd`. Verify JSON output format, error handling, exit codes.
- **Files**: `crates/maenifold-cli/tests/system_tools_cli_tests.rs` (or new files)
- **Status**: pending
- **Depends on**: T-1.1, T-1.2, T-1.3

### T-1.6: Parity tests for Sprint 1 tools | RTM: FR-1, FR-2, FR-3, NFR-1
Write parity tests comparing Rust output to C# oracle for Adopt, AnalyzeConceptCorruption, UpdateAssets. Use IsolatedRoot sandbox + semantic JSON equality.
- **Files**: `tests/compat/` (new parity test file)
- **Status**: pending
- **Depends on**: T-1.4, T-1.5

## Sprint 2 — Medium Complexity (2 parallel pipelines)

### T-2.1: Implement RecentActivity handler | RTM: FR-4
Replace placeholder in `registry.rs` with real handler. Add H2 section extraction helper to `markdown.rs`. Query `file_content` table with type filtering and timespan. Classify files by path pattern. Format activity report with thought/step counts and snippets. Match C# `RecentActivityTools.cs` + `RecentActivity.Reader.cs` + `RecentActivity.Formatter.cs`.
- **Files**: `crates/maenifold-core/src/tools.rs`, `crates/maenifold-core/src/markdown.rs`, `crates/maenifold-core/src/registry.rs`
- **Status**: pending

### T-2.2: Implement RunFullBenchmark handler | RTM: FR-5
Replace placeholder in `registry.rs` with real handler. Implement 5 benchmark categories using `std::time::Instant`: CTE vs N+1 graph traversal, hybrid/semantic/fulltext search, sync, complex traversal (depth:5 maxNodes:200), system health (DB stats). Report min/max/avg/median. Match C# `PerformanceBenchmark.cs` + `PerformanceBenchmark.Graph.cs`.
- **Files**: `crates/maenifold-core/src/tools.rs` (or new `benchmark.rs`), `crates/maenifold-core/src/registry.rs`
- **Status**: pending

### T-2.3: Boundary + CLI tests for Sprint 2 tools | RTM: FR-4, FR-5
Write boundary and CLI tests for RecentActivity and RunFullBenchmark. Cover TC-4.1 through TC-4.8, TC-5.1 through TC-5.6.
- **Files**: `crates/maenifold-core/tests/`, `crates/maenifold-cli/tests/`
- **Status**: pending
- **Depends on**: T-2.1, T-2.2

### T-2.4: Parity tests for Sprint 2 tools | RTM: FR-4, NFR-1
Write parity tests for RecentActivity. RunFullBenchmark is timing-dependent so parity testing focuses on output structure, not values.
- **Files**: `tests/compat/`
- **Status**: pending
- **Depends on**: T-2.3

## Sprint 3 — Complex (1 focused pipeline)

### T-3.1: Implement RepairConcepts handler | RTM: FR-6
Replace placeholder in `registry.rs` with real handler. Implement two modes: (A) WikiLink replacement via regex `\[\[variant\]\]` → `[[canonical]]`, (B) WikiLink creation via pulldown-cmark AST walking — find plain-text mentions in paragraphs/headings/lists, skip code blocks/existing links, wrap in `[[canonical]]`. Implement semantic validation by querying vec0 embeddings for cosine similarity (graceful degradation if vec0 unavailable). Match C# `ConceptRepairTool.cs`.
- **Files**: `crates/maenifold-core/src/search.rs`, `crates/maenifold-core/src/registry.rs`
- **Status**: pending

### T-3.2: Tests for RepairConcepts | RTM: FR-6, NFR-1
Write boundary, CLI, and parity tests for RepairConcepts. Cover TC-6.1 through TC-6.9. Special focus on AST-aware code block exclusion, nested bracket prevention, and semantic validation blocking.
- **Files**: `crates/maenifold-core/tests/`, `crates/maenifold-cli/tests/`, `tests/compat/`
- **Status**: pending
- **Depends on**: T-3.1

## Cross-Sprint

### T-0.1: Verify baseline — cargo test green | RTM: NFR-3
Run `cargo test` to confirm baseline is green before any changes.
- **Status**: pending

### T-0.3: Fix pre-existing clippy errors | RTM: NFR-4
Fix 23 pre-existing clippy errors found under `cargo clippy --workspace --all-targets -- -D warnings`. Use `cargo clippy --fix` where possible. Also: make `normalize_concept` pub in `markdown.rs` (blocks FR-2, FR-6) and add `extract_h2_sections` helper to `markdown.rs` (blocks FR-4).
- **Files**: `crates/maenifold-core/src/markdown.rs`, `crates/maenifold-core/src/sessions.rs`, and others as identified by clippy
- **Status**: pending

### T-0.2: Update readme.md and registry after all tools complete | RTM: NFR-3
Remove "still placeholder" section from readme.md. Verify all 28 registry entries point to real handlers.
- **Status**: pending
- **Depends on**: T-1.6, T-2.4, T-3.2

---

## Sprint 9 — Dynamic Model Loading + On-Demand Download + CI/CD

### T-9.1: Dynamic ONNX model discovery in vector.rs | RTM: FR-7
Replace hardcoded `MODEL_FILENAME`, `EMBEDDING_DIMENSIONS`, and `MAX_TOKENS` constants with runtime discovery. Scan `assets/models/*.onnx`, sort by file size DESC, load largest. Read output tensor shape from ONNX session to determine embedding dimensions. Read `model_max_length` from tokenizer config for max tokens. Store dimensions in `ModelState`. Prefer `tokenizer.json` (HuggingFace standard) over manual `vocab.txt`+`tokenizer_config.json` construction. Export a `fn embedding_dimensions() -> Option<usize>` for other modules.
- **Files**: `crates/maenifold-core/src/vector.rs`
- **Status**: pending

### T-9.2: Dynamic vec0 schema in db.rs | RTM: FR-8
Add `model_metadata` table: `CREATE TABLE IF NOT EXISTS model_metadata(key TEXT PRIMARY KEY, value TEXT)`. On model load, store `model_filename` and `embedding_dimensions`. On subsequent loads, compare stored dimensions vs current model. If changed, DROP and recreate `vec_concepts` and `vec_memory_files` with new `FLOAT[N]`. Use `vector::embedding_dimensions()` to get current dimensions instead of hardcoded 384. If no model loaded, skip vec0 table creation.
- **Files**: `crates/maenifold-core/src/db.rs`
- **Status**: pending
- **Depends on**: T-9.1

### T-9.3: Update search.rs for runtime dimensions | RTM: FR-7, FR-8
Replace all references to `vector::EMBEDDING_DIMENSIONS` constant with calls to `vector::embedding_dimensions()`. Update byte-length validation in `generate_concept_embeddings` and `generate_file_embedding`. Update the hardcoded `384` in degraded diagnostics output string. Ensure graceful degradation when no model is loaded.
- **Files**: `crates/maenifold-core/src/search.rs`
- **Status**: pending
- **Depends on**: T-9.1

### T-9.4: CI workflow — build.yml | RTM: FR-9
Create `.github/workflows/build.yml`. Trigger: push/PR to `dev` and `main`. Runner: `ubuntu-latest`. Steps: checkout, install Rust stable, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`. Cache cargo registry + target.
- **Files**: `.github/workflows/build.yml`
- **Status**: pending

### T-9.5: Release workflow — release.yml | RTM: FR-9
Create `.github/workflows/release.yml`. Trigger: push tag `v*`. Matrix: `[{os: macos-latest, target: aarch64-apple-darwin}, {os: ubuntu-latest, target: x86_64-unknown-linux-gnu}, {os: windows-latest, target: x86_64-pc-windows-msvc}]`. Steps: checkout, install Rust + target, `cargo build --release --target $TARGET`, package binary + assets/ (excluding *.onnx) into ZIP, create GitHub Release with all ZIPs. Match C# release.yml structure.
- **Files**: `.github/workflows/release.yml`
- **Status**: pending

### T-9.6: Tests for dynamic model loading | RTM: FR-7, FR-8
Boundary tests: model discovery picks largest *.onnx, dimensions read from model, tokenizer.json preferred, fallback to vocab.txt, no model = graceful degradation, vec0 recreation on dimension change.
- **Files**: `crates/maenifold-core/tests/`
- **Status**: pending
- **Depends on**: T-9.1, T-9.2, T-9.3

### T-9.7: Red-team audit Sprint 9 | RTM: all Sprint 9
Audit all Sprint 9 changes for: correctness (dimension mismatch handling, vec0 recreation atomicity), cross-compat (C# runtime sharing same model directory), model discovery edge cases (empty dir, corrupt onnx, zero-byte files).
- **Status**: pending
- **Depends on**: T-9.6
