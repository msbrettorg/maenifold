---
id: T-80.162
title: "Test Evaluation: sync_perf_baseline"
status: done
pipeline: tdd
---

# Test Evaluation: sync_perf_baseline
**File:** `crates/maenifold-core/tests/sync_perf_baseline.rs`
**Test count:** 8
**Evaluated:** 2026-06-18
**Traceability:** T-0.1 · NFR-3

---

## Summary

This file harbors a structural identity crisis: tests 1–2 are integration tests of `sync_one`/`sync_all` that *claim* to measure timing but assert no timing thresholds, while tests 3–8 are unit tests of **test-helper functions** defined locally in `sync_perf_support.rs` — not production code. Of the six "unit" tests, only two (`extract_wikilinks_with_sections_*`) directly exercise a production function; the other four test the correctness of the local counting utilities that cross-check sync outputs elsewhere. This is a significant misrepresentation of what the file actually guards.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `baseline_process_file_timing_per_file` | **REWRITE** | Misleading name: no timing threshold is asserted. `eprintln!` output goes to stderr and is invisible in CI unless the run fails. The *only* real assertions are `output.contains("Synced memory://perf-baseline/")` (functional, valid) and `file_content_row_count == 10` (functional, valid). **Rename** to `sync_single_file_indexes_all_files_in_corpus` and remove the dead timing boilerplate, or add an actual `assert!(median_ms < BUDGET_MS)` with a documented budget. |
| `baseline_full_sync_timing_over_corpus` | **REWRITE** | Same problem: timing is logged but never asserted. The assertions (`"Sync complete:"`, `"files processed"`) are minimal functional checks. **Rename** to `sync_all_completes_over_ten_file_corpus` and either enforce a timing budget or drop the timing instrumentation entirely. A test that measures but never fails on measurements is not a performance gate — it is a functional test with noisy output. |
| `count_concept_occurrences_golden_alpha_two_beta_one` | **REWRITE** (clarify subject) | This tests the **local test helper** `count_visible_wikilink_occurrences` defined in `sync_perf_support.rs`, not a production function. That helper is used as a cross-check oracle in other `sync_perf_*.rs` files. Testing the oracle itself has value, but the test name implies it is testing production counting logic. **Rename** to `helper_count_visible_wikilinks_alpha_two_beta_one` to make the subject clear. Assertions are correct and load-bearing for the helper's validity. |
| `extract_wikilinks_with_sections_group_by_count_documents_relation_to_count_occurrences` | **KEEP** (rename) | This is the most important test in the file. It documents a key behavioral contract of the production function `extract_wikilinks_with_sections`: it returns **one entry per concept per section** (deduplicated within a section), *not* one entry per raw occurrence. The contrast with `count_visible_wikilink_occurrences` (which counts all occurrences) is the entire point. This distinction would be easy to misunderstand when reading the code. **Rename** to `extract_wikilinks_with_sections_deduplicates_per_section_unlike_occurrence_count`. |
| `count_concept_occurrences_across_sections_counts_all_occurrences` | **MERGE** with `count_concept_occurrences_golden_alpha_two_beta_one` | Both test the same local helper function `count_visible_wikilink_occurrences` on static string content. The only difference is that this one uses multi-section content. These two could be one parameterized test, or if kept separate, test #3 should be renamed (as above) and this test clarified. Currently load-bearing only for the helper, not production code. |
| `extract_wikilinks_with_sections_assigns_section_indices_correctly` | **KEEP** | Tests the production function `maenifold_core::markdown::extract_wikilinks_with_sections` directly, verifying that preamble content gets `section_index=0` and subsequent H2 sections get 1, 2, etc. Load-bearing: a regression in section indexing would corrupt the graph's section-weighted co-occurrence model. Assertions are against exact values. |
| `count_all_concept_occurrences_matches_per_concept_counts_for_all_concepts` | **REWRITE** (clarify subject) | Tests the **local test helper** `count_all_visible_wikilink_occurrences` is consistent with calling `count_visible_wikilink_occurrences` per concept. The consistency assertion is valid (both use the same underlying function). The inline `code` comment `[[alpha]]` should NOT count` is a noteworthy boundary — but the test does **not actually assert it**: inline code stripping is tested implicitly by the content string, but there is no isolated assertion for the inline-code-exclusion behavior. **Split out a dedicated test** for inline code exclusion, or the boundary comment is silent documentation that a future refactor could break without a red test. |
| `count_all_concept_occurrences_returns_zero_for_absent_concepts` | **KEEP** | Tests that the local oracle returns `0` for concepts that appear nowhere. Necessary to guard other `sync_perf` tests that rely on `0` counts as proof a concept was not indexed. Assertions are exact. |

---

## Recommendations

1. **Decide: performance gate or functional test.** Tests 1 and 2 currently do both jobs poorly. Either commit to a timing budget (`assert!(median_ms < 200, "single-file sync regressed: {median_ms}ms")`) and document the machine baseline in a comment, or strip the timing instrumentation and rename them as pure functional tests. The current hybrid state means they can never fail on performance, only on correctness.

2. **Rename tests 3, 5, 7** to prefix with `helper_` or `oracle_` to signal they test local test infrastructure, not production code. This prevents future contributors from treating them as coverage of `maenifold_core` behavior.

3. **Add an explicit inline-code exclusion test.** Test 7 contains the comment `// Inline code [[alpha]] should NOT count` but asserts `actual.get("alpha") == Some(&3)` — which means the inline code was indeed excluded and the count is correct. But there is no isolated test for *just* the code-block stripping behavior of `visible_text_without_code`. A dedicated test like:
   ```rust
   fn count_visible_wikilinks_skips_inline_code_spans() {
       let content = "[[real]] and `[[fake]]`";
       assert_eq!(count_visible_wikilink_occurrences(content, "real"), 1);
       assert_eq!(count_visible_wikilink_occurrences(content, "fake"), 0);
   }
   ```
   would make the boundary explicit and catch regressions in the code-stripping logic.

