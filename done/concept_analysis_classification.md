---
id: T-80.31
title: "Test Evaluation: concept_analysis_classification"
status: done
pipeline: tdd
---

# Test Evaluation: concept_analysis_classification
**File:** `crates/maenifold-core/tests/concept_analysis_classification.rs`
**Test count:** 6
**Evaluated:** 2026-07-12
**Traceability:** T-1.4 (FR-2, RTM-completed) — consistent with `concept_analysis_counting.md` and `concept_analysis_core.rs`; same T-1.2 vs T-1.4 discrepancy vs `repair/analysis.rs` noted in the counting evaluation applies here too.

---

## Summary

Six tests covering the *classification* axis of `AnalyzeConceptCorruption`: which bucket does a matched concept land in (File Paths, Singular/Plural, Compound, Other), and does classification priority hold when a concept matches more than one rule. The scope is well-chosen and complementary to `concept_analysis_counting.rs` (which owns the counting/aggregation axis) and `concept_analysis_core.rs` (which owns the end-to-end multi-variant smoke case).

Every test is load-bearing — each would catch a distinct regression. However, the suite has three compounding quality problems that erode confidence:

**1. Universal reliance on raw `output.contains()` string matching.** All six tests assert exclusively against the raw text output. There is no structured query, no bucket-level API, and no count verification. A cosmetic rename of any section header (e.g. `"File Paths:"` → `"Path-Like Concepts:"`) would turn six green tests red with no behavior change. The inverse is also true: if classification logic broke silently but the header text was preserved, the tests would still pass.

