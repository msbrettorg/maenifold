# Test Evaluation: memory_status_parity
**File:** `crates/maenifold-core/tests/memory_status_parity.rs`
**Test count:** 7
**Evaluated:** 2026-03-14
**Traceability:** T-13.4 / FR-7 / NFR-1

---

## Summary

This file tests `MemoryStatus` output against the C# oracle parity contract. The intent is correct, but **five of seven tests stop at checking that a label string is present** rather than asserting the value next to the label. A test that checks `result.contains("Total folders:")` is asserting the format, not the behavior — it would pass if the implementation printed `"Total folders: ERROR"` or `"Total folders: 0"` regardless of what exists on disk. Only `memory_status_without_db_shows_not_initialized` and `memory_status_has_correct_header` make assertions that are genuinely behavioral. The parity file name is aspirational; the actual coverage is mostly structural.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `memory_status_contains_folder_count` | **REWRITE** | Creates exactly 1 subdirectory with 1 file. Asserts `"Total folders:"` is present and `"Total folders: 0"` is absent. **Does not assert `"Total folders: 1"`**. A bug that returns `"Total folders: 99"` passes. Should assert the exact count or a `>= 1` parse of the number from the output. |
| `memory_status_contains_total_size` | **REWRITE** | Writes a file, then asserts `"Total size:"` and `"MB"` exist. **Does not assert the value is non-zero.** `"Total size: 0.00 MB"` passes this test. Should assert the parsed float is `> 0.0`. The `"MB"` check is testing format, not parity. |
| `memory_status_contains_graph_stats` | **REWRITE** | Checks three label strings (`"### Graph Database"`, `"- Concepts:"`, `"- Relations:"`, `"- Mentions:"`). No value assertions at all — an implementation that printed `"- Concepts: ?"` would pass. At minimum should parse and assert the values are non-negative integers, even if zero in an empty DB. |
| `memory_status_contains_db_size` | **MERGE** with `memory_status_contains_graph_stats` | Both test the `### Graph Database` section of the same output. `"Database size:"` and `"MB"` label checks are the same class of weak assertion as in `contains_total_size`. Merge these two into a rewritten `memory_status_graph_section_reports_valid_counts_and_sizes` that asserts parseable numbers. |
| `memory_status_without_db_shows_not_initialized` | **KEEP** | Only test for the no-DB code path. Asserts the exact expected message `"Not initialized (run sync first)"`. Behavioral, precise, load-bearing — a regression in that else-branch produces no feedback without this test. |
| `memory_status_has_correct_header` | **KEEP** | Asserts `result.starts_with("# Memory System Status")`. This is a format check, but the header is part of the C# oracle parity contract (NFR-1) and determines how downstream clients (MCP consumers, SRE agents) parse the response. A regression here is a real compatibility bug. |
| `memory_status_file_count_reflects_files` | **REWRITE** | Writes 3 files, asserts `"Total files:"` and not `"Total files: 0"`. Comment in the test explains asset initialisation inflates the count, hence `!= 0` instead of `== 3`. The workaround is understandable but the test is still weaker than it should be. Should count assets before and after, or assert `>= 3`. A `>= 3` check would actually verify the 3 written files are counted. |

---

## Recommendations

1. **Rewrite `contains_folder_count` to assert the count is `1`** after creating exactly one subdirectory in a clean `isolated_env`. The env starts empty (besides assets which live in a sibling dir). A `walkdir`-counting regression to any value other than 1 would then fail.

2. **Rewrite `contains_total_size` to parse the float from the output** and assert it is `> 0.0`. Pattern: extract the substring between `"Total size: "` and `" MB"`, parse as `f64`, assert `> 0.0`. This is the difference between testing the C# parity contract and testing that the label string compiled.

3. **Rewrite `contains_graph_stats`**: after `isolated_env()` initializes the DB schema, concept/relation/mention counts should be zero. Assert exactly `"- Concepts: 0"`, `"- Relations: 0"`, `"- Mentions: 0"`. This would catch an implementation that queries the wrong table or omits the query entirely.

4. **Merge `contains_db_size` into a rewritten `contains_graph_stats`**: after initialization the DB file exists and has a non-zero size. Parse and assert `db_size_mb > 0.0`. The two assertions (graph counts + DB size) are naturally co-located in the `### Graph Database` section and share setup.

