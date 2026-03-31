---
id: decay_function_exponential
status: done
pipeline: tdd
---

# Test Evaluation: decay_function_exponential
**File:** `crates/maenifold-core/tests/decay_function_exponential.rs`
**Test count:** 8
**Evaluated:** 2026-07-10
**Traceability:** T-12.6 (RTM NFR-1)

---

## Summary

Three of the eight tests redundantly verify the same single branch of implementation code (`age_days <= grace_period_days → return 1.0`). Tests 4–6 verify N × half-life periodicity with N=1,2,3 but are structurally identical and parameterizable into one table test. Of the eight, only tests 3, 4, 7, and 8 cover genuinely distinct logic paths. Additionally, tests 1–3 are misclassified: the grace-period behavior they test is **not exponential-specific** — it is shared by all decay functions — which means this file's tests partially belong in `decay_function_grace_periods.rs` or `decay_function_boundaries.rs` instead.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `exponential_decay_within_grace_period_returns_weight_one` | **MERGE** with `exponential_decay_exactly_at_grace_period_boundary_returns_weight_one` | Same code path as test 3: `age_days (5) <= grace_period_days (14)` → `return 1.0`. The boundary test (test 3) is the more valuable of the two; this interior-point test adds zero additional coverage. Neither the integer value `5` nor any structural difference in the implementation distinguishes `age=5` from `age=0`. |
| `exponential_decay_zero_age_returns_weight_one` | **MERGE** with `exponential_decay_exactly_at_grace_period_boundary_returns_weight_one` | Same code path as tests 1 and 3. `age=0` is not a special-cased branch in the implementation — it falls through the same `age_days <= grace_period_days` guard. Skeptical challenge: *could `age=0` cause a division-by-zero or underflow?* No — the grace check fires first. This test's insurance value is zero beyond what test 3 already provides. |
| `exponential_decay_exactly_at_grace_period_boundary_returns_weight_one` | **KEEP** | The only test among tests 1–3 that provides distinct value: the off-by-one risk (`<` vs `<=`) on the grace period boundary. If the guard were changed to `age_days < grace_period_days`, this test catches the regression while tests 1–2 remain green. True fence-post guard. |
| `exponential_decay_one_half_life_past_grace_returns_weight_half` | **KEEP** | Load-bearing mathematical property: `2^(-1) = 0.5` at exactly one half-life. Validates the formula's exponent sign and the grace-period offset. Removing this would allow a sign flip (`2^(+1)` vs `2^(-1)`) to ship undetected. |
| `exponential_decay_two_half_lives_past_grace_returns_weight_quarter` | **MERGE** with tests 4 and 6 | Structurally identical to tests 4 and 6 — same formula, different N. Value as a standalone test: marginal. As part of a parameterized suite `[(1, 0.5), (2, 0.25), (3, 0.125)]`, all three are worth keeping. As isolated functions they are three copies of the same assertion. |
| `exponential_decay_three_half_lives_past_grace_returns_weight_eighth` | **MERGE** with tests 4 and 5 | Same reasoning as test 5. N=3 adds statistical confidence the formula is correct (not coincidentally passing with N=1), but this is better expressed as a table test than three separate functions. |
| `exponential_decay_half_of_half_life_past_grace_returns_intermediate_weight` | **KEEP** | Unique and load-bearing: tests a **non-integer** exponent (`half_life / 2 → 2^(-0.5)`), distinguishing correct exponential decay from a piecewise linear approximation. Expected value is computed analytically (`0.5f64.powf(0.5)`), which is correct and self-documenting. No other test in this file or in `decay_function_boundaries.rs` covers this case. |
| `exponential_decay_very_old_content_approaches_zero_but_never_reaches` | **KEEP** | Tests the asymptotic lower bound (`weight > 0.0` AND `weight < 0.001`) at 10 years (3650 days). While `decay_function_boundaries.rs::exponential_decay_never_goes_below_zero` also checks `age=3650`, that test only asserts `weight >= 0.0`. This test additionally asserts `weight < 0.001` — verifying the *rate* of decay is reasonable, not just that it doesn't underflow. The assertion is tighter and complementary, not redundant. |

---

## Recommendations

1. **Merge tests 1 and 2 into test 3** (high value, low effort): Remove `exponential_decay_within_grace_period_returns_weight_one` and `exponential_decay_zero_age_returns_weight_one`. The boundary test (`exactly_at_grace_period_boundary`) is the only one of the three that can catch a real regression. If interior coverage is desired for documentation purposes, add `age=5` as a comment in the boundary test, not a separate function.

2. **Parameterize tests 4, 5, and 6** into one test function (medium priority):
   ```rust
   #[test]
   fn exponential_decay_n_half_lives_past_grace_returns_correct_weight() {
       let _guard = acquire_test_lock();
       let env = TestConfigGuard::new();
       for (n, expected) in [(1, 0.5), (2, 0.25), (3, 0.125)] {
           let age_days = DEFAULT_GRACE_PERIOD_DAYS + (n * DEFAULT_HALF_LIFE_DAYS);
           let weight = compute_exponential_decay(&env.cfg, "memory://projects/n-half-lives", age_days, DEFAULT_GRACE_PERIOD_DAYS);
           assert_approx_eq(weight, expected, 0.0001);
       }
   }
   ```
   This makes the periodicity pattern explicit and reduces three near-identical functions to one.

3. **Relocate or duplicate grace-period tests to `decay_function_grace_periods.rs`**: Tests 1–3 test the grace period behavior shared by all decay functions (exponential, power-law). Keeping them only in `decay_function_exponential.rs` creates a false impression that grace-period logic is exponential-specific. The file `decay_function_grace_periods.rs` already exists — these tests belong there, and `decay_function_exponential.rs` should contain only tests that require exponential-specific arithmetic.

4. **Review test attribution** (informational): All 8 tests carry `// T-12.6: RTM NFR-1` attribution but several (1, 2, 3) test grace-period logic introduced in a different task. Verify whether T-12.6 is the correct traceability reference for all eight tests, or whether T-12.5 (or the grace-period task) should be cited for tests 1–3.

---

## Load-Bearing Score: 6/8

> Tests 1 and 2 (`within_grace_period` and `zero_age`) are not load-bearing. Any regression they could catch is already caught by test 3 (`exactly_at_grace_period_boundary`). Removing them would not allow any new bug to ship that test 3 does not already block.

---
## SWE Remediation
- REMOVE `exponential_decay_within_grace_period_returns_weight_one`: subsumed by boundary test (P0)
- REMOVE `exponential_decay_zero_age_returns_weight_one`: subsumed by boundary test (P0)
- KEEP `exponential_decay_exactly_at_grace_period_boundary_returns_weight_one`: added comment noting subsumption
- MERGE tests 4,5,6 → `exponential_decay_n_half_lives_past_grace_returns_correct_weight`: parameterized table test with [(1, 0.5), (2, 0.25), (3, 0.125)] (P1)
- KEEP `exponential_decay_half_of_half_life_past_grace_returns_intermediate_weight`: untouched
- KEEP `exponential_decay_very_old_content_approaches_zero_but_never_reaches`: untouched
- P2 (relocate grace-period tests) and P3 (attribution review): skipped per SWE scope rules
- 8 tests → 4 tests, zero coverage loss
- Commit: b59f267
