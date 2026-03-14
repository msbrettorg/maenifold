# Test Evaluation: recent_activity_pipeline_tool
**File:** `crates/maenifold-core/tests/recent_activity_pipeline_tool.rs`
**Test count:** 7
**Evaluated:** 2026-03-14
**Traceability:** T-2.3 / FR-4

---

## Summary

These are full-pipeline integration tests: they seed raw files on disk, run `Sync` through the tool registry, then invoke `RecentActivity`. The approach is sound, but **four of the seven tests duplicate behavior already covered by `recent_activity_boundary_queries.rs`** (which seeds directly into the DB, making it faster and more targeted). The two tests with genuine pipeline-only value are the negative-timespan error and the end-to-end timespan exclusion after Sync.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `recent_activity_negative_timespan_returns_error` | **KEEP** | Only test exercising the negative-timespan validation path through the real `tools::recent_activity()` entry point. Behavioral assertion on `MaenifoldError::InvalidArgument` and message content. Removing it would allow a sign-check regression to ship silently. |
| `recent_activity_no_database_returns_no_database_error` | **KEEP** | Validates the no-DB guard through the full tool call chain. `collect_recent_activity_returns_error_when_no_database_exists` covers the same at the function level — this adds marginal value by ensuring ToolRegistry dispatch and error propagation also work. Overlap is acceptable. |
| `recent_activity_learn_mode_returns_help_content` | **REWRITE** | Assertions are too weak: `result.len() > 20` is noise; `result.contains("RecentActivity")` could be satisfied by an error string. Should assert specific expected section headers from the help doc (e.g., `contains("timespan")`, `contains("filter")`) so an accidentally truncated or swapped help file actually fails this test. |
| `recent_activity_normal_query_returns_formatted_report` | **KEEP** | One of two pipeline tests with genuine Sync-path value. Seeds both a sequential file and a memory file, runs Sync, verifies both appear in output. Confirms file classification round-trips correctly through disk → Sync → DB → RecentActivity. However, rename is warranted: name implies it covers all formatting, it only covers presence. |
| `recent_activity_filter_thinking_excludes_memory_files` | **MERGE** with `recent_activity_boundary_queries.rs::recent_activity_filter_variants_select_expected_path_families` | The filter SQL logic is fully tested at boundary (faster, direct DB). The only thing this adds is that Sync correctly categorises the paths — but `recent_activity_normal_query_returns_formatted_report` already proves Sync works. Running Sync twice for filter coverage is not load-bearing. If merged, seed both thinking and memory file types in the boundary test's support fixture. |
| `recent_activity_limit_returns_at_most_limit_entries` | **MERGE** with `recent_activity_boundary_queries.rs::recent_activity_limit_caps_number_of_results` | Boundary already tests `limit: 2` with `rendered_item_count`. Pipeline adds nothing new — the SQL `LIMIT` clause doesn't behave differently when rows came from Sync. Merge or delete; prefer deletion since boundary coverage is superior (direct DB seeds, faster, no Sync dependency). |
| `recent_activity_timespan_filter_excludes_old_entries_end_to_end` | **KEEP** | Only pipeline test that exercises the interaction between Sync and `set_last_indexed`. Critical question answered: does running Sync reset the `last_indexed` timestamp we manually backdated? If it does, the filter would never exclude anything. This is unique value the boundary tests cannot provide. |

---

## Recommendations

1. **Rewrite `learn_mode` assertions**: replace `len() > 20` with checks for at least two specific expected headings or parameter names from the actual help document (e.g., `"timespan"`, `"filter"`, `"limit"`). This would catch a help file that was accidentally blanked or swapped.

2. **Delete or merge `filter_thinking` and `limit` pipeline tests**: their coverage is already stronger in `recent_activity_boundary_queries.rs`. Keeping duplicates burns test time (each runs Sync against a temp DB) and blurs ownership of which layer owns which assertion.

3. **Rename `recent_activity_normal_query_returns_formatted_report`**: the name overclaims. Suggest `recent_activity_pipeline_sync_produces_output_for_both_file_types` to be precise about what is actually asserted.

4. **Add parameterized error-path tests**: negative timespan, zero limit, empty timespan string, and invalid filter string are all distinct `InvalidArgument` paths. A table-driven approach would consolidate them and surface coverage gaps.

---

## Load-Bearing Score: 5/7

> `learn_mode` (weak assertions) and `limit` (fully covered at boundary) are not load-bearing in practice.