4. **Tests 4 and 6 (production function tests) should migrate to a dedicated file** such as `markdown_wikilink_section_tests.rs` or be consolidated into `wikilink_filter_unit_tests.rs`. They are mishoused in a "perf baseline" file.

5. **Tests 3 and 5 are parameterizable.** Both test `count_visible_wikilink_occurrences` with simple content strings and known counts. They could collapse into a single test with multiple `assert_eq!` calls or a data-driven table, reducing noise without losing coverage.

---

## Load-Bearing Score: 5/8

> Tests 1 and 2 assert no timing thresholds (misleading framing). Tests 3, 5, and 7 test local helper functions, not production code — they are "testing the tests." Only tests 4, 6, and 8 are unambiguously load-bearing for production behavior.

---

## SWE Remediation

- REWRITE `baseline_process_file_timing_per_file` → `sync_single_file_indexes_all_files_in_corpus`: stripped dead timing instrumentation (Instant, per_file_ms, eprintln), kept functional assertions (sync output contains URI, file_content_row_count == 10)
- REWRITE `baseline_full_sync_timing_over_corpus` → `sync_all_completes_over_ten_file_corpus`: stripped dead timing instrumentation, kept functional assertions (Sync complete, files processed)
- MERGE `count_concept_occurrences_golden_alpha_two_beta_one` + `count_concept_occurrences_across_sections_counts_all_occurrences` → `helper_count_visible_wikilinks_single_and_multi_section`: combined single-section and multi-section cases into one test with `helper_` prefix per P0 rec 2
- KEEP (rename) `extract_wikilinks_with_sections_group_by_count_documents_relation_to_count_occurrences` → `extract_wikilinks_with_sections_deduplicates_per_section_unlike_occurrence_count`: logic and assertions unchanged
- KEEP `extract_wikilinks_with_sections_assigns_section_indices_correctly`: unchanged
- REWRITE `count_all_concept_occurrences_matches_per_concept_counts_for_all_concepts` → `helper_count_all_occurrences_matches_per_concept_counts`: added `helper_` prefix per P0 rec 2, assertions unchanged
- NEW `helper_count_visible_wikilinks_skips_inline_code_spans`: explicit inline-code exclusion test per P0 rec 3
- KEEP `count_all_concept_occurrences_returns_zero_for_absent_concepts`: unchanged
- Skipped rec 4 (migrate tests 4/6 to dedicated file) — P3, structural move outside eval scope
- Commit: eef9d64

