# Backlog — maenifold-rs Tool Parity

## Sprint 1 — Easy Wins (3 parallel pipelines)

### T-1.1: Implement Adopt handler | RTM: FR-1
Replace placeholder in `registry.rs` with real handler. Read asset by type+identifier using existing `AssetKind::parse` and `AssetManager::read_asset`. Two-step lookup: fast path by filename, fallback by scanning JSON `id` fields. Return structured LLM adoption instruction with embedded JSON. Match C# `AdoptTools.cs`.
- **Files**: `crates/maenifold-core/src/tools.rs` (or `assets.rs`), `crates/maenifold-core/src/registry.rs`
- **Status**: done

### T-1.2: Implement AnalyzeConceptCorruption handler | RTM: FR-2
Replace placeholder in `registry.rs` with real handler. Scan all memory files for WikiLinks matching concept family. Classify variants into pattern types (file paths, singular/plural, compound, suffix, other). Generate suggested RepairConcepts commands. Match C# `ConceptAnalyzer.cs`.
- **Files**: `crates/maenifold-core/src/search.rs`, `crates/maenifold-core/src/registry.rs`
- **Status**: done

### T-1.3: Implement UpdateAssets handler | RTM: FR-3
Replace placeholder in `registry.rs` with real handler. Extend `AssetManager` with comparison logic: recursive directory walk, file size + SHA256 hash comparison, optional file copy. Return markdown summary with added/updated file lists. Match C# `AssetManager.cs` update logic.
- **Files**: `crates/maenifold-core/src/assets.rs`, `crates/maenifold-core/src/registry.rs`
- **Status**: done

### T-1.4: Boundary tests for Sprint 1 tools | RTM: FR-1, FR-2, FR-3
Write boundary tests for Adopt, AnalyzeConceptCorruption, UpdateAssets in `crates/maenifold-core/tests/`. Use `isolated_config()` pattern with TempDir. Cover TC-1.1 through TC-1.7, TC-2.1 through TC-2.6, TC-3.1 through TC-3.6.
- **Files**: `crates/maenifold-core/tests/` (new or existing test files)
- **Status**: done
- **Depends on**: T-1.1, T-1.2, T-1.3

### T-1.5: CLI tests for Sprint 1 tools | RTM: FR-1, FR-2, FR-3
Write CLI contract tests for Adopt, AnalyzeConceptCorruption, UpdateAssets using `assert_cmd`. Verify JSON output format, error handling, exit codes.
- **Files**: `crates/maenifold-cli/tests/system_tools_cli_tests.rs` (or new files)
- **Status**: done
- **Depends on**: T-1.1, T-1.2, T-1.3

