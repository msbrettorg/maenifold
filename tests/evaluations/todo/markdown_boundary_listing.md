# Test Evaluation: markdown_boundary_listing

**File:** `crates/maenifold-core/tests/markdown_boundary_listing.rs`
**Test count:** 11
**Evaluated:** 2026-07-10
**Traceability:** T-W1-2 (RTM FR-28.5, FR-28.6, FR-28.7, FR-28.8)

---

## Summary

Seven of eleven tests are genuinely load-bearing. One test is outright redundant (`fr28_5_ac4`), and three tests (`fr28_6_*`) exercise a **test-only support helper** (`get_memory_directories` in `support.rs`) rather than any production API — the production `list_memories` tool uses `fs::read_dir` directly and shares no code path with the helper under test.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `fr28_5_ac1_get_memory_files_returns_memory_uris` | REWRITE | `assert!(files.len() >= 3)` is weak in an isolated env; should be `== 3`. Any leak from a prior test (or the env owning background files) silently passes. Assert exact count. |
| `fr28_5_ac2_get_memory_files_excludes_hidden_files` | KEEP | Directly exercises `is_hidden_path()` filtering on `.obsidian/`; if hidden-path logic regresses, only this test fails. |
| `fr28_5_ac3_get_memory_files_folder_uri_filter_returns_only_subset` | KEEP | Asserts `== 2` (exact count) and validates folder-scoped filtering. Strong and unique. |
| `fr28_5_ac4_get_memory_files_null_filter_returns_all_files` | REMOVE | Redundant with `fr28_5_ac1`. Test 1 already passes `None`, seeds 3 files including a nested path, and verifies all are returned. The `>= 2` assertion here adds zero signal. |
| `fr28_6_ac1_get_memory_directories_returns_memory_uris` | REWRITE | Calls `get_memory_directories` from `support.rs` — a **test-only function** not present in production. `list_memories` uses `fs::read_dir` directly. These tests exercise the support helper's correctness, not any production directory-listing contract. Rewrite against the production `list_memories` tool or acknowledge this is testing the support helper explicitly. |
| `fr28_6_ac2_get_memory_directories_excludes_hidden_dirs` | REWRITE | Same concern: test target is a support-only function. However, it does indirectly test `is_hidden_path` through `get_memory_files`. If the goal is testing hidden-dir exclusion in directory context, make the intent explicit and test the right entry point. |
| `fr28_6_ac3_get_memory_directories_folder_uri_filter_returns_only_subset` | REWRITE | Same concern. Folder-filter on directories is a valid requirement, but the tested code path (`support.get_memory_directories`) is not the production implementation. Tests may pass while production `list_memories` has a different bug. |
| `fr28_7_ac1_file_exists_existing_file_returns_true` | KEEP | Direct test of `markdown::file_exists` happy path. Unique, load-bearing. |
| `fr28_7_ac2_file_exists_non_existent_file_returns_false` | KEEP | Covers the false branch. Could be MERGED with `ac1` into one test, but the two-test split is reasonable. |
| `fr28_8_ac1_delete_file_existing_file_removes_file` | KEEP | Verifies the postcondition (`file_exists == false`) via the same API; catches bugs where `delete_file` returns `Ok` but doesn't actually remove the file. |
| `fr28_8_ac2_delete_file_non_existent_file_returns_not_found_error` | REWRITE | The assertion `assert_message_error(err, "Memory file not found")` pins a specific error *message substring*. If the message changes to `"No file at memory://never-existed"`, this test breaks for the wrong reason. Assert the `MaenifoldError::Message` variant is returned (not `Ok`, not a panic), then separately validate the URI is echoed — or elevate this to a `NotFound` error variant so tests can check type, not text. |

---

## Recommendations

1. **Remove `fr28_5_ac4`** — it is covered by `fr28_5_ac1` and its `>= 2` guard is meaningless.
2. **Fix `fr28_5_ac1` assertion** — change `assert!(files.len() >= 3)` to `assert_eq!(files.len(), 3)`. An isolated temp env should contain exactly the seeded files; a loose lower-bound check silently tolerates test contamination.
3. **Rewrite or rename the `fr28_6_*` trilogy** — either (a) test the production `list_memories` tool output for directory presence, or (b) rename the test file to make it explicit that it's a unit test of the `support::get_memory_directories` helper used only by other test modules. Currently it masquerades as an FR-28.6 acceptance test while the production implementation uses completely different code.
4. **`fr28_8_ac2` error assertion** — introduce a `NotFound` error variant or at minimum assert `is::<MaenifoldError::Message>()` separately from message content, so a message wording change doesn't silently pass or fail the test.
5. **Parameterization opportunity** — `fr28_7_ac1` and `fr28_7_ac2` could be a single `#[rstest]` table with `(uri, exists_in_env, expected)` rows. Similarly `fr28_8_*`. Low priority given small count.

---

## Load-Bearing Score: 7/11

Tests 1 (with caveat), 2, 3, 7, 8, 9, 10 are genuinely load-bearing. Test 4 is redundant. Tests 5, 6, 7 (the `fr28_6_*` group) exercise a test-only helper rather than a production API surface; their load-bearing value for the actual product is indirect at best.
