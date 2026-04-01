---
id: T-80.32
title: "Test Evaluation: concept_analysis_core"
status: done
pipeline: tdd
---

# Test Evaluation: concept_analysis_core
**File:** `crates/maenifold-core/tests/concept_analysis_core.rs`
**Test count:** 6
**Evaluated:** 2026-07-14
**Traceability:** T-1.4 (FR-2, RTM-008 through RTM-013, completed) — implementation cites `// T-1.2: RTM FR-2`; the T-1.2 vs T-1.4 discrepancy with `repair/analysis.rs` is inherited from this codebase's sprint history and noted in the companion `concept_analysis_classification.md` evaluation. All RTM entries are completed work.

---

## Summary

Six tests covering the primary end-to-end scenarios for `AnalyzeConceptCorruption`: basic operation (variant counts + report header), classification bucket population, `maxResults` truncation, unknown-family empty report, `learn=true` help-mode dispatch, and repair-command generation. Every test is load-bearing and every scenario is unique — no test should be removed outright. However, two tests contain assertion-quality defects serious enough to warrant a rewrite, and two more carry naming inaccuracies or misleading structural choices that should be corrected.

**Three observations that Gray demands answers to before accepting this suite as reliable:**

**1. Test 3 uses an asymmetric upper-bound assertion (`<= 2`).** With a fixture containing 4 variants and `maxResults: 2`, the expected behavior is exactly 2 results. The assertion `visible_variants <= 2` passes silently if the implementation returns 0 or 1 variant — hiding under-truncation bugs entirely. Confirmed by fixture trace: after sort-by-count-then-alpha, `alpha-tool` and `beta-tool` are the expected survivors. The assertion should be `== 2`.

**2. Test 6's third assertion is noise.** `output.contains("tools") && output.contains("tool")` provides no signal beyond "output is not empty." Both strings appear throughout the output — in variant listings, section headers, and the fixture body — completely independent of whether a valid repair command was generated. The real question is: does "tools" appear as a `conceptsToReplace` argument in the SUGGESTED REPAIRS section? The current assertion cannot answer that.

**3. Test 1's name claims multi-file scanning; the fixture writes one file.** `scans_memory_files_and_reports_variant_counts` implies the scanner traverses a corpus of files. The test writes exactly one fixture. The multi-file behavior is not exercised here, and the gap is invisible to future readers who see the test and assume it covers that scenario.

**What is not tested at all:** all three error paths in `analyze_concept_corruption` (`conceptFamily` missing → `MissingProperty`, empty `conceptFamily` → `InvalidArgument`, `maxResults < 0` → `InvalidArgument`) have zero coverage in this file or any other discoverable test file. Multi-file scanning behavior is also absent.

