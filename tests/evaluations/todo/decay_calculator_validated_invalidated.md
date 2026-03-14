# Test Evaluation: decay_calculator_validated_invalidated
**File:** `crates/maenifold-core/tests/decay_calculator_validated_invalidated.rs`
**Test count:** 8
**Evaluated:** 2026-06-05
**Traceability:** T-12.7 · T-7.8 · RTM NFR-1 · RTM FR-7.8

## Summary

All eight tests are logically correct and load-bearing; none would willingly pass a broken implementation. However, four tests are direct merge targets (tests 1+2 and tests 3+4), and the case-insensitivity approach used for `validated` (comparing against a fixed `1.0`) is epistemically weaker than the approach used for `invalidated` (comparing two computed outputs with a 1e-9 tolerance). Fixing this inconsistency would improve the coverage signal.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `validated_status_returns_one_when_new` | MERGE with `validated_status_returns_one_when_very_old` | Both assert `weight == 1.0` for `validated` status at different ages. The intent is to prove "validated never decays." Two separate tests for a single invariant — merge into `validated_status_never_decays` with ages `[1, 90, 365, 730]` in a loop. |
| `validated_status_returns_one_when_very_old` | MERGE with above | See above. Redundant test structure. |
| `validated_status_is_case_insensitive_uppercase` | MERGE with `validated_status_is_case_insensitive_mixed_case` | Both confirm that case variants of `"validated"` return `1.0`. They collapse into a single `validated_status_is_case_insensitive` test iterating over `["VALIDATED", "Validated", "vAlIdAtEd"]`. |
| `validated_status_is_case_insensitive_mixed_case` | MERGE with above | See above. |
| `invalidated_status_within_grace_period_returns_one` | KEEP | Tests the grace-period boundary for `invalidated` — distinct from the validated tests because `invalidated` *does* decay outside the grace window, making the grace-period assertion non-trivial. |
| `invalidated_status_past_grace_period_decays_aggressively` | KEEP | The most mathematically load-bearing test in the file. Asserts `weight ≈ 0.5` using `assert_approx_eq`. **One caution:** the inline comment `// 7 + 14 = one half-life of decay` is ambiguous — it does not state the effective half-life for `invalidated` status or explain why 14 days past grace equals one half-life when the global `half_life=30`. If the `exponential_defaults` half-life ever changes, this test will silently flip meaning without the comment guiding the reader. Add an explicit derivation in the comment. |
| `invalidated_status_is_case_insensitive` | KEEP | Superior oracle design — compares two computed outputs (lower vs upper) with a 1e-9 tolerance instead of comparing against a fixed expected value. This correctly isolates case-sensitivity from all other behavior. Note: tests 3 & 4 for `validated` use the weaker fixed-expected-value approach. Align them to this pattern once merged. |
| `invalidated_status_decays_faster_than_active` | KEEP | Load-bearing semantic check. However, the term `"active"` as a status value is never explained in the test or its module-level comment. If `"active"` is simply treated as the null/default status, rename the helper argument to `None` (or add an explanatory comment) so readers do not need to trace through `compute_decay_weight` to understand the intent. |

## Recommendations

1. **Merge tests 1 & 2** into `validated_status_never_decays` with a representative spread of ages (`[1, 90, 365, 730]`). Eight test lines replace sixteen with no loss of coverage.
2. **Merge tests 3 & 4** into `validated_status_is_case_insensitive` that iterates over `["VALIDATED", "Validated", "vAlIdAtEd"]`. Adopt the comparison-of-two-outputs pattern from test 7 — compare each variant against the lowercase baseline with a tight tolerance rather than against a hardcoded `1.0`; this isolates case-sensitivity behavior from the "validated-always-returns-one" behavior.
3. **Annotate test 6** (`invalidated_status_past_grace_period_decays_aggressively`): derive the expected `0.5` inline — state the effective half-life for `invalidated` status explicitly so the assertion does not silently break if global defaults shift.
4. **Clarify `"active"` in test 8**: document whether `"active"` is a recognized status token or a stand-in for "no special status." If the latter, use `None` as the status argument and rename the test to `invalidated_status_decays_faster_than_no_status`.
5. **Consider adding**: a test for `invalidated` at exactly the grace boundary (age == grace_days) to pin the inclusive/exclusive edge — the current tests only cover age < grace and age >> grace.

## Load-Bearing Score: 8/8

Every test would catch a real regression. The merging opportunities are a readability and maintainability concern, not a correctness concern.