### T-1.6: Parity tests for Sprint 1 tools | RTM: FR-1, FR-2, FR-3, NFR-1
Write parity tests comparing Rust output to C# oracle for Adopt, AnalyzeConceptCorruption, UpdateAssets. Use IsolatedRoot sandbox + semantic JSON equality.
- **Files**: `tests/compat/system_tools_parity.rs`, `tests/compat/concept_repair_parity.rs`
- **Status**: done (tests written; `#[ignore]` gated on C# oracle availability)
- **Depends on**: T-1.4, T-1.5

## Sprint 2 — Medium Complexity (2 parallel pipelines)

### T-2.1: Implement RecentActivity handler | RTM: FR-4
Replace placeholder in `registry.rs` with real handler. Add H2 section extraction helper to `markdown.rs`. Query `file_content` table with type filtering and timespan. Classify files by path pattern. Format activity report with thought/step counts and snippets. Match C# `RecentActivityTools.cs` + `RecentActivity.Reader.cs` + `RecentActivity.Formatter.cs`.
- **Files**: `crates/maenifold-core/src/tools.rs`, `crates/maenifold-core/src/markdown.rs`, `crates/maenifold-core/src/registry.rs`
- **Status**: done

### T-2.2: Implement RunFullBenchmark handler | RTM: FR-5
Replace placeholder in `registry.rs` with real handler. Implement 5 benchmark categories using `std::time::Instant`: CTE vs N+1 graph traversal, hybrid/semantic/fulltext search, sync, complex traversal (depth:5 maxNodes:200), system health (DB stats). Report min/max/avg/median. Match C# `PerformanceBenchmark.cs` + `PerformanceBenchmark.Graph.cs`.
- **Files**: `crates/maenifold-core/src/tools.rs` (or new `benchmark.rs`), `crates/maenifold-core/src/registry.rs`
- **Status**: done

### T-2.3: Boundary + CLI tests for Sprint 2 tools | RTM: FR-4, FR-5
Write boundary and CLI tests for RecentActivity and RunFullBenchmark. Cover TC-4.1 through TC-4.8, TC-5.1 through TC-5.6.
- **Files**: `crates/maenifold-core/tests/`, `crates/maenifold-cli/tests/`
- **Status**: done
- **Depends on**: T-2.1, T-2.2

### T-2.4: Parity tests for Sprint 2 tools | RTM: FR-4, NFR-1
Write parity tests for RecentActivity. RunFullBenchmark is timing-dependent so parity testing focuses on output structure, not values.
- **Files**: `tests/compat/system_tools_parity.rs`
- **Status**: done (tests written; `#[ignore]` gated on C# oracle availability)
- **Depends on**: T-2.3

## Sprint 3 — Complex (1 focused pipeline)

### T-3.1: Implement RepairConcepts handler | RTM: FR-6
Replace placeholder in `registry.rs` with real handler. Implement two modes: (A) WikiLink replacement via regex `\[\[variant\]\]` → `[[canonical]]`, (B) WikiLink creation via pulldown-cmark AST walking — find plain-text mentions in paragraphs/headings/lists, skip code blocks/existing links, wrap in `[[canonical]]`. Implement semantic validation by querying vec0 embeddings for cosine similarity (graceful degradation if vec0 unavailable). Match C# `ConceptRepairTool.cs`.
- **Files**: `crates/maenifold-core/src/search.rs`, `crates/maenifold-core/src/registry.rs`
- **Status**: done

### T-3.2: Tests for RepairConcepts | RTM: FR-6, NFR-1
Write boundary, CLI, and parity tests for RepairConcepts. Cover TC-6.1 through TC-6.9. Special focus on AST-aware code block exclusion, nested bracket prevention, and semantic validation blocking.
- **Files**: `crates/maenifold-core/tests/repair_concepts_boundary.rs`, `crates/maenifold-cli/tests/`, `tests/compat/concept_repair_parity.rs`
- **Status**: done (boundary+CLI tests pass; parity tests `#[ignore]` gated on C# oracle)
- **Depends on**: T-3.1

## Cross-Sprint

### T-0.1: Verify baseline — cargo test green | RTM: NFR-3
Run `cargo test` to confirm baseline is green before any changes.
- **Status**: done

### T-0.3: Fix pre-existing clippy errors | RTM: NFR-4
Fix 23 pre-existing clippy errors found under `cargo clippy --workspace --all-targets -- -D warnings`. Use `cargo clippy --fix` where possible. Also: make `normalize_concept` pub in `markdown.rs` (blocks FR-2, FR-6) and add `extract_h2_sections` helper to `markdown.rs` (blocks FR-4).
- **Files**: `crates/maenifold-core/src/markdown.rs`, `crates/maenifold-core/src/sessions.rs`, and others as identified by clippy
- **Status**: done

### T-0.2: Update readme.md and registry after all tools complete | RTM: NFR-3
Remove "still placeholder" section from readme.md. Verify all 28 registry entries point to real handlers.
- **Status**: done

---

## Sprint 9 — Dynamic Model Loading + On-Demand Download + CI/CD

### T-9.1: Dynamic ONNX model discovery in vector.rs | RTM: FR-7
Replace hardcoded `MODEL_FILENAME`, `EMBEDDING_DIMENSIONS`, and `MAX_TOKENS` constants with runtime discovery. Scan `assets/models/*.onnx`, sort by file size DESC, load largest. Read output tensor shape from ONNX session to determine embedding dimensions. Read `model_max_length` from tokenizer config for max tokens. Store dimensions in `ModelState`. Prefer `tokenizer.json` (HuggingFace standard) over manual `vocab.txt`+`tokenizer_config.json` construction. Export a `fn embedding_dimensions() -> Option<usize>` for other modules.
- **Files**: `crates/maenifold-core/src/vector.rs`
- **Status**: done

### T-9.2: Dynamic vec0 schema in db.rs | RTM: FR-8
Add `model_metadata` table: `CREATE TABLE IF NOT EXISTS model_metadata(key TEXT PRIMARY KEY, value TEXT)`. On model load, store `model_filename` and `embedding_dimensions`. On subsequent loads, compare stored dimensions vs current model. If changed, DROP and recreate `vec_concepts` and `vec_memory_files` with new `FLOAT[N]`. Use `vector::embedding_dimensions()` to get current dimensions instead of hardcoded 384. If no model loaded, skip vec0 table creation.
- **Files**: `crates/maenifold-core/src/db.rs`
- **Status**: done
- **Depends on**: T-9.1

### T-9.3: Update search.rs for runtime dimensions | RTM: FR-7, FR-8
Replace all references to `vector::EMBEDDING_DIMENSIONS` constant with calls to `vector::embedding_dimensions()`. Update byte-length validation in `generate_concept_embeddings` and `generate_file_embedding`. Update the hardcoded `384` in degraded diagnostics output string. Ensure graceful degradation when no model is loaded.
- **Files**: `crates/maenifold-core/src/search.rs`
- **Status**: done
- **Depends on**: T-9.1

### T-9.4: CI workflow — build.yml | RTM: FR-9
Create `.github/workflows/build.yml`. Trigger: push/PR to `dev` and `main`. Runner: `ubuntu-latest`. Steps: checkout, install Rust stable, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`. Cache cargo registry + target.
- **Files**: `.github/workflows/build.yml`
- **Status**: done

### T-9.5: Release workflow — release.yml | RTM: FR-9
Create `.github/workflows/release.yml`. Trigger: push tag `v*`. Matrix: `[{os: macos-latest, target: aarch64-apple-darwin}, {os: ubuntu-latest, target: x86_64-unknown-linux-gnu}, {os: windows-latest, target: x86_64-pc-windows-msvc}]`. Steps: checkout, install Rust + target, `cargo build --release --target $TARGET`, package binary + assets/ (excluding *.onnx) into ZIP, create GitHub Release with all ZIPs. Match C# release.yml structure.
- **Files**: `.github/workflows/release.yml`
- **Status**: done

### T-9.6: Tests for dynamic model loading | RTM: FR-7, FR-8
Boundary tests: model discovery picks largest *.onnx, dimensions read from model, tokenizer.json preferred, fallback to vocab.txt, no model = graceful degradation, vec0 recreation on dimension change.
- **Files**: `crates/maenifold-core/tests/`
- **Status**: done
- **Depends on**: T-9.1, T-9.2, T-9.3

### T-9.7: Red-team audit Sprint 9 | RTM: all Sprint 9
Audit all Sprint 9 changes for: correctness (dimension mismatch handling, vec0 recreation atomicity), cross-compat (C# runtime sharing same model directory), model discovery edge cases (empty dir, corrupt onnx, zero-byte files).
- **Status**: obsolete (superseded by Sprint 14 red-team remediation which addressed all critical findings)
- **Depends on**: T-9.6

---

## Sprint 10 — Port Parity Completion

### T-10.1: Copy 28 missing workflow JSONs | RTM: FR-10
Copy all workflow JSON files from `../maenifold/src/assets/workflows/` that are absent in `assets/workflows/`. Missing: bayesian-reasoning, communication-iteration, confirmation-bias, confirmation-replication, conjecture-refutation, controlled-experiment, crucial-experiment, double-blind-study, empiricism, experimental-design, falsification, hypothesis-development, hypothetico-deductive, mathematical-modeling, model-dependent-realism, occams-razor, paradigm-analysis, peer-review, pragmatism, prediction, rationalism, reproducibility, scientific-method, statistical-analysis, strong-inference, systematic-observation, theory-ladenness, thought-experiment.
- **Files**: `assets/workflows/`
- **Status**: done

### T-10.2: Add MCP resources capability | RTM: FR-11
Extend `mcp.rs` MCP server to advertise `resources` capability. Implement `resources/list` and `resources/read`. Match C# `AssetResources.cs` behavior.
- **Files**: `crates/maenifold-core/src/mcp.rs`, `crates/maenifold-core/src/mcp/resources.rs`
- **Status**: done

### T-10.3: Add MCP resource-change notifications | RTM: FR-11
Implement asset directory watcher that emits `notifications/resources/list_changed` MCP notifications when JSON asset files change on disk.
- **Files**: `crates/maenifold-core/src/mcp.rs`, `crates/maenifold-core/src/mcp/resources.rs`
- **Status**: done
- **Depends on**: T-10.2

### T-10.4: Auto-start watchers in --mcp mode | RTM: FR-12
Update `cli.rs` `--mcp` code path to auto-start the incremental sync watcher and asset watcher on startup, matching C# `Program.cs` lines 33-41.
- **Files**: `crates/maenifold-core/src/cli.rs`
- **Status**: done
- **Depends on**: T-10.3

### T-10.5: Oracle parity tests for Adopt + UpdateAssets | RTM: FR-13
Add oracle-comparison parity tests in `tests/compat/` for Adopt and UpdateAssets.
- **Files**: `tests/compat/system_tools_parity.rs`
- **Status**: done (tests written; `#[ignore]` gated on C# oracle availability)

### T-10.6: Oracle parity tests for RecentActivity + RunFullBenchmark | RTM: FR-13
Add oracle-comparison parity tests for RecentActivity (full) and RunFullBenchmark (structure-only).
- **Files**: `tests/compat/system_tools_parity.rs`
- **Status**: done (tests written; `#[ignore]` gated on C# oracle availability)

### T-10.7: Oracle parity tests for RepairConcepts + AnalyzeConceptCorruption | RTM: FR-13
Add oracle-comparison parity tests for RepairConcepts (Mode A + Mode B) and AnalyzeConceptCorruption.
- **Files**: `tests/compat/concept_repair_parity.rs`
- **Status**: done (tests written; `#[ignore]` gated on C# oracle availability)

### T-10.8: Fix CI to run workspace-root tests | RTM: FR-14
Update `.github/workflows/build.yml` to run `cargo test` at workspace root instead of crate-scoped `-p` flags.
- **Files**: `.github/workflows/build.yml`
- **Status**: done

### T-10.9: Rename config env vars to match C# | RTM: FR-15
Rename `MAENIFOLD_SESSION_ABANDONMENT_MINUTES` → `MAENIFOLD_SESSION_ABANDON_MINUTES` and `MAENIFOLD_ENABLE_INCREMENTAL_SYNC` → `MAENIFOLD_AUTO_SYNC`. Add reads for missing vars: `MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY` (default 40), `MAENIFOLD_INCREMENTAL_VACUUM_MINUTES` (default 720), `MAENIFOLD_SESSION_CLEANUP` (default true), `MAENIFOLD_SYNC_LOGGING` (default true), `MAENIFOLD_WATCHER_BUFFER` (default 65536), `MAENIFOLD_SQLITE_BUSY_TIMEOUT` (as env var, default 5000), `MAENIFOLD_EMBEDDING_LOGS` (default false), `MAENIFOLD_VECTOR_LOGS` (default false).
- **Files**: `crates/maenifold-core/src/config.rs`, `crates/maenifold-core/src/sessions.rs`, `crates/maenifold-core/src/watchers.rs`, `crates/maenifold-core/src/db.rs`, `crates/maenifold-core/src/search.rs`, `crates/maenifold-core/src/vector.rs`
- **Status**: done

### T-10.10: Fix RepairConcepts Mode B stub-acceptance | RTM: FR-17
Remove all `if output.contains("not yet implemented")` bail-out paths from `repair_concepts_boundary.rs`. Replace with real assertions. If Mode B is not implemented, implement it.
- **Files**: `crates/maenifold-core/tests/repair_concepts_boundary.rs`, `crates/maenifold-core/src/search.rs`
- **Status**: done

### T-10.11: Add missing Sprint 9 RTM test coverage | RTM: FR-18
Add boundary tests for: RTM-052 (tokenizer.json preferred), RTM-054 (max tokens default 512), RTM-058 (vec0 runtime dimensions), RTM-059 (vec0 recreation on dimension change).
- **Files**: `crates/maenifold-core/tests/dynamic_model_tests.rs`
- **Status**: done

### T-10.12: Update stale readme.md | RTM: FR-16
Remove "still placeholder" section. Update tool surface to show 28/28. Update remaining gaps.
- **Files**: `readme.md`
- **Status**: done

### T-10.13: Update stale TODO.md status fields | RTM: FR-16
Update completed T-* items from `Status: pending` to `Status: done`.
- **Files**: `TODO.md`
- **Status**: done

### T-10.14: Create docs/DEVELOPMENT.md | RTM: FR-16
Create Rust-specific developer guide with build, test, and contribution instructions.
- **Files**: `docs/DEVELOPMENT.md` (new)
- **Status**: done

### T-10.15: Copy bundled integration assets | RTM: FR-16
Copy `src/assets/integrations/claude-code/` directory from C# reference repo to `assets/integrations/claude-code/` in Rust. Includes hook scripts, README, QUICK_START, and SKILL that ship alongside the binary.
- **Files**: `assets/integrations/` (new directory tree)
- **Status**: done

### T-10.16: Fix WikiLink filter format to match C# | RTM: FR-19
Change `WikiLinkFilter` in `markdown.rs` to read `.wikilink-filter.json` (JSON object format) instead of `.wikilink-filter` (line-based text). C# `WikiLinkFilter.cs` reads JSON where keys are concept names and values are block reasons. The Rust filter must match this format for shared-root compatibility. Update file path from `memory_path.join(".wikilink-filter")` to `memory_path.join(".wikilink-filter.json")`. Parse as JSON object. Keep mtime cache, symlink rejection, and normalize_concept logic. Update all tests that create `.wikilink-filter` fixtures to use JSON format.
- **Files**: `crates/maenifold-core/src/markdown.rs`, `crates/maenifold-core/tests/wikilink_filter_unit_tests.rs`, `crates/maenifold-core/tests/wikilink_filter_integration_tests.rs`
- **Status**: done

### T-10.17: Remove asset catalog cross-repo masking | RTM: FR-20
Remove or gate `preferred_catalog_assets_root()` in `assets.rs` so it never falls through to a sibling C# repo's `src/assets/`. The function at line 578 hardcodes `../maenifold/src/assets` — this masks missing Rust assets during testing. Either remove the function entirely and always use the Rust repo's own `assets/` path, or gate it behind an explicit env var like `MAENIFOLD_CATALOG_OVERRIDE`. Update any callers.
- **Files**: `crates/maenifold-core/src/assets.rs`
- **Status**: done

## Sprint 11 — Search Core Fix

### T-11.17: Fix SearchMemories text scoring formula | RTM: NFR-1
Replace binary term matching with C#-parity occurrence counting (+5 title bonus per term). Fix snippet whitespace normalization.
- **Files**: `crates/maenifold-core/src/search.rs`
- **Status**: done

### T-11.18: Wire real vector search into SearchMemories | RTM: NFR-1
Replace content.contains() heuristic with two-pipeline architecture: text pipeline (occurrence counting) + vector pipeline (vec_distance_cosine on vec_memory_files) + RRF fusion (k=60). Graceful degradation when vec0 unavailable.
- **Files**: `crates/maenifold-core/src/search.rs`
- **Status**: done
- **Depends on**: T-11.17

### Pre-existing failures (not Sprint 10 regressions)

### T-10.18: Fix pre-existing MCP stdio test failures | RTM: NFR-3
6 tests in `crates/maenifold-cli/tests/mcp_tests.rs` fail with `connection closed: initialized request`. Verified pre-existing via git bisect to before Sprint 10. Root cause: MCP stdio handshake protocol mismatch between test harness and rmcp-based server.
- **Files**: `crates/maenifold-cli/tests/mcp_tests.rs`, `crates/maenifold-core/src/mcp.rs`
- **Status**: done

### T-10.19: Fix pre-existing oracle parity test failures | RTM: NFR-1
12 tests across `tests/compat/` fail against C# oracle. Verified pre-existing. Failures: read_memory_not_found, write_memory_validation_error, write_memory_path_security_error, read_catalog_json, build_context_json, search_memories_json, sequential_thinking_requires_zero, assumption_ledger_append, workflow_json, extract_concepts, find_similar_concepts, sync_json.
- **Files**: `tests/compat/*.rs`
- **Status**: done


## Sprint 12 — Structural Refactor / Facade Preservation

### T-12.0: Split workflow, MCP resource, and graph helper modules while preserving public entrypoints | RTM: NFR-1
Extract `workflow_ops.rs`, `mcp_resources.rs`, `graph_preview.rs`, `graph_analyzer.rs`, and `similar.rs`; keep `workflow.rs`, `mcp.rs`, `graph.rs`, and the `search.rs` facade behaviorally stable while delegating to the split modules.
- **Files**: `crates/maenifold-core/src/workflow.rs`, `crates/maenifold-core/src/workflow_ops.rs`, `crates/maenifold-core/src/mcp.rs`, `crates/maenifold-core/src/mcp_resources.rs`, `crates/maenifold-core/src/graph.rs`, `crates/maenifold-core/src/graph_preview.rs`, `crates/maenifold-core/src/graph_analyzer.rs`, `crates/maenifold-core/src/similar.rs`, `crates/maenifold-core/src/search.rs`
- **Status**: done

### T-12.1: Extract sync runtime from `search.rs` while preserving the existing tool surface | RTM: NFR-1
Move sync / incremental-sync logic into `sync.rs` and rewire callers without changing observed tool behavior.
- **Files**: `crates/maenifold-core/src/search.rs`, `crates/maenifold-core/src/sync.rs`, `crates/maenifold-core/src/watchers.rs`, `crates/maenifold-core/src/benchmark.rs`
- **Status**: done

### T-12.2: Extract concept repair / concept analysis handlers from `search.rs` | RTM: NFR-1
Move `RepairConcepts`, `AnalyzeConceptCorruption`, and concept-extraction helpers into `repair.rs` while preserving the public tool surface.
- **Files**: `crates/maenifold-core/src/repair.rs`, `crates/maenifold-core/src/search.rs`, `crates/maenifold-core/src/registry.rs`, `crates/maenifold-core/tests/frontmatter_tests.rs`, `crates/maenifold-core/tests/memory_path_tests.rs`, `crates/maenifold-core/tests/vector_tools_tests.rs`
- **Status**: done

### T-12.3: Extract `FindSimilarConcepts` runtime helpers from `search.rs` | RTM: NFR-1
Move similarity-search implementation into `similar.rs` while preserving the caller-facing `search` facade and registry dispatch.
- **Files**: `crates/maenifold-core/src/similar.rs`, `crates/maenifold-core/src/search.rs`, `crates/maenifold-core/src/registry.rs`, `crates/maenifold-core/src/lib.rs`
- **Status**: done

### T-12.4: Finish sync-module extraction cleanup inside `sync.rs` | RTM: NFR-1
Own extracted helper lookups and remove reverse dependencies on `search.rs` so `sync.rs` is self-contained.
- **Files**: `crates/maenifold-core/src/sync.rs`
- **Status**: done

### T-12.5: Extract shared decay logic into `decay.rs` while keeping `memory.rs` backward-compatible | RTM: NFR-1
Move shared decay/date/path helpers into `decay.rs` and keep `memory.rs` as the stable public facade for existing callers.
- **Files**: `crates/maenifold-core/src/decay.rs`, `crates/maenifold-core/src/memory.rs`
- **Status**: done

### T-12.6: Reconcile exports, registry wiring, benchmark routing, and regression-test imports after the split | RTM: NFR-1
Update `lib.rs`, `registry.rs`, benchmark routing, and affected tests so the extracted modules compile against the existing public API surface.
- **Files**: `crates/maenifold-core/src/lib.rs`, `crates/maenifold-core/src/registry.rs`, `crates/maenifold-core/src/benchmark.rs`, `crates/maenifold-core/tests/frontmatter_tests.rs`, `crates/maenifold-core/tests/memory_path_tests.rs`, `crates/maenifold-core/tests/vector_tools_tests.rs`
- **Status**: done

### T-12.7: Sprint 12 acceptance / landing hygiene | RTM: NFR-1
Run focused acceptance gates, keep traceability comments honest for the Sprint 12 structural refactor, and stage the extracted runtime modules with their tracked importers before landing.
- **Files**: `crates/maenifold-core/src/*.rs`, `crates/maenifold-core/tests/*.rs`, `crates/maenifold-cli/tests/*.rs`
- **Status**: done

## Sprint 13 — Parity Audit Remediation

### T-13.1: Implement separate search mode paths (Semantic / FullText / Hybrid) | RTM: FR-6
C# has 3 distinct code paths: `PerformHybridSearch` (RRF fusion of text+vector), `PerformSemanticSearch` (vector-only, filter by semantic score, decay-weighted), `PerformFullTextSearch` (text-only, max-normalized scores, decay-weighted). Rust currently runs ALL modes through the hybrid RRF path and just filters by different score key afterward. Must implement:
1. When mode=Semantic: skip text search, run vector-only, apply decay weighting to raw semantic scores, return single `score` field
2. When mode=FullText: skip vector search, run text-only, normalize by max score, apply decay weighting, return single `score` field
3. When mode=Hybrid (default): keep existing RRF fusion logic unchanged
4. Output format: non-hybrid modes return `score` instead of `fusedScore`/`textScore`/`semanticScore`
- **Files**: `crates/maenifold-core/src/search/api.rs`
- **Status**: done

### T-13.2: Fix PreFusion multiplier to match C# (3x not 5x) | RTM: FR-6
C# uses `PreFusionMultiplier = 3` (`pageSize * 3`) for candidate fetching before RRF fusion. Rust uses `page_size * 5`. Change Rust to `page_size * 3` to match. Also match the non-hybrid modes: C# uses `pageSize * 2` for Semantic and FullText single-mode searches.
- **Files**: `crates/maenifold-core/src/search/api.rs`
- **Status**: done
- **Depends on**: T-13.1

### T-13.3: Add FullText score normalization | RTM: FR-6
C# FullText mode normalizes text scores by dividing each by the maximum score (`score / maxScore`). Rust does not normalize text scores. Add max-normalization to the FullText search path created in T-13.1.
- **Files**: `crates/maenifold-core/src/search/api.rs`
- **Status**: done
- **Depends on**: T-13.1

### T-13.4: Complete MemoryStatus output to match C# | RTM: FR-7
Rust `memory_status` returns only `fileCount` + `databasePath`. C# returns: file count, folder count, total size MB, graph database stats (concepts count, relations count, mentions count), DB file size MB. Add all missing fields to the Rust implementation. Use `walkdir` for folder count and file sizes, open DB connection for graph stats, use `std::fs::metadata` for DB file size.
- **Files**: `crates/maenifold-core/src/tools/diagnostics.rs`
- **Status**: done

### T-13.5: Port ToolResponse.WithNextSteps and WithHint helpers | RTM: FR-8
C# has `ToolResponse.WithNextSteps(result, "ReadMemory", "BuildContext", ...)` and `ToolResponse.WithHint(result, toolName, hint)` used 26 times across tool responses. These append suggested next tools and contextual hints to help LLM consumers take follow-up actions. Implement equivalent helpers in Rust and wire them into all 26 call sites matching C# usage. Check each tool's return path in C# to identify where WithNextSteps/WithHint is called and replicate.
- **Files**: `crates/maenifold-core/src/tools/response.rs`, plus tool modules for wiring
- **Status**: done

### T-13.6: Evaluate and resolve dead code behind #[allow(dead_code)] | RTM: NFR-1
9 functions in `sync/graph_state.rs` (3), `sync/graph_relations.rs` (3), `graph/preview.rs` (3) are marked `#[allow(dead_code)]`. For each function: determine if it's needed for a C# feature not yet wired up, or if it's truly dead. If needed: remove the allow and wire it into the caller chain. If dead: delete it. Document rationale for each decision.
- **Files**: `crates/maenifold-core/src/sync/graph_state.rs`, `crates/maenifold-core/src/sync/graph_relations.rs`, `crates/maenifold-core/src/graph/preview.rs`
- **Status**: done

### T-13.7: Search parity tests | RTM: FR-6
Write integration tests verifying:
1. Semantic mode returns vector-only results with single `score` field, no text search executed
2. FullText mode returns text-only results with normalized scores, no vector search executed
3. Hybrid mode returns fused results with `fusedScore`/`textScore`/`semanticScore`
4. PreFusion candidate count matches C# (pageSize*3 for hybrid, pageSize*2 for single-mode)
5. FullText score normalization produces scores in [0.0, 1.0] range
- **Files**: `crates/maenifold-core/tests/search_status_parity_tests.rs`
- **Status**: done
- **Depends on**: T-13.1, T-13.2, T-13.3

### T-13.8: MemoryStatus and ToolResponse tests | RTM: FR-7, FR-8
Write integration tests verifying:
1. MemoryStatus returns folder count, file count, total size, concept/relation/mention counts, DB size
2. WithNextSteps appends correct tool suggestions to markdown output
3. WithHint appends contextual hints
4. JSON output mode includes next_steps/hints in structured response
- **Files**: `crates/maenifold-core/tests/memory_status_parity.rs`, `crates/maenifold-core/tests/search_status_parity_tests.rs`
- **Status**: done
- **Depends on**: T-13.4, T-13.5

## Sprint 14 — Red-Team Drift Remediation

### T-14.1: Fix decay fallback for missing last_accessed | RTM: FR-9
Rust `compute_decay_weight` in `decay/scoring.rs` returns 1.0 when `last_accessed` is None. C# falls back to `created` timestamp and still computes exponential/power-law decay. Fix: when `last_accessed` is None, use the file's creation date (from frontmatter or filesystem) as the fallback before computing decay. Without this fix, never-accessed files permanently dominate rankings.
- **Files**: `crates/maenifold-core/src/decay/scoring.rs`
- **Status**: done

### T-14.2: Graceful frontmatter parse failure | RTM: FR-10
Rust `read_markdown` in `markdown/storage.rs` returns `Err` when YAML frontmatter is malformed. C# catches the parse error, logs a warning, and continues with `frontmatter = null`. Fix: catch `serde_yaml` deserialization errors and return `Ok` with empty frontmatter + full content, matching C# behavior.
- **Files**: `crates/maenifold-core/src/markdown/storage.rs`
- **Status**: done

### T-14.3: Support CRLF line endings in frontmatter detection | RTM: FR-10
Rust `parse_frontmatter` only matches `\n---\n`. Files with `\r\n` (Windows) or no trailing newline after closing `---` never expose frontmatter. Fix: normalize line endings before delimiter matching, or use a regex that handles both `\n` and `\r\n`.
- **Files**: `crates/maenifold-core/src/markdown/storage.rs`
- **Status**: done

### T-14.4: Fix co_occurrence_count to count files not sum weights | RTM: FR-11
Rust sync sums section proximity weights (1.0/0.5) into `co_occurrence_count`. C# counts unique files (1 per file). This inflates graph edges and breaks community detection / BuildContext parity. Fix: align Rust to count unique contributing files, not sum weights. Keep the proximity-weighted score in a separate field if needed for future use.
- **Files**: `crates/maenifold-core/src/sync/graph_relations.rs`, `crates/maenifold-core/src/sync/graph_state.rs`
- **Status**: done

### T-14.5: Add symlink canonicalization to path validation | RTM: FR-12
`validate_relative_path` in `util/paths.rs` never resolves symlinks. An attacker with a symlink under `assets/` or `memory/` can escape the catalog root. Fix: canonicalize the final resolved path and verify it remains under the expected base path. Apply to `memory_uri_to_path`, `is_watchable_asset_json_path`, and MCP resource handlers.
- **Files**: `crates/maenifold-core/src/util/paths.rs`, `crates/maenifold-core/src/assets/support.rs`
- **Status**: done

### T-14.6: Fix NMI threshold env var name | RTM: FR-13
Rust reads `MAENIFOLD_NMI_THRESHOLD`. C# reads `MAENIFOLD_COMMUNITY_NMI_THRESHOLD`. Fix Rust to match the C# env var name so operators can control drift thresholds consistently across runtimes.
- **Files**: `crates/maenifold-core/src/community/metrics.rs`
- **Status**: done

### T-14.7: Remove co_occurrence_count fallback in community edge weights | RTM: FR-13
Rust `graph_edge_weight` falls back to `co_occurrence_count` when `source_files` is empty. C# only uses `source_files`, so edges without source-file metadata carry zero weight. Remove the fallback to match C# behavior.
- **Files**: `crates/maenifold-core/src/community/data.rs`
- **Status**: done

### T-14.8: Truncate long titles instead of rejecting | RTM: FR-14
Rust `write_memory` rejects titles whose slug exceeds 255 chars with `PATH_TOO_LONG`. C# `SanitizeUserInput` truncates to 255 chars and succeeds. Fix: truncate the slug to 255 chars instead of returning an error.
- **Files**: `crates/maenifold-core/src/memory/access.rs`
- **Status**: done

### T-14.9: Return warning text instead of Err for semantic validation | RTM: FR-15
Rust `validate_repair_concepts_semantics` returns `Err(MaenifoldError)` when variants are too dissimilar. C# returns a human-readable "BLOCKED" warning string as a successful response. Fix: return `Ok(warning_text)` instead of `Err` so automation that expects 200 responses with textual warnings continues to work.
- **Files**: `crates/maenifold-core/src/repair/api.rs`
- **Status**: done

### T-14.10: Accept suffix-free legacy session IDs | RTM: FR-16
Rust `is_valid_session_id` requires `session-{timestamp}-{suffix}` format. C# accepts `session-{timestamp}` without suffix. Fix: relax validation to accept both formats for cross-runtime session resumption.
- **Files**: `crates/maenifold-core/src/thinking/session.rs`, `crates/maenifold-core/src/sessions/support.rs`
- **Status**: done

### T-14.11: Add entropy to session ID suffix | RTM: FR-16
Rust session ID suffix is derived from nanosecond counter (predictable from wall-clock time). C# uses `Random.Shared.Next`. Fix: mix in random entropy (e.g., `thread_rng`) so session IDs cannot be predicted by an observer who knows the creation timestamp.
- **Files**: `crates/maenifold-core/src/thinking/session.rs`
- **Status**: done

## Sprint 15 — CLI Parity Remediation

### T-15.0: Sprint 15 CLI parity — 27 fixes | RTM: NFR-1
Batch remediation of 27 CLI parity issues identified by oracle comparison across all tool categories. Executed by 8 parallel SWE agents.
- **Status**: done

Key fixes included:
- **PARITY-S01/S02/S05**: SearchMemories NULL distance crash (Hybrid/Semantic/Tags) — `row.get::<_, f64>` → `Option<f64>` with NULL skip
- **PARITY-S03**: FullText search output format matches C# oracle (emoji headers, bold titles, score display, page indicators)
- **PARITY-B01/RES003**: BuildContext format + depth traversal matches C# (header, section layout, file truncation, direct_relations)
- **PARITY-F01**: FindSimilarConcepts displays weighted_similarity (not raw)
- **PARITY-ERR-PREFIX**: Removed 'Invalid argument:' prefix from error messages
- **T-16.1**: CLI help tool groupings match C# (MCP group, no Adopt:/Assets: prefixes)
- **PARITY-CLI-PAYLOAD**: Missing --payload now rejected (was silently using `{}`)
- **PARITY-VAL002/VAL003**: Empty title + integer folder type validation added
- **PARITY-INT002**: Nested WikiLink regex rejects malformed brackets
- **PARITY-R03**: RepairConcepts blocks when vec0 unavailable (was warning)
- **PARITY-WORK001**: Workflow step JSON pretty-printed (was compact)
- **PARITY-R01-FMT**: RepairConcepts output format matches C#
- **PARITY-E02-ZERO**: ExtractConcepts zero-result message matches C#
- **PARITY-R02-NA**: ReadMemory shows N/A for missing dates (was empty)
- **PARITY-R01-TS**: ReadMemory timestamps in local time format
- **PARITY-CLI-HELP-EXIT**: --help exits 1 (matches C#)
- **PATH_TOO_LONG**: Io error mapped to correct JSON error code

### T-15.1: Fix vec0 extension loading | RTM: NFR-1
Deduplicate vec0 extension loading helper; load in `repair/semantics.rs` and `db/init.rs` for reliable extension availability.
- **Files**: `crates/maenifold-core/src/repair/semantics.rs`, `crates/maenifold-core/src/db/init.rs`
- **Status**: done

---

## Remaining Polish

> These are low-severity parity gaps identified during Sprint 15 oracle comparison.
> All core functionality works; these affect output formatting only.

### PARITY-A01-FMT: RecentActivity field order | RTM: NFR-1
RecentActivity output field ordering may diverge from C# oracle in edge cases (thinking vs. memory path field sequences). Verify and align field emission order to match C# `RecentActivity.Formatter.cs`.
- **Files**: `crates/maenifold-core/src/tools/recent_activity_format.rs`
- **Status**: done (verified in Sprint 15; field order matches C#)

### PARITY-E01: ExtractConceptsFromFile sort order | RTM: NFR-1
ExtractConceptsFromFile must preserve document order (not alphabetical). Previously used BTreeSet which silently re-sorted. Fixed to use insertion-order-preserving collection.
- **Files**: `crates/maenifold-core/src/repair/analysis.rs`
- **Status**: done (fixed in Sprint 15; uses HashSet+Vec for document order)

### PARITY-A04: Adopt error wording | RTM: NFR-1
Adopt tool error message must read "Must be one of:" (not "Valid types are:") to match C# `AdoptTools.cs`.
- **Files**: `crates/maenifold-core/src/assets/adopt.rs`, `crates/maenifold-core/src/assets/types.rs`
- **Status**: done (fixed in Sprint 10; both files use "Must be one of:")

### PARITY-U01: UpdateAssets output format + comparison scope | RTM: NFR-1
UpdateAssets output format: no Status/DRY RUN header line, no "Skipped symlinks" count, omit empty sections. Match C# `AssetManager.cs` update output.
Comparison scope restricted to the 4 published asset catalogs (workflows/, roles/, colors/, perspectives/) via `AssetKind::all()`. Directories like integrations/, models/, agents/, native/, usage/ are excluded — matching C# effective scope.
- **Status**: done
- **Files**: `crates/maenifold-core/src/assets/update.rs`
- **Status**: done (fixed in Sprint 15; output matches C# oracle)

---

## Summary

| Sprint | Items | Done | Pending | Obsolete |
|--------|-------|------|---------|----------|
| Sprint 1 | 6 | 6 | 0 | 0 |
| Sprint 2 | 4 | 4 | 0 | 0 |
| Sprint 3 | 2 | 2 | 0 | 0 |
| Cross-Sprint | 3 | 3 | 0 | 0 |
| Sprint 9 | 7 | 6 | 0 | 1 |
| Sprint 10 | 17 | 17 | 0 | 0 |
| Sprint 11 | 3 | 3 | 0 | 0 |
| Sprint 12 | 8 | 8 | 0 | 0 |
| Sprint 13 | 8 | 8 | 0 | 0 |
| Sprint 14 | 11 | 11 | 0 | 0 |
| Sprint 15 | 2 | 2 | 0 | 0 |
| Polish | 4 | 4 | 0 | 0 |
| **Total** | **75** | **74** | **0** | **1** |

**Build**: `cargo test` — 500+ tests passing, 0 failures
**Tools**: 28/28 implemented (zero placeholders)
**Parity**: CLI output matches C# oracle (27 format fixes in Sprint 15)

---

## Sprint 16 — Storage Abstraction + PostgreSQL Backend

### T-16.1: Define operation-oriented `trait DbBackend` and `SqliteBackend` wrapper | RTM: FR-22
Introduce `trait DbBackend` in `crates/maenifold-core/src/db/backend.rs` using an **operation-oriented** design — trait methods map to logical data operations (e.g., `upsert_file_content()`, `query_concepts()`, `vector_search()`, `graph_traverse()`), NOT to connection handles. Callers never see a `Connection` or `Pool` — they call domain operations and get results. This ensures a third backend (e.g., Cosmos DB, in-memory, remote API) is a clean trait impl addition, not a rewrite.

**Design principles:**
- Methods grouped by domain: file content, graph, vector search, sync, maintenance
- All methods take `&self` + domain-specific params, return `Result<DomainType>`
- No `connection()` method — backend owns its connection lifecycle internally
- `SqliteBackend` wraps existing `rusqlite` code internally, exposing only domain operations. Zero behavioral change.
- `PgBackend` wraps `sqlx::PgPool` internally behind the same trait surface.
- `ToolContext` carries `Arc<dyn DbBackend>` instead of `database_path: PathBuf`

**Files**: `crates/maenifold-core/src/db/backend.rs` (new), `crates/maenifold-core/src/db/mod.rs`, `crates/maenifold-core/src/tools/context.rs`
- **Status**: pending

### T-16.2: Migrate 27 call sites from `open_connection()` to `DbBackend` trait methods | RTM: FR-22
Replace all 27 inline `db::open_connection(&context.config.database_path)` calls + raw SQL with calls to `context.backend.method()` domain operations. Each call site invokes a trait method (e.g., `backend.query_file_content()`, `backend.upsert_concept()`) instead of opening a connection and writing SQL. The `SqliteBackend` impl produces identical results. No behavioral change — domain logic preserved, SQL encapsulated inside backend impls.
- **Files**: `benchmark/mod.rs`, `benchmark/render.rs`, `community/persist.rs`, `decay/metadata.rs`, `graph/analyzer.rs`, `graph/build_context.rs`, `memory/listing.rs`, `memory/support.rs`, `repair/semantics.rs`, `search/api.rs`, `sessions/cleanup.rs`, `similar/mod.rs`, `sync/leader.rs`, `sync/mod.rs`, `sync/watchers.rs`, `tools/diagnostics.rs`, `tools/recent_activity.rs`
- **Status**: pending
- **Depends on**: T-16.1

### T-16.3: Implement `PgBackend` with sqlx + pgvector | RTM: FR-22
Implement `PgBackend` behind `--features postgres`. Uses `sqlx::PgPool` for connection management. Async pool operations wrapped with `tokio::task::block_in_place` to preserve sync `ToolFn` signature. Requires `sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "json", "time"] }` and `pgvector = { version = "0.4", features = ["sqlx"] }`.
- **Files**: `crates/maenifold-core/src/db/pg_backend.rs` (new), `Cargo.toml`
- **Status**: pending
- **Depends on**: T-16.1

### T-16.4: PostgreSQL schema — relational tables | RTM: FR-22
Create PG schema migration for: `file_content`, `concepts`, `concept_mentions`, `concept_graph`, `concept_communities`, `model_metadata`, `sync_leader_lease`. Use `sqlx::migrate!()` macro. Map SQLite types: `INTEGER PRIMARY KEY` → `SERIAL`, `TEXT` → `TEXT`, `REAL` → `DOUBLE PRECISION`, `BLOB` → `BYTEA`. Add `tsvector` generated column on `file_content` for future FTS.
- **Files**: `crates/maenifold-core/migrations/` (new directory), `crates/maenifold-core/src/db/pg_backend.rs`
- **Status**: pending
- **Depends on**: T-16.3

### T-16.5: PostgreSQL schema — pgvector tables + HNSW indexes | RTM: FR-22
Replace vec0 virtual tables with: `vec_concepts (concept_name TEXT PK, embedding VECTOR(N))` and `vec_memory_files (file_path TEXT PK, embedding VECTOR(N))`. Create HNSW indexes with `vector_cosine_ops`. Rewrite 6 `vec_distance_cosine()` query sites to use `<=>` operator.
- **Files**: `crates/maenifold-core/migrations/`, `crates/maenifold-core/src/search/vector.rs`, `crates/maenifold-core/src/similar/vector_query.rs`, `crates/maenifold-core/src/db/pg_backend.rs`
- **Status**: pending
- **Depends on**: T-16.4

### T-16.6: Tests for storage abstraction | RTM: FR-22, NFR-5
Boundary tests: SqliteBackend produces identical results to current code. Feature-gated PG tests using `testcontainers-rs` with PostgreSQL image. Verify: schema init, CRUD on all tables, vector search, concurrent access (NFR-6).
- **Files**: `crates/maenifold-core/tests/backend_tests.rs` (new)
- **Status**: pending
- **Depends on**: T-16.2, T-16.5

## Sprint 17 — Streamable HTTP Transport

### T-17.1: Add rmcp HTTP transport feature + axum dependency | RTM: FR-23
Update workspace `Cargo.toml`: add `transport-streamable-http-server` to rmcp features (gated behind `http` feature flag). Add `axum = "0.7"` and `tower-http = { version = "0.6", features = ["cors"] }` dependencies.
- **Files**: `Cargo.toml`, `crates/maenifold-core/Cargo.toml`
- **Status**: pending

### T-17.2: Implement `serve_http()` in MCP server | RTM: FR-23
Add `MaenifoldMcpServer::serve_http(port: u16)` alongside existing `serve_stdio()`. Uses `StreamableHttpService` from rmcp mounted on axum router. CORS permissive. Session tracking via `Mcp-Session-Id` headers. Same `MaenifoldHandler` used for both transports.
- **Files**: `crates/maenifold-core/src/mcp/http.rs` (new), `crates/maenifold-core/src/mcp/mod.rs`
- **Status**: pending
- **Depends on**: T-17.1

### T-17.3: CLI `--http` flag and `--port` option | RTM: FR-23
Add `--http` and `--port <PORT>` CLI arguments. `--http` starts `serve_http()`. `--mcp` unchanged. Mutually exclusive flags. Default port 8080.
- **Files**: `crates/maenifold-cli/src/main.rs` or `crates/maenifold-core/src/cli.rs`
- **Status**: pending
- **Depends on**: T-17.2

### T-17.4: Polling-based watcher fallback | RTM: FR-26
Add polling mode to watcher: scan `memory_path` on interval using `walkdir` + mtime comparison. Activated by `MAENIFOLD_WATCHER_POLL_SECONDS` env var (default: 0 = disabled, use native). When set, replaces `notify::recommended_watcher` with a timer-driven scan loop. Same `sync_single_file()` incremental logic.
- **Files**: `crates/maenifold-core/src/sync/watchers.rs`
- **Status**: pending

### T-17.5: HTTP transport tests | RTM: FR-23
Integration tests: start HTTP server on random port, send MCP initialize + tool/call requests via reqwest, verify JSON-RPC responses. Test session creation and cleanup.
- **Files**: `crates/maenifold-core/tests/http_transport_tests.rs` (new)
- **Status**: pending
- **Depends on**: T-17.2

## Sprint 18 — Entra ID Authentication

### T-18.1: RFC 9728 protected resource metadata endpoint | RTM: FR-24
Add `GET /.well-known/oauth-protected-resource` route to axum router. Returns JSON with `resource` (server URL), `authorization_servers` (Entra ID tenant URL), `scopes_supported` (["maenifold:tools"]), `bearer_methods_supported` (["header"]). Configurable via env vars: `MAENIFOLD_AUTH_TENANT_ID`, `MAENIFOLD_AUTH_SERVER_URL`.
- **Files**: `crates/maenifold-core/src/auth/metadata.rs` (new), `crates/maenifold-core/src/mcp/http.rs`
- **Status**: pending
- **Depends on**: T-17.2

### T-18.2: JWT validation middleware | RTM: FR-24
Axum middleware layer that validates Bearer tokens from Authorization header. Uses `jsonwebtoken` crate with Entra ID JWKS endpoint (`https://login.microsoftonline.com/{tenant}/discovery/v2.0/keys`). Validates: issuer, audience, expiration, `maenifold:tools` scope. Returns 401 with `WWW-Authenticate` header on failure. Cached JWKS with 1-hour refresh.
- **Files**: `crates/maenifold-core/src/auth/jwt.rs` (new), `crates/maenifold-core/src/auth/mod.rs` (new)
- **Status**: pending
- **Depends on**: T-18.1

### T-18.3: PG managed identity authentication | RTM: FR-24
Use `azure_identity::ManagedIdentityCredential` to acquire token for `https://ossrdbms-aad.database.windows.net/.default`. Pass as password in sqlx `PgConnectOptions`. Handle token refresh (pool `max_lifetime` < 1 hour).
- **Files**: `crates/maenifold-core/src/db/pg_backend.rs`
- **Status**: pending
- **Depends on**: T-16.3

### T-18.4: `--no-auth` bypass flag | RTM: FR-24
Skip JWT validation when `--no-auth` is passed. For local development and testing only. Log warning when active.
- **Files**: `crates/maenifold-core/src/auth/mod.rs`, `crates/maenifold-core/src/cli.rs`
- **Status**: pending
- **Depends on**: T-18.2

### T-18.5: Auth tests | RTM: FR-24
Test: valid token accepted, expired token rejected, wrong audience rejected, missing token returns 401, `--no-auth` bypasses validation, RFC 9728 metadata returned correctly.
- **Files**: `crates/maenifold-core/tests/auth_tests.rs` (new)
- **Status**: pending
- **Depends on**: T-18.2

## Sprint 19 — Deployment + SRE Agent Integration

### T-19.1: Dockerfile | RTM: FR-25
Multi-stage Dockerfile: Rust builder image → debian-slim runtime. Copy binary + assets + ONNX model. Expose port 8080. Health check endpoint. ENV defaults for PG connection, auth tenant, memory path.
- **Files**: `Dockerfile` (new)
- **Status**: pending
- **Depends on**: T-17.2

### T-19.2: Bicep/azd infrastructure templates | RTM: FR-25
Bicep templates for: ACA environment + container app (1 replica, 1 vCPU, 2GB), Azure PG Flexible Server (B1ms, pgvector extension enabled), Azure Files share (mounted at /maenifold/memory), managed identity (Contributor on PG, Storage File Data SMB Share Contributor on Files). azd integration for `azd up` deployment.
- **Files**: `deploy/main.bicep` (new), `deploy/modules/` (new), `azure.yaml` (new)
- **Status**: pending

### T-19.3: SRE Agent connector configuration | RTM: FR-25
Sample YAML for Azure SRE Agent connector registration. Subagent configuration with `mcp_tools: [maenifold-mcp/*]` wildcard. Documentation for connector setup in SRE Agent portal.
- **Files**: `deploy/sre-agent/connector.yaml` (new), `deploy/sre-agent/subagent.yaml` (new), `docs/SRE_AGENT.md` (new)
- **Status**: pending
- **Depends on**: T-19.1

### T-19.4: CI container build workflow | RTM: FR-25
GitHub Actions workflow: trigger on `v*` tags. Build container image with `--features postgres,http`. Push to GitHub Container Registry (ghcr.io). Optionally push to Azure Container Registry if configured.
- **Files**: `.github/workflows/container.yml` (new)
- **Status**: pending
- **Depends on**: T-19.1

### T-19.5: End-to-end deployment test | RTM: FR-25, NFR-6
Deploy to a test ACA environment. Verify: container starts, PG schema initializes, Sync completes, tool calls succeed over HTTP, Entra ID auth works, 5 concurrent clients work.
- **Files**: `deploy/test/e2e.sh` (new)
- **Status**: pending
- **Depends on**: T-19.1, T-19.2, T-18.2

## Sprint 20 — Vector Sync Diagnostics

### T-20.1: Surface ONNX inference error details for concept embedding failures | RTM: FR-7
When `Sync` records `CONCEPT_EMBEDDING_FAILED`, include actionable error detail from the failing `vector::generate_embeddings` call path so users can distinguish model discovery, tokenizer, and ONNX runtime inference failures.
- **Files**: `crates/maenifold-core/src/sync/vector_embeddings.rs`, `crates/maenifold-core/src/sync/vector_diagnostics.rs`, `crates/maenifold-core/src/vector/inference.rs` (if needed)
- **Status**: done

### T-20.2: Red-team audit for vector sync diagnostics patch | RTM: FR-7
Audit T-20.1 for correctness, control-flow safety, data exposure risk, and traceability to FR-7.
- **Files**: `crates/maenifold-core/src/sync/vector_embeddings.rs`, `crates/maenifold-core/src/sync/vector_diagnostics.rs`, `crates/maenifold-core/tests/vector_sync_diag_reporting.rs`
- **Status**: done
- **Depends on**: T-20.1

### T-20.3: Blue-team targeted validation for diagnostics patch | RTM: FR-7
Run targeted diagnostics tests to verify existing counters/reason codes plus newly surfaced failure detail output.
- **Files**: `crates/maenifold-core/tests/vector_sync_diag_reporting.rs`, `crates/maenifold-core/tests/vector_sync_diag_unavailable.rs`
- **Status**: done
- **Depends on**: T-20.1

### T-20.4: Release build verification for diagnostics patch | RTM: NFR-3
Build release artifacts after T-20.1 validation completes.
- **Files**: workspace build output
- **Status**: done
- **Depends on**: T-20.2, T-20.3

### T-20.5: Fix model discovery for symlinked binary invocation | RTM: FR-7
When running `maenifold` via a symlinked install path (e.g., `/usr/local/bin/maenifold`), model discovery SHALL still resolve ONNX models from the real executable location and supported runtime asset roots.
- **Files**: `crates/maenifold-core/src/vector/discovery.rs`
- **Status**: pending

### T-20.6: Red-team audit for model discovery fix | RTM: FR-7
Audit T-20.5 for parity, control-flow safety, and path-resolution security.
- **Files**: `crates/maenifold-core/src/vector/discovery.rs`, related tests
- **Status**: pending
- **Depends on**: T-20.5

### T-20.7: Blue-team validation for symlink discovery fix | RTM: FR-7
Run targeted tests and command-level verification that `maenifold --tool sync --payload '{}'` resolves models correctly via symlinked and real binary paths.
- **Files**: `crates/maenifold-core/tests/` (targeted), runtime command verification
- **Status**: pending
- **Depends on**: T-20.5