**Fixture trace (for ground truth):** `CONCEPT_ANALYSIS_FIXTURE` contains `[[tool]](2x), [[tools]](2x), [[coding-tool]](1x), [[tool system]](1x), [[GraphTools.cs]](1x)` — 5 unique variants, not 7. Classification: `tool` → With Suffix (because `classify_corruption_variant` checks `concept_lower.contains("tool")`, so the canonical name itself lands in With Suffix alongside `tool system`); `tools` → Singular/Plural; `coding-tool` → Compound (with -); `GraphTools.cs` → File Paths. Repair: `plural_canonical` for `tool` is `tool` itself (doesn't end in 's'), so the plural repair command reads `canonicalConcept='tool'`.

---

## Verdicts

| # | Test Function | Verdict | Reason |
|---|---|---|---|
| 1 | `analyze_concept_corruption_scans_memory_files_and_reports_variant_counts` | **KEEP** | Load-bearing smoke test: verifies the report header appears, and exact counts for two variants (tool 2x, tools 2x) are rendered. Count-format coupling is an accepted trade-off at this layer. Name is slightly misleading ("scans memory files" plural but only one file is written); not a defect, just imprecise. The assertions are correctly scoped to counting behavior and do not overlap with test 2. |
| 2 | `analyze_concept_corruption_classifies_variants_into_expected_categories` | **KEEP** | Load-bearing: each of the four assertions checks a distinct section header AND a concept name unique to that bucket. Because the implementation skips empty sections, a misclassification would cause the section header assertion to fail correctly (e.g., if `coding-tool` landed in Other, "Compound (with -):" would not appear). The AND structure is sound here — the section header does the real work; the concept name is a secondary signal. Identical setup to test 1 is acceptable given distinct assertion scope. |
| 3 | `analyze_concept_corruption_respects_max_results_limit` | **REWRITE** | Load-bearing but asymmetrically specified. The assertion `visible_variants <= 2` passes when the implementation returns 0 or 1 variants, masking under-truncation regressions. With 4 variants in the fixture and `maxResults: 2`, the correct assertion is `== 2`. Additionally, the test does not assert *which* 2 variants are visible (deterministic ordering by count desc then alpha means `alpha-tool` and `beta-tool` should be the survivors — this is verifiable and would catch sort-order regressions). The `report_body_before_repairs` split is a good technique and should be retained. |
| 4 | `analyze_concept_corruption_returns_empty_report_for_unknown_family` | **KEEP** | Load-bearing: establishes the zero-match boundary. Single assertion on `"Found 0 unique variants"` is correct and sufficient for this case. The name is slightly imprecise ("empty report" is inaccurate — the report emits a header and an always-present SUGGESTED REPAIRS section per PARITY-REPA02; only the variant list is empty). Not worth a rewrite for the name alone, but a comment would clarify intent. |
| 5 | `analyze_concept_corruption_learn_true_returns_help_doc_instead_of_analysis` | **KEEP** | Well-designed. Two complementary assertions: positive (tool name mentioned in help) and negative (output does not start with the analysis header). The `trim_start()` on the negative check is thoughtful. The fact that no fixture is written implicitly tests the short-circuit before file I/O — this is correct behavior, intentional or not. The learn-mode path is exercised nowhere else. |
| 6 | `analyze_concept_corruption_suggests_repair_commands_for_safe_merges` | **REWRITE** | Load-bearing, but the third assertion is ineffective. `output.contains("tools") && output.contains("tool")` trivially passes because both strings appear in variant listings, section headers, and canonical-name positions throughout the output — entirely independent of whether a repair command was generated. Assertions 1 (`"SUGGESTED REPAIRS"`) and 2 (`"RepairConcepts"`) are sound. Assertion 3 should be replaced with a section-scoped check that `"conceptsToReplace='tools'"` appears in the repair command, confirming the plural-to-singular repair targeting is correct. |

---

## Recommendations

1. **FIX test 3's asymmetric assertion.** Replace `visible_variants <= 2` with `visible_variants == 2`. While there, add assertions for *which* variants survive: `assert!(report_body.contains("alpha-tool"), ...)` and `assert!(report_body.contains("beta-tool"), ...)`, and assert the dropped ones are absent: `assert!(!report_body.contains("gamma-tool"), ...)`. This turns the test from "didn't return too many" into "returned the right ones in the right order."

2. **FIX test 6's third assertion.** Replace `output.contains("tools") && output.contains("tool")` with an assertion that is scoped to the repair command content. The simplest approach: split on `"SUGGESTED REPAIRS"` (as `report_body_before_repairs` already does for the opposite side) to isolate the repairs section, then assert `repairs_section.contains("conceptsToReplace=") && repairs_section.contains("tools")`. Alternatively, assert `output.contains("conceptsToReplace='tools'")` directly, which anchors the variant name to its structural position in the repair command.

3. **ADD error-path coverage.** Three `InvalidArgument`/`MissingProperty` branches in `analyze_concept_corruption` are completely uncovered:
   - `conceptFamily: ""` (empty string) → `InvalidArgument`
   - `maxResults: -1` → `InvalidArgument`
   - payload with no `conceptFamily` key → `MissingProperty`
   These are RTM-specified contract invariants. At minimum, add `#[should_panic]` or error-return assertions for each. A dedicated `concept_analysis_error_paths.rs` test file is the cleanest location given the existing per-concern file structure.

4. **ADD multi-file scanning test.** Test 1's name implies multi-file corpus scanning but writes one file. Add a test that writes variants across *two* memory files and asserts the counts aggregate correctly. This exercises the `get_memory_files` iteration loop and would catch any early-exit bug in the scanner. Suggested name: `analyze_concept_corruption_aggregates_variants_across_multiple_files`.

5. **FIX test 1's name.** `scans_memory_files_and_reports_variant_counts` promises multi-file behavior that isn't exercised. Rename to `analyze_concept_corruption_reports_variant_counts_from_single_file` or promote it to a genuine multi-file test (see recommendation 4). If the new multi-file test is added, rename test 1 to be explicit about its single-file scope.

6. **CLARIFY test 4's name.** `returns_empty_report_for_unknown_family` is inaccurate: the report is not empty (header + SUGGESTED REPAIRS always present). Rename to `returns_zero_variants_for_unknown_family`. Low urgency, but a reader encountering this test for the first time may wonder why the assertion doesn't check for an empty string.

7. **ALIGN traceability annotations** (inherited issue): `repair/analysis.rs` cites `// T-1.2: RTM FR-2` on `analyze_concept_corruption`; all tests in this file cite `// T-1.4: RTM FR-2`. Both reference completed work. Standardize the implementation comment to `// T-1.4: RTM FR-2` to match the test files, or add an inline note explaining both tasks contributed to this function.

8. **CONSIDER a combined setup helper.** All 6 tests (plus the paired classification tests) repeat the same 4-line preamble: `TEST_MUTEX.lock()`, `ConfigOverrideReset`, `isolated_config()`, `write_memory_fixture()`. The `concept_analysis_support.rs` module already centralizes individual helpers. A `setup_with_fixture(relative_path, content) -> (MutexGuard, TempDir, Config)` helper would reduce per-test boilerplate to one line and ensure future setup-order changes are made in one place. Not a correctness issue, but reduces maintenance surface as the suite grows.

---

## Load-Bearing Score: 6/6

All six tests exercise distinct, regression-catching behaviors. None should be removed. The suite earns full marks on coverage breadth across the happy-path scenarios mandated by RTM-008 through RTM-013.

However, the *reliability* score is lower: tests 3 and 6 carry assertion defects that produce false passes for specific bug classes (under-truncation in test 3; repair-command regression in test 6). The REWRITE verdicts on those two tests are the highest-priority actions before relying on this suite as a quality gate.

The complete absence of error-path coverage (3 untested branches) and the missing multi-file corpus test are structural gaps that should be addressed before this tool's contract is considered fully specified by tests.

---

## SWE Remediation

- REWRITE test 3 (`respects_max_results_limit`): replaced asymmetric `<= 2` with exact `== 2` assertion; added which-variant survival checks (alpha-tool, beta-tool survive; delta-tool, gamma-tool truncated per count-desc-then-alpha sort). Strengthened per P0 rec 1.
- REWRITE test 6 (`suggests_repair_commands_for_safe_merges`): replaced ineffective third assertion (`output.contains("tools") && output.contains("tool")`) with section-scoped `output.contains("conceptsToReplace='tools'")` per P0 rec 2.
- ADD 3 error-path tests: `rejects_missing_concept_family` (MissingProperty), `rejects_empty_concept_family` (InvalidArgument), `rejects_negative_max_results` (InvalidArgument). Covers all 3 untested branches per P1 rec 3.
- RENAME test 1: `reports_variant_counts_from_single_file` (was misleading `scans_memory_files_and_reports_variant_counts`) per P1 rec 5.
- RENAME test 4: `returns_zero_variants_for_unknown_family` (was inaccurate `returns_empty_report_for_unknown_family`) per P2 rec 6.
- ADD `try_analyze_concept_corruption` helper to support module for fallible error-path testing.
- SKIP rec 4 (multi-file test): already covered by `concept_analysis_counting::aggregates_counts_across_multiple_files`.
- SKIP rec 7 (align traceability): impl-side annotation, outside test file scope.
- SKIP rec 8 (setup helper): P3, not a correctness issue.
- Test count: 6 → 9 (6 original + 3 error-path). All 9 pass.
- Commit: 28a32ce
