# Test Evaluation: repair_concepts_batch
**File:** `crates/maenifold-core/tests/repair_concepts_batch.rs`
**Test count:** 7
**Evaluated:** 2026-03-14
**Traceability:** T-3.2, RTM FR-6

---

## Summary

The strongest of the three files evaluated. All seven tests are load-bearing and exercise real behavioral contracts with correct assertions. One test (`repair_concepts_handles_whitespace_only_files_gracefully`) is genuinely distinct from the empty-file test only if the implementation handles empty vs. whitespace via different code paths — if it does not, these two can merge without coverage loss. The special-character and case-insensitivity tests are critical regression anchors that should never be removed.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `repair_concepts_can_replace_multiple_variants_in_one_operation` | KEEP | Tests multi-variant batch replacement in a single file. Verifies: all variants absent, canonical count matches (4), summary count correct. Precise count assertion (`== 4`) is intentional — the input has exactly 4 occurrences and the test should catch off-by-one errors. |
| `repair_concepts_modifies_multiple_files_in_a_single_batch` | KEEP | Tests cross-file batch processing. Critically includes a third file that should NOT be modified — this negative control is the most important assertion in the test. Without it, a "modify everything" bug would pass. |
| `repair_concepts_handles_empty_files_gracefully` | KEEP | Edge case that would catch a crash or a dirty write to an empty file. Correct. |
| `repair_concepts_handles_whitespace_only_files_gracefully` | MERGE with `repair_concepts_handles_empty_files_gracefully` | Behaviorally nearly identical to the empty-file test: no WikiLinks present, no modifications expected. Only distinct if whitespace triggers a different code path (e.g., a `trim().is_empty()` guard vs. `content.is_empty()` guard). If the implementation branches on these, keep separate; if not, collapse into a single `#[test]` with two inputs. In either case, the test is correct as written. |
| `repair_concepts_replaces_variants_with_special_characters` | KEEP | `c#`, `c++`, `.net` are realistic WikiLink names that could break a naive regex. This test is a regression anchor against escaping bugs. Do not remove. |
| `repair_concepts_matches_wikilinks_case_insensitively` | KEEP | Distinct from the special-character test. Tests case folding (`[[Tool]]`, `[[TOOL]]`, `[[tool]]` → `[[unified-tool]]`). Regression anchor for the case-insensitive match path. Both the "all variants replaced" and "count == 3" assertions are correct. |
| `repair_concepts_scans_nested_folder_depths` | KEEP | The only test that verifies recursive directory traversal. Without this, a bug that limits scan depth to 1 would not be caught by any other test in this file. The three-level structure (level1/level2/level3) provides adequate depth coverage. |

---

## Recommendations

1. **INVESTIGATE whether empty vs. whitespace-only files take different code paths** before merging tests 3 and 4. Run with `--nocapture` and add a temporary `eprintln!` to the handler to confirm. If both follow the same `content.is_empty()` or early-return branch, merge them into one test with a `for content in ["", "   \n\n   \t\n"]` loop. If they diverge, keep both and add a comment explaining why.

2. **Add a `dryRun: true` batch test**: All seven tests use `dryRun: false`. The batch behavior of dry-run mode (reports counts without modifying files) is not tested here. A single test verifying that files remain unchanged after a dry run of the multi-file case would close this gap.

3. **Consider a test for folder scoping**: `repair_concepts_modifies_multiple_files_in_a_single_batch` uses `"folder": "repair"` and `repair_concepts_scans_nested_folder_depths` also uses `"folder": "repair"`. Neither tests the behavior when `folder` is omitted entirely (scans all of `memory_path`) or when `folder` is set to a nonexistent path. If there is no existing test for these, add one.

4. **The `REPAIR_FIXTURE` constant in `support.rs` is unused by any test in this file**: The support module defines `REPAIR_FIXTURE` with a specific WikiLink pattern (`[[tools]]`, `[[coding-tool]]`, `[[tool system]]`), but none of the batch tests use it — they all write their own fixtures inline. Either use it in one of the tests or remove it to reduce dead code confusion.

