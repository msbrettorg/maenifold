---
id: recent_activity_pipeline_collect
status: done
pipeline: tdd
---

# Test Evaluation: recent_activity_pipeline_collect
**File:** `crates/maenifold-core/tests/recent_activity_pipeline_collect.rs`
**Test count:** 7
**Evaluated:** 2026-03-14
**Traceability:** T-2.3 / FR-4

---

## Summary

These tests exercise `collect_recent_activity_records()` — the exported public function one layer below the tool entry point — by seeding real files, running Sync, and calling the function directly. The test suite is coherent and well-targeted: each test isolates one orthogonal axis (filter variant, timespan, limit, error path). Assertions are precise and behavioral throughout. The main question to raise: is the Sync-based seeding approach adding value over direct DB seeding, and is there true overlap with `recent_activity_boundary_queries.rs`? **Answer: No — the boundary file tests `tools::recent_activity()` via `ToolRegistry`, not `collect_recent_activity_records()` directly.** The collect function's public API contract is only covered here.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `collect_recent_activity_returns_all_files_when_filter_is_null` | **KEEP** | Only test verifying `filter = None` returns both sequential and memory file types from `collect_recent_activity_records`. Checks both presence of expected paths and `results.len() >= 2`. Behavioral and necessary. |
| `collect_recent_activity_filter_thinking_returns_only_thinking_files` | **KEEP** | Tests the "thinking" filter SQL clause (`file_path LIKE 'memory://thinking/%'`) at the collect function level. Presence + exclusion assertions. Unique — no other test file targets this function with this filter. |
| `collect_recent_activity_filter_memory_excludes_thinking_and_chat_files` | **KEEP** | The "memory" filter SQL has **two** exclusion conditions (`NOT LIKE thinking` AND `NOT LIKE chat`). This is the only test that validates both halves of that conjunction. Seeds all three file types and asserts each category correctly. High value, would catch a partial regression in the SQL clause. |
| `collect_recent_activity_filter_assumptions_returns_chat_path_files` | **KEEP** | Tests the "assumptions" filter (`file_path LIKE 'memory://chat/%'`). Seeds a chat file and a memory file, verifies only the chat file is returned. Orthogonal to the other filter tests. |
| `collect_recent_activity_timespan_filter_excludes_old_entries` | **KEEP** | Passes `TimeDelta::hours(24)` directly (not a parsed string), testing the timespan logic in isolation from the string parser. Complements `pipeline_tool.rs::recent_activity_timespan_filter_excludes_old_entries_end_to_end`. Together they cover both the parsed-string and programmatic-API entry points. |
| `collect_recent_activity_limit_respects_max_results` | **KEEP** | Seeds 5 files, calls with `limit = 2`, asserts `results.len() == 2`. This is the most precise limit test in the suite — a direct `Vec` length assertion is stronger than `rendered_item_count` on a formatted string. Not redundant. |
| `collect_recent_activity_returns_error_when_no_database_exists` | **KEEP** | Tests the `recent_activity_table_exists` guard inside `collect_recent_activity_records_with_filter`. Verifies `MaenifoldError::Message` with the correct message string. Orthogonal to `pipeline_tool.rs` version which tests the same through ToolRegistry. |

---

## Recommendations

1. **All 7 tests earn KEEP**, but this file is a candidate for **parameterization of the four filter tests** (`null`, `thinking`, `memory`, `assumptions`). Each follows an identical structure: seed files → sync → collect with a filter → assert expected paths present/absent. A single table-driven test with four rows would reduce maintenance surface while preserving coverage:
   ```rust
   // Pseudo-sketch
   for (filter, should_contain, should_not_contain) in cases {
       let results = collect_recent_activity_records(&cfg, None, filter, 50)?;
       ...
   }
   ```
   This is worth doing if maintaining the test file becomes a friction point, but is not urgent.

2. **`collect_recent_activity_filter_memory_excludes_thinking_and_chat_files`** implicitly assumes that `seed_chat_file` produces a path under `memory://chat/`. Consider adding an explicit assertion that the seeded chat file **is** included in the `assumptions` filter (a positive check) to complement the negative check. Currently, if chat files were excluded from both `memory` and `assumptions`, this test would still pass.

3. **Verify the `>= 2` floor in `returns_all_files_when_filter_is_null`**: the comment in the assertion references "at least two results" but says nothing about what the two _are_. Consider asserting `results.len() == 2` after a clean isolated env, or document explicitly that other seeding from asset init is expected. The `>= 2` floor masks accidental over-seeding.

4. **No coverage of the invalid `filter` string path** (`collect_recent_activity_records(&cfg, None, Some("bogus"), 50)`). This should return `MaenifoldError::InvalidArgument`. Add one test or fold it into `pipeline_tool.rs` negative tests.

---

## Load-Bearing Score: 7/7

> Every test in this file covers a distinct, meaningful axis of `collect_recent_activity_records`. None are redundant with each other; none are label-checking.

---
## SWE Remediation
- KEEP all 7 existing tests: no modifications (all verdicts KEEP)
- ADD `collect_recent_activity_rejects_invalid_filter_string`: covers recommendation 4 — validates `MaenifoldError::InvalidArgument` for bogus filter values, asserts both "Invalid filter" message and offending value echo
- SKIP recommendation 1 (parameterization): eval notes "not urgent", low friction currently
- SKIP recommendations 2–3: would modify KEEP tests; improvements are cosmetic
- Test count: 7 → 8 (all passing)
- Commit: 28a0c3c

---
## Red-team Audit
- ✅ KEEP 7 original tests: diff is pure addition (23 lines appended), zero modifications to existing test bodies — verified via `git show 28a0c3c`
- ✅ ADD `collect_recent_activity_rejects_invalid_filter_string`: matches eval recommendation 4 exactly — calls `collect_recent_activity_records(&cfg, None, Some("bogus"), 50)`, asserts `MaenifoldError::InvalidArgument`, validates both "Invalid filter" substring and offending value "bogus" echo
- ✅ SKIP R1 (parameterization): eval explicitly notes "not urgent" — justified
- ✅ SKIP R2–R3: eval marked these as improvements to KEEP tests, not required changes — SWE correctly avoids modifying KEEP-verdict tests
- ✅ Traceability: new test carries `T-2.3: RTM FR-4` comment, consistent with file header and eval metadata
- ✅ No unrelated changes: commit touches only the test file and eval spec (2 files, 31 insertions)
- ✅ All 8 tests pass: `cargo test -p maenifold-core collect_recent_activity` — 8 passed, 0 failed
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- recent_activity_pipeline_collect`: 8 passed, 0 failed
- Verdict: PASS
