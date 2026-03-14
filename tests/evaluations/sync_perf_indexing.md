# Test Evaluation: sync_perf_indexing
**File:** `crates/maenifold-core/tests/sync_perf_indexing.rs`
**Traceability:** T-0.1 · T-8.4 · RTM NFR-3 · FR-25.2 · FR-33.3
**Test count:** 7
**Evaluated:** 2026-07-13

## Summary

A mixed bag. Three tests are genuinely unique to this file and load-bearing; one is the strongest decay integration test in the suite; two are **duplicated** across at least three other mtime/hash-guard test files; and the first decay test is a weaker variant of the seventh. The naming in test 6 is actively misleading.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `get_decay_weight_for_file_reads_from_db_matches_decay_calculator_directly` | **REWRITE** | Tests 60-day decay ±0.005. Test 7 tests 120-day decay with identical tolerance PLUS end-to-end BuildContext integration. This test would only catch a decay-formula regression that manifests at 60 days but not 120 — unlikely given the continuous nature of the decay function. If kept, it should assert a property unique to the shorter age: e.g., `assert!(actual > 0.8, "60-day file should be mostly fresh")` to distinguish it from test 7. As-is, it adds marginal confidence at the cost of duplicated setup. Suggested name: `sync_decay_weight_for_60_day_file_is_near_fresh`. |
| `process_file_persists_created_from_frontmatter_to_database` | **KEEP** | Unique: the only test in the suite that verifies `created` from frontmatter survives through sync into the `file_content` DB row (±2s tolerance). Frontmatter read/write tests exist elsewhere but don't sync-index. Removal would allow a regression in the sync path's frontmatter-to-DB pipeline. |
| `process_file_falls_back_to_filesystem_ctime_when_frontmatter_has_no_created_key` | **KEEP** | Unique: verifies the `created` fallback path — when frontmatter lacks a `created` key, sync falls back to filesystem ctime. No other test in the suite covers this code branch. Correctly uses a before/after time window rather than an exact timestamp. |
| `process_file_skips_reindexing_when_mtime_unchanged` | **MERGE** (remove from this file) | Duplicated by: `incremental_sync_unification_guards.rs::incremental_sync_mtime_guard_skips_unchanged_files`, `sync_mtime_opt_guards.rs::process_file_changed_when_mtime_unchanged_skips_before_read_and_does_not_restore_state`, `incremental_sync_boundary_guards.rs::sync_single_file_skips_unchanged_file_with_mtime_guard`, `sync_mtime_skip.rs::sync_when_mtime_unchanged_skips_without_read_and_does_not_restore_mentions_on_unix`. Four dedicated files already cover mtime-guard behavior, some more thoroughly (mention restoration checks). This test is a weaker re-assertion of a path that has dedicated test files. |
| `process_file_skips_reindexing_when_md5_matches_despite_mtime_change` | **MERGE** (remove from this file) | Duplicated by: `incremental_sync_unification_guards.rs::incremental_sync_hash_guard_updates_last_indexed_only`, `sync_mtime_opt_guards.rs::process_file_changed_when_mtime_differs_but_hash_same_updates_last_indexed_only`, `incremental_sync_boundary_guards.rs::sync_single_file_skips_file_with_same_hash`. The `sync_mtime_opt_guards` version is more thorough (verifies mention state is not restored). This test adds no unique assertion beyond what the dedicated guard files already verify. |
| `generate_file_embedding_populates_vec_table_insert_or_replace_runs_to_completion` | **KEEP** (rename required) | Unique: the only test that queries `vec_memory_files` to verify a real ONNX-backed embedding blob is non-empty and non-zero after sync. Tagged T-8.4 / FR-25.2 / FR-33.3, distinct from the NFR-3 cluster. Load-bearing: without it, a regression zeroing out embeddings or failing the `INSERT OR REPLACE` silently would go undetected. **The name is wrong**: "insert_or_replace_runs_to_completion" describes the SQL operation, not the observable behavior. Rename to `sync_populates_vec_memory_files_with_onnx_embedding_blob`. |
| `get_decay_weight_for_file_uses_db_row_not_disk_io_matches_decay_calculator` | **KEEP** | Strongest test in the file. 120-day file; asserts `expected < 1.0` (proves decay is meaningful, not a formula defect returning 1.0 always); tolerance check; then verifies end-to-end that `BuildContext` JSON contains `WeightedScore = CoOccurrenceCount × DecayWeight` with floating-point precision (≤1e-9). This last assertion is unique in the suite — no other test validates the `WeightedScore` formula in a real context payload. Removal would allow a decay integration bug to ship. |

---

## Recommendations

1. **Remove tests 4 and 5** (`mtime_unchanged` and `md5_matches`). Four dedicated test files already cover these guards with equal or better assertion depth. Keeping them in `sync_perf_indexing.rs` creates false confidence that this file "owns" guard coverage when it doesn't.

2. **Rename test 6** immediately — it is actively misleading:
   - Current: `generate_file_embedding_populates_vec_table_insert_or_replace_runs_to_completion`
   - Suggested: `sync_populates_vec_memory_files_with_non_trivial_onnx_embedding`
   - The phrase "insert_or_replace_runs_to_completion" is an implementation detail that will silently become incorrect if the SQL changes.

3. **Rewrite test 1** to assert something distinct from test 7. Currently both assert `(actual - expected).abs() <= 0.005` — the only difference is 60 days vs 120 days. If both are kept, test 1 should assert a value *range* that is only true at ~60 days (e.g., `actual > 0.85`) to give it fault-detection value independent of test 7.

4. **Consider consolidating the two decay tests (1 and 7)** into a single parameterized helper called from two test functions, each with different `days_ago` + distinct `expected_range` assertions. This removes duplication while preserving age-bracket coverage.

5. **The `T-0.1: RTM NFR-3` tag on tests 4 and 5** suggests they were planted as regression anchors when the sync module was refactored. Their purpose is now served by dedicated guard files; the tag no longer justifies presence here.

6. **Test 6 queries the DB directly** (`open_db`), bypassing any tool-layer abstraction. When FR-22 (`DbBackend` trait) ships, this test will need updating to work against the abstraction layer or against `SqliteBackend` explicitly. Flag for T-16.6 test migration.

---

## Load-Bearing Score: 4/7

Tests 2 (`persists_created`), 3 (`ctime_fallback`), 6 (`embedding blob`), and 7 (`decay + BuildContext integration`) are load-bearing. Test 1 is marginally load-bearing only if test 7 is removed. Tests 4 and 5 are fully covered by dedicated files and are not load-bearing in isolation.