5. **Naming accuracy on test 1**: `repair_concepts_can_replace_multiple_variants_in_one_operation` — consider `repair_concepts_replaces_all_listed_variants_in_single_file` to distinguish it from test 2 (multi-file batch). "One operation" is ambiguous; it could mean "single invocation" (which all tests use anyway) or "single file" (which is the distinguishing characteristic).

---

## Load-Bearing Score: 7/7

Every test would allow a real bug to ship if removed. The whitespace-only test gets a conditional asterisk — it drops to 6/7 if the implementation treats `""` and `"   \n"` identically throughout.

---

## SWE Remediation

- **MERGE** `repair_concepts_handles_empty_files_gracefully` + `repair_concepts_handles_whitespace_only_files_gracefully` → `repair_concepts_handles_empty_and_whitespace_only_files_gracefully`: Confirmed both follow same code path (zero regex matches → `edits.is_empty()` early return → `replacement_count == 0` skip). Collapsed into single test iterating over `[("empty", ""), ("whitespace-only", "   \n\n   \t\n")]`.
- **RENAME** `repair_concepts_can_replace_multiple_variants_in_one_operation` → `repair_concepts_replaces_all_listed_variants_in_single_file` per P3 naming accuracy recommendation.
- **ADD** `repair_concepts_dry_run_reports_counts_without_modifying_files`: New dry-run batch test per P1 recommendation. Verifies multi-file scenario with `dryRun: true` — files remain unchanged, replacement count reported, output indicates dry-run mode.
- **SKIP** P2 folder scoping: Already tested in `repair_concepts_replacement.rs` (`repair_concepts_dry_run_with_invalid_folder_returns_error_output`).
- **SKIP** P3 REPAIR_FIXTURE cleanup: Constant is used by `repair_concepts_replacement.rs`; not dead code at module level.
- Test count: 7 (was 7 — 2 merged into 1, 1 added)
- Commit: c370c84

---

## Red-team Audit

- ✅ RENAME `repair_concepts_can_replace_multiple_variants_in_one_operation` → `repair_concepts_replaces_all_listed_variants_in_single_file`: only function name changed, body/assertions untouched, matches eval P3 recommendation #5
- ✅ MERGE `repair_concepts_handles_empty_files_gracefully` + `repair_concepts_handles_whitespace_only_files_gracefully` → `repair_concepts_handles_empty_and_whitespace_only_files_gracefully`: correctly collapses into parameterized loop `for (label, content) in [("empty", ""), ("whitespace-only", "   \n\n   \t\n")]` with fresh `isolated_config()` per iteration; code-path investigation documented in merge comment; both original assertion categories preserved (content unchanged + "Files modified: 0")
- ✅ ADD `repair_concepts_dry_run_reports_counts_without_modifying_files`: satisfies P1 recommendation — multi-file scenario (3 files, 2 matching, 1 non-matching), `dryRun: true`, asserts all files unchanged via `assert_eq!`, asserts replacement count reported, asserts `DRY RUN` indicator present
- ✅ SKIP P2 folder scoping: verified `repair_concepts_replacement.rs:142` already tests `repair_concepts_dry_run_with_invalid_folder_returns_error_output` with nonexistent folder path
- ✅ SKIP P3 REPAIR_FIXTURE cleanup: verified `REPAIR_FIXTURE` used at 4 call sites in `repair_concepts_replacement.rs` (lines 13, 48, 76, 77) — not dead code at module level
- ✅ KEEP tests untouched: `modifies_multiple_files_in_a_single_batch`, `replaces_variants_with_special_characters`, `matches_wikilinks_case_insensitively`, `scans_nested_folder_depths` — no assertion changes (appear only as diff hunk context headers)
- ✅ Test count: 7 → 7 (2 merged into 1, 1 added). Arithmetic confirmed.
- ✅ No unrelated changes: commit c370c84 touches only `repair_concepts_batch.rs` (1 file, 71 insertions, 45 deletions)
- Verdict: **PASS**

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- repair_concepts_batch`: 7 passed, 0 failed
- Verdict: PASS