**2. Tests 1 and 2 are structurally identical.** Both test the file-path bucket (different trigger: slash-prefix vs dot-notation). Same setup, same assertions, different fixture strings. This is a textbook parameterization opportunity. If a future third file-path classification rule is added (e.g., `\` backslash prefix, or `::` Rust path notation), the writer must copy-paste a third time rather than add a row to a table.

**3. Asymmetric assertions between tests 4 and 5.** Test 4 (plural repair) asserts `"RepairConcepts"` is mentioned in the suggestion. Test 5 (compound repair) omits that assertion. Either the `"RepairConcepts"` mention matters — in which case test 5 is under-specified — or it does not matter — in which case test 4 carries a spurious assertion. The asymmetry is evidence that the tests were written incrementally without a checklist.

Test 3 has a name that actively misleads: `classifies_unmatched_variants_as_other` — the fixture uses `[[workflow]]`, which is not a "variant" at all, it is the exact canonical form. The bucket labeled "Other" in this context means "matched the family name but triggered no corruption-repair heuristic", which is functionally the opposite of a problematic variant. The misleading name makes a future reader unsure whether this is a baseline test or an edge-case test.

Test 6 contains the most fragile assertion in the file: a negative `output.contains()` check that embeds a hard-coded section header, a literal newline, and a Unicode bullet character (`•`). If any of these change, the negative check silently inverts to a false pass — the test will claim priority is working when it is not. This is the highest-risk test in the file.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `analyze_concept_corruption_classifies_slash_prefixed_concepts_as_file_paths` | KEEP | Load-bearing: tests the slash-prefix rule (`/src/workflow` → File Paths). Distinct from dot-notation rule. Both assertions are necessary. Candidate for parameterization with test 2. |
| `analyze_concept_corruption_classifies_dot_concepts_as_file_paths` | MERGE | Structurally identical to test 1 — same boilerplate, same bucket, different trigger string. Tests a distinct classification rule (`workflow.json` dot-notation) but the test body is an exact copy-paste. Should be merged into a parameterized `classifies_file_path_patterns` test alongside test 1. |
| `analyze_concept_corruption_classifies_unmatched_variants_as_other` | REWRITE | Load-bearing (establishes the baseline "no-repair-needed" Other bucket), but the name is actively misleading. `[[workflow]]` is the canonical form, not a "variant". Rename to `classifies_canonical_form_as_other` or `classifies_exact_family_match_as_other`. The `"workflow (1x)"` assertion couples to count formatting; if the output renders counts differently for 1 occurrence (e.g. `"workflow"` without count, or `"workflow (once)"`), the test breaks despite correct behavior. Extract the count-format concern or use a less brittle assertion. |
| `analyze_concept_corruption_produces_plural_repair_suggestions` | KEEP | Load-bearing: tests that plural forms generate a specific repair pathway (Fix plural forms + RepairConcepts mention + variant name). Three assertions are appropriate. The `"RepairConcepts"` assertion is the most valuable — it confirms the output is actionable. Note: this assertion is absent from the structurally parallel test 5 (see Recommendations). |
| `analyze_concept_corruption_produces_compound_repair_suggestions` | REWRITE | Load-bearing: tests that compound/hyphenated forms generate a repair suggestion. However, it has only 2 assertions while the parallel test 4 has 3. The missing assertion is the `"RepairConcepts"` check — the most important one. Either test 5 is under-specified and needs the assertion added, or test 4 has a spurious assertion and it should be removed from there. The asymmetry is the defect; the fix direction depends on whether `"RepairConcepts"` is contractual output. Given that test 4 verifies it, treat it as contractual and add it to test 5. |
| `analyze_concept_corruption_prefers_path_classification_over_plural_classification` | REWRITE | Load-bearing (priority ordering is a real correctness concern) and the only test that exercises precedence. The positive assertion (`"File Paths:"` present) is sound. The negative assertion is not: `!output.contains("Singular/Plural:\n  • /workflows")` embeds an exact newline, the Unicode bullet `•`, and the full section header in a single string literal. A format-only change to the output renderer silently transforms this into a false pass. Replace with a structural approach: either parse the Singular/Plural section explicitly and assert `/workflows` is absent from it, or (better) add a positive assertion that `/workflows` appears in the File Paths section and assert the total unique-variants count is 1, which structurally proves it was not double-counted. |

---

## Recommendations

1. **PARAMETERIZE tests 1 and 2** into a single `classifies_file_path_pattern_variants` test using `#[rstest]` or inline `for` loop over a fixture table. Each row specifies: fixture content, concept family, expected bucket header, expected concept in output. A future contributor adding a third file-path classification rule (e.g., backslash prefix or `::` Rust path notation) adds a row, not a copy-pasted function. The existing `concept_analysis_support.rs` helpers already make the setup one-liner; only the assertions need to be parameterized.

2. **REWRITE test 3's name and assertion**: Rename to `analyze_concept_corruption_classifies_canonical_form_as_other`. Replace `output.contains("workflow (1x)")` with `output.contains("workflow")` and a separate `output.contains("Other:")` (already present), or use the `bullet_count(output)` helper from `concept_analysis_support.rs` to verify exactly one Other entry without coupling to the count-rendering format. The count-format coupling is the same fragility identified in the `concept_analysis_counting.md` evaluation for the `report_body_before_repairs` helper.

3. **FIX test 5's missing `"RepairConcepts"` assertion**: The most actionable output from `AnalyzeConceptCorruption` is the `RepairConcepts` invocation snippet. Test 4 asserts it; test 5 does not. This is not a style preference — it means the compound-forms repair pathway can regress to producing suggestions with no actionable guidance and test 5 will not catch it. Add `assert!(output.contains("RepairConcepts"), "compound repair suggestion should mention RepairConcepts: {output}")` to test 5.

4. **REWRITE test 6's negative assertion**: The current form `!output.contains("Singular/Plural:\n  • /workflows")` is the most fragile assertion in the entire classification suite. A whitespace normalization change, a bullet-character swap, or a section header rename silently inverts it to a false pass. Replace with: (a) a positive `output.contains("/workflows")` assertion in the `File Paths:` section (scan forward from "File Paths:" until the next section break, and check the substring exists there), or (b) invert the structure — use the `bullet_count` helper on each section independently. At minimum, break the single compound string into separate `output.contains("Singular/Plural:")` (to verify the section exists at all) and a section-scoped absence check.

5. **DEDUPLICATE setup boilerplate**: All six tests repeat the same four-line setup block (`TEST_MUTEX.lock()`, `ConfigOverrideReset`, `isolated_config()`, `write_memory_fixture()`). The `concept_analysis_support.rs` module already provides a `write_memory_fixture` helper but there is no helper that composes the full per-test setup. A `classification_fixture(relative_path, content) -> (TempDir, Config, MutexGuard)` helper (or macro) would make the test bodies 3–4 lines instead of 8, and would ensure any future change to setup order is made in one place. The counting file has the same issue — this is a cross-file concern for the `concept_analysis_support.rs` author.

6. **ALIGN traceability annotations** (inherits from the counting evaluation): `repair/analysis.rs` cites `// T-1.2: RTM FR-2` while all tests cite `// T-1.4: RTM FR-2`. Both are completed work. Standardize on T-1.4 in the implementation file, or add a note explaining both task references are intentional.

---

## Load-Bearing Score: 6/6

All six tests would catch distinct regressions in the current codebase. There is no dead weight. However, the raw reliability score is lower than the count implies: two tests (test 3's count-format coupling, test 6's fragile negative assertion) carry a non-trivial false-pass risk in the face of pure output-formatting changes. The REWRITE verdicts on tests 3, 5, and 6 are aimed at converting currently fragile load-bearing tests into *reliably* load-bearing ones.