---

## Red-team Audit

| Remediation Item | Verdict | Notes |
|---|---|---|
| REWRITE `baseline_process_file_timing_per_file` → `sync_single_file_indexes_all_files_in_corpus` | ✅ | Dead timing instrumentation removed (`Instant`, `per_file_ms`, `sort_unstable`, `eprintln`). Functional assertions preserved verbatim (`"Synced memory://perf-baseline/"`, `file_content_row_count == 10`). Name matches eval spec. |
| REWRITE `baseline_full_sync_timing_over_corpus` → `sync_all_completes_over_ten_file_corpus` | ✅ | Dead timing instrumentation removed (`Instant`, `elapsed_ms`, `eprintln`). Functional assertions preserved verbatim (`"Sync complete:"`, `"files processed"`). Name matches eval spec. |
| MERGE tests 3+5 → `helper_count_visible_wikilinks_single_and_multi_section` | ✅ | All four original assertions preserved: alpha=2, beta=1 (from test 3), gamma=3, delta=1 (from test 5). `helper_` prefix applied per rec 2. No coverage lost. |
| KEEP (rename) → `extract_wikilinks_with_sections_deduplicates_per_section_unlike_occurrence_count` | ✅ | Body byte-identical to original. Only the function name changed. Name matches eval spec exactly. |
| KEEP `extract_wikilinks_with_sections_assigns_section_indices_correctly` | ✅ | Unchanged. |
| REWRITE → `helper_count_all_occurrences_matches_per_concept_counts` | ✅ | Only the function name changed (added `helper_` prefix, shortened). All 7 assertions intact and identical. |
| NEW `helper_count_visible_wikilinks_skips_inline_code_spans` | ✅ | Content string and assertions match eval rec 3 exactly: `"[[real]] and \`[[fake]]\`"`, real=1, fake=0. Makes inline-code-exclusion boundary a red-test-guarded contract. |
| KEEP `count_all_concept_occurrences_returns_zero_for_absent_concepts` | ✅ | Unchanged. |

**P0 recs addressed:**
- Rec 1 (decide perf gate vs functional): ✅ — chose functional, stripped all timing.
- Rec 2 (helper_ prefix for oracle tests): ✅ — tests 3+5 merged with prefix, test 7 renamed with prefix.
- Rec 3 (inline-code exclusion test): ✅ — NEW test added per exact spec.

**P3 rec deferred:** Rec 4 (migrate tests 4/6 to dedicated file) explicitly skipped — acceptable, structural move outside eval scope.

**Test count:** 8 original → 8 after (-1 MERGE, +1 NEW). All 8 pass (`cargo test -- sync_perf_baseline`, 0.89s).

**Unrelated changes:** Commit eef9d64 also touches `similar_concepts_plateau_tests.rs` (164 insertions). That file is not part of this eval spec. Bundling unrelated changes in the same commit is a minor process concern but does not affect the correctness of this eval's remediations.

- Verdict: **PASS**

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- sync_perf_baseline`: 8 passed, 0 failed (1.20s)
- All 8 test functions confirmed green:
  - `sync_single_file_indexes_all_files_in_corpus`
  - `sync_all_completes_over_ten_file_corpus`
  - `helper_count_visible_wikilinks_single_and_multi_section`
  - `extract_wikilinks_with_sections_deduplicates_per_section_unlike_occurrence_count`
  - `extract_wikilinks_with_sections_assigns_section_indices_correctly`
  - `helper_count_all_occurrences_matches_per_concept_counts`
  - `helper_count_visible_wikilinks_skips_inline_code_spans`
  - `count_all_concept_occurrences_returns_zero_for_absent_concepts`
- Traceability: T-0.1 · NFR-3 — all test functions carry `// T-0.1: RTM NFR-3` comments
- Verdict: **PASS**
