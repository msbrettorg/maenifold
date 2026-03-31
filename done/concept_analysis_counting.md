---
id: concept_analysis_counting
status: done
---

# Test Evaluation: concept_analysis_counting
**File:** `crates/maenifold-core/tests/concept_analysis_counting.rs`
**Test count:** 7
**Evaluated:** 2026-07-12
**Traceability:** T-1.4 (FR-2, RTM-completed) — note: `analysis.rs` cites T-1.2; support.rs cites T-1.4. All tests cite T-1.4.

---

## Summary

Seven tests covering the counting and aggregation dimension of `AnalyzeConceptCorruption`. The file is well-scoped: it owns the counting axis (zero-state, extraction, repetition, multi-variant, cross-file, truncation accounting, directory exclusion) and defers classification and repair suggestions to sibling files. Every test is load-bearing. No test should be removed.

Two mechanical concerns are worth noting. First, tests 2–4 form a natural parameterization progression (1-file × N-variant × M-occurrence), and could be consolidated into a table-driven test without losing clarity — but the gain is small and the current naming is precise enough that the benefit does not justify the disruption. Second, test 6 relies on the `report_body_before_repairs` helper splitting output on the literal string `"SUGGESTED REPAIRS"`. If that header is ever renamed during a parity adjustment, the helper silently returns the full output and `bullet_count` may exceed 2 even with `maxResults=2` in place, producing a false failure. The helper is defined in `concept_analysis_support.rs` and is shared, so fixing it would fix multiple test files at once.

One traceability anomaly: the implementation in `repair/analysis.rs` cites `// T-1.2: RTM FR-2` for `analyze_concept_corruption`, while the test file and support module both cite `// T-1.4: RTM FR-2`. Both T-1.2 and T-1.4 are completed work (CHANGELOG.md). The discrepancy is cosmetic but breaks mechanical traceability tooling if any is added later.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `analyze_concept_corruption_empty_memory_directory_reports_zero_unique_variants` | KEEP | Load-bearing zero-state baseline. Distinct from the core.rs "unknown family" zero test — that has a populated fixture, this has no files at all. Both assertions (exact header text + zero count) are precise. |
| `analyze_concept_corruption_extracts_wikilinks_from_markdown_content` | KEEP | Minimal smoke test: one file, one variant, one occurrence. Establishes the precondition for all subsequent counting tests. Overlap with `concept_analysis_core.rs::analyze_concept_corruption_scans_memory_files_and_reports_variant_counts` is partial, not full — the core test uses a complex multi-variant fixture, not an isolated single-concept file. |
| `analyze_concept_corruption_counts_multiple_wikilinks_in_the_same_file` | KEEP | Tests the aggregation dimension: same concept repeated 3× in one file produces `1 unique variants` and `workflow (3x)`. This specific combination (same-concept repetition → aggregate count, not new variant) is not exercised anywhere else. Both assertions are necessary. |
| `analyze_concept_corruption_counts_multiple_matching_variants_in_one_file` | KEEP | Tests the orthogonal dimension: two *different* variants in one file each register as distinct unique variants. Meaningfully different from test 3. Assertions check both count lines individually. |
| `analyze_concept_corruption_aggregates_counts_across_multiple_files` | KEEP | Tests the core cross-file aggregation: `workflow` appears in two files (expected 2x) and `workflows` appears in one (expected 1x). This is the highest-value test in the file — it proves the entire purpose of the tool. Precise assertions on both aggregate values. |
| `analyze_concept_corruption_reports_total_unique_variants_before_max_results_truncation` | KEEP (note fragility) | Uniquely valuable — no sibling test verifies that the `"Found N unique variants"` header reports the *pre-truncation* total. The `concept_analysis_core.rs` max-results test only checks visible bullet count, not the header. The `report_body_before_repairs(&output)` call before counting bullets is correct but brittle: if `"SUGGESTED REPAIRS"` is renamed, the split fails silently and bullet count includes repair-section entries, falsely exceeding 2. This is a helper-level concern, not a test-level flaw. |
| `analyze_concept_corruption_excludes_files_in_git_directories` | KEEP (note weak negative) | Load-bearing correctness test for `.git/` exclusion via `is_hidden_path`. The assertion (`Found 0 unique variants`) correctly catches a regression — without exclusion, the `.git/git-tracked.md` fixture would contribute 1 count. Weakness: an entirely non-traversing implementation (e.g., early return on empty walk) would also satisfy the assertion. Mitigated by the fact that `legitimate.md` exists with no matching wikilinks, proving the traversal ran and found nothing — but that file does not prove the `.git` entry was specifically encountered and skipped. The test is adequate in practice. |

---

## Recommendations

1. **STRENGTHEN `analyze_concept_corruption_excludes_files_in_git_directories`**: After calling `analyze_concept_corruption`, read the `.git/git-tracked.md` fixture back from disk and assert its content is unchanged (as `repair_concepts_dry_run_returns_summary_without_modifying_files` does in the repair suite). More importantly, consider adding a second fixture in a non-`.git` path that *does* contain `[[workflow]]`, so that a successful pass proves: (a) the non-hidden file WAS counted, and (b) the `.git` file was NOT counted. Currently the only proof is absence of count, which conflates "file not found" with "file found but excluded".

2. **FIX `report_body_before_repairs` in `concept_analysis_support.rs`**: The helper splits on `"SUGGESTED REPAIRS"` — a UI string that is susceptible to rename during C# parity work. The split should target a more stable delimiter, or the helper should count bullets by scanning for the first `=== SUGGESTED` prefix pattern rather than an exact string match. This affects test 6 in this file and any other test that calls `bullet_count(report_body_before_repairs(...))`.

3. **ALIGN traceability annotations**: `repair/analysis.rs` cites `// T-1.2: RTM FR-2` while all tests cite `// T-1.4: RTM FR-2`. These are both completed tasks, so no behavioral issue exists today. However, if mechanical traceability tooling is added (e.g., a CI check that maps test comments to RTM entries), the mismatch would surface as noise. Pick one reference — T-1.4 is the more widely-cited one across the test layer — and propagate it into the implementation file.

4. **Consider parameterization for tests 2–4**: These three tests share the same structure (1 file, N variants, M occurrences → expected unique count, expected display string). A `#[rstest]` or inline table-driven test would reduce boilerplate and make the progression explicit. Not urgent — the current naming is precise and the tests are short.

---

## Load-Bearing Score: 7/7

All seven tests reliably catch distinct regressions. There is no dead weight. The two weaknesses identified (test 6's helper fragility, test 7's weak negative proof) are implementation-quality concerns, not correctness failures — neither test would *miss* a real regression in the current codebase.
