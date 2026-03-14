# Test Evaluation: decay_function_boundaries
**File:** `crates/maenifold-core/tests/decay_function_boundaries.rs`
**Test count:** 8
**Evaluated:** 2026-06-05
**Traceability:** T-12.6 · RTM NFR-1

## Summary

Six of eight tests are genuinely load-bearing; the other two have assertions so loose they would pass through clearly defective implementations unchanged. The power law is represented by exactly one test, producing an asymmetry that leaves an entire function family under-covered compared to the exponential branch.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `exponential_decay_never_exceeds_one` | MERGE with `exponential_decay_never_goes_below_zero` | Both tests assert that the result is in `[0.0, 1.0]`. Running them separately yields no additional coverage. Merge into `exponential_decay_stays_in_unit_interval` that asserts `(0.0..=1.0).contains(&weight)` for every sampled age. |
| `exponential_decay_never_goes_below_zero` | MERGE with `exponential_decay_never_exceeds_one` | See above. The age samples (100–10 000) are distinct from the first test's set, but the assertion is logically identical and should live in the same loop. |
| `exponential_decay_negative_age_treated_as_zero` | KEEP | Tests a specific input class (negative age) with a specific contract (exactly 1.0). The exact-equality assertion is sound here only if the implementation guards via an early-return `if age_days < 0 { return 1.0 }`. That contract is worth pinning. Consider expanding the loop to cover several negative values (`-1, -5, -100`) to rule out a special-case for only `-5`. |
| `exponential_decay_zero_half_life_handled_gracefully` | REWRITE | The assertion `(0.0..=1.0).contains(&weight)` is so weak it would pass if the implementation returned `0.999_999` due to a silent integer-overflow path. The test name implies "graceful handling" but does not define **what** graceful means numerically. With age=20, grace=14, half_life=0 the only mathematically defensible result is 0.0 (instant-decay interpretation) or a deterministic fallback constant. Assert the concrete expected value; if none is specified in the design, that is itself a spec gap that needs resolving. |
| `exponential_decay_negative_grace_period_treated_as_zero` | REWRITE | `assert!(weight < 1.0)` is almost content-free as an oracle. For age=5 days and half_life=30, treating grace as 0 yields `2^(-5/30) ≈ 0.891`. An implementation that returns `0.0` or `0.999` would both satisfy `< 1.0`. The test name promises "treated_as_zero" but the assertion does not verify that equivalence; it only rules out the trivial case of the grace-period swallowing the age entirely. Add `assert_approx_eq(weight, expected, 0.01)` with the analytically derived expected value. |
| `exponential_decay_datetime_api_calculates_correct_age` | REWRITE (name + stronger assertion) | The 10-day-old file sits within the 14-day grace period, so the result is trivially 1.0 regardless of whether the age calculation is correct — any implementation that respects grace period would pass. The name "calculates_correct_age" is a lie: the test cannot distinguish a correct age calculation from an incorrectly calculated age that still lands inside the grace window. Rename to `exponential_decay_datetime_within_grace_returns_one` and add a second case where the date is *outside* the grace window and assert a non-trivial approximate value. |
| `exponential_decay_datetime_api_future_date_returns_one` | KEEP | Tests an important edge case (future timestamp → weight = 1.0) that a signed-integer age path could easily mishandle. Assertion is exact and correct. |
| `power_law_decay_datetime_api_calculates_correct_age` | KEEP (rename) | The math is sound: age=4, A=1.0, B=0.5 → `1.0 × 4^(-0.5) = 0.5`. Assertion uses `assert_approx_eq` correctly. However, this is the **only** power-law test in the file, leaving the `[0, 1]` bound and negative-age / negative-grace edge cases entirely untested for the power-law branch. Rename to `power_law_decay_datetime_age_four_no_grace_yields_half` to encode the scenario rather than the meta-claim "calculates_correct_age". The missing boundary tests should be added. |

## Recommendations

1. **Merge tests 1 & 2** into `exponential_decay_stays_in_unit_interval`, iterating over the combined age list and asserting `(0.0..=1.0).contains(&weight)`.
2. **Rewrite test 4** (`zero_half_life_handled_gracefully`): define and assert the exact expected value for the zero-half-life edge case. Leaving it as a range check is a spec gap masquerading as a test.
3. **Rewrite test 5** (`negative_grace_period_treated_as_zero`): replace `weight < 1.0` with `assert_approx_eq(weight, 2f64.powf(-5.0 / 30.0), 0.01)` — or whatever the design specifies.
4. **Rename test 6** and add an out-of-grace-period case that verifies actual age propagation through the datetime API.
5. **Add power-law boundary coverage**: `power_law_decay_never_exceeds_one`, `power_law_decay_never_goes_below_zero`, `power_law_decay_negative_age_treated_as_zero`, `power_law_decay_zero_exponent_handled_gracefully` — the exponential branch has five dedicated boundary tests; the power-law branch has one.
6. Test 3 (`negative_age_treated_as_zero`) asserts exact float equality (`assert_eq!(weight, 1.0)`). This is currently safe only because the implementation explicitly returns `1.0` before entering any arithmetic. Document that assumption in the test or use `assert_approx_eq` with a tight tolerance to make it resilient to future refactoring.

## Load-Bearing Score: 6/8

Tests 4 and 5 are present in the binary but their oracles are too weak to catch the bugs their names advertise. They inflate test count without providing proportional defect-detection value.