5. **Rewrite `file_count_reflects_files`**: call `isolated_env()`, count the markdown files it seeds (consult `AssetManager::initialize_assets` to know that count), write 3 more, assert `>= baseline + 3`. This eliminates the hand-wavy "at least not zero" check and makes the test actually reflect what the method name claims.

6. **Consider adding a `memory_status_file_count_excludes_non_md_files` test**: the implementation explicitly filters to `.md` only (T-16.2 / NFR-1 parity). No test currently verifies that a `.txt` or `.json` file in the memory root is excluded from the file count. This is a meaningful parity edge case.

---

## Load-Bearing Score: 4/7

> `without_db_shows_not_initialized` and `has_correct_header` are genuinely load-bearing. `file_count` and `folder_count` are marginally load-bearing (they detect a count going to zero). The three label-only tests (`total_size`, `graph_stats`, `db_size`) are not load-bearing — an implementation that prints correct labels with wrong values passes them all.

---
## SWE Remediation
- REWRITE `memory_status_contains_folder_count`: now parses integer from output and asserts exact count == 1 after creating one subdirectory (was label-only check)
- REWRITE `memory_status_contains_total_size`: writes 6KB file (exceeds 0.005 MB threshold for `{:.2}` formatting), parses float from output, asserts > 0.0 (was label-only check)
- REWRITE `memory_status_file_count_reflects_files`: parses integer from output, asserts >= 3 after writing 3 .md files (was `!= 0` label check)
- MERGE `memory_status_contains_graph_stats` + `memory_status_contains_db_size` → `memory_status_graph_section_reports_valid_counts_and_sizes`: asserts exact `"- Concepts: 0"`, `"- Relations: 0"`, `"- Mentions: 0"` in fresh DB; parses DB size float and asserts > 0.0
- REMOVE `memory_status_contains_db_size`: subsumed by merged test
- KEEP `memory_status_without_db_shows_not_initialized`: unchanged
- KEEP `memory_status_has_correct_header`: unchanged
- Added `parse_label_int` and `parse_label_float` helper functions for value extraction from formatted output
- Test count: 7 → 6 (merge reduced by 1)
- Commit: 3b78900

---
## Red-team Audit
- ✅ REWRITE `memory_status_contains_folder_count`: `parse_label_int` extracts integer, `assert_eq!(folder_count, 1)` — exact count per eval recommendation 1. Matches spec.
- ✅ REWRITE `memory_status_contains_total_size`: 6KB file exceeds `{:.2}` rounding threshold (smart), `parse_label_float` extracts value, asserts `> 0.0`. Matches eval recommendation 2 pattern exactly.
- ✅ REWRITE `memory_status_file_count_reflects_files`: `parse_label_int` extracts count, asserts `>= 3`. Eval offered `>= baseline + 3` (stronger) or `>= 3` (acceptable). SWE chose the simpler variant. Matches spec.
- ✅ MERGE `contains_graph_stats` + `contains_db_size` → `memory_status_graph_section_reports_valid_counts_and_sizes`: new name matches eval exactly. Asserts `"- Concepts: 0"`, `"- Relations: 0"`, `"- Mentions: 0"` (rec 3). Parses DB size float, asserts `> 0.0` (rec 4). Section header check retained. All original coverage preserved.
- ✅ REMOVE `memory_status_contains_db_size`: confirmed deleted from entire test tree (`grep` returns 0 matches). DB size assertion subsumed by merged test — no unique coverage lost.
- ✅ KEEP `memory_status_without_db_shows_not_initialized`: unchanged in diff, verified in code.
- ✅ KEEP `memory_status_has_correct_header`: unchanged in diff, verified in code.
- ✅ Helper functions `parse_label_int` / `parse_label_float`: clean utilities, well-documented, no hidden logic.
- ✅ Test count: 6 `#[test]` functions confirmed (was 7). Merge reduced by exactly 1.
- ✅ No unrelated changes: diff touches only `memory_status_parity.rs`, 66 insertions / 53 deletions.
- ✅ All 6 tests pass (`cargo test -p maenifold-core memory_status` — 9/9 ok including related modules).
- ℹ️ Eval recommendation 6 (add `file_count_excludes_non_md_files`) was advisory ("Consider adding"), not a verdict-level action. Acceptable to omit from this remediation.
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_status_parity`: 6 passed, 0 failed
- Verdict: PASS
