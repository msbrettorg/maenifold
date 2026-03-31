---
id: decay_function_power_law
status: done
pipeline: tdd
---

# Test Evaluation: decay_function_power_law

**File:** `crates/maenifold-core/tests/decay_function_power_law.rs`
**Test count:** 11
**Evaluated:** 2026-07-10
**Traceability:** T-12.6 (RTM NFR-1)

---

## Summary

The suite has one excellent property-based test (`memory_halves_when_time_quadruples`), one test that only tests a test-support helper (`custom_parameters`), two tests with assertions too weak to justify their names (`slower_than_exponential`, `produces_different_curve`), and two tests that are near-duplicates of each other (`below_day_one`/`zero_age`). The production power-law formula is hardcoded to `A=1.0, B=0.5`; any test exercising custom `A` or `B` through `power_law_formula(a, b)` from `support.rs` is testing a test-only function, not production behavior.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `power_law_decay_basic_calculation_follows_formula` | KEEP | Anchors the formula: `weight(4) ≈ 4^(-0.5) = 0.5`. Only test that verifies production output against an independent formula computation. |
| `power_law_decay_at_day_one_returns_weight_one` | KEEP | Verifies the critical boundary: `effective_age = max(1-0, 1) = 1`, so `1^(-0.5) = 1.0`. Distinct from the zero-age path which hits the `age_days <= grace` branch. |
| `power_law_decay_below_day_one_caps_at_one` | MERGE | Functionally identical to `zero_age_returns_weight_one` — both exercise the `age_days <= grace_period_days → 1.0` branch. The `DecayEnvGuard` with `grace=7` and a 12-hour-old timestamp yields `age_days=0`, which is the same effective case as `zero_age`. Merge into `zero_age` with an added sub-day timestamp assertion, or add a comment explaining what branch makes this distinct. |
| `power_law_decay_zero_age_returns_weight_one` | KEEP | Clearly exercises `age_days == 0 → 1.0` via the grace-period gate. Keep; absorb `below_day_one` here. |
| `power_law_decay_never_exceeds_one` | KEEP | Defensive; the `.clamp(0.0, 1.0)` call makes this trivially pass today, but the test would catch an accidental removal of the clamp or an `A > 1.0` drift. Worth keeping as a property invariant. |
| `power_law_decay_never_goes_below_zero` | MERGE | Identical in structure to `never_exceeds_one`; the clamp makes both trivially true. Merge into a single loop that asserts `0.0 <= weight && weight <= 1.0` for the same age inputs. Saves a test slot while strengthening the property. |
| `power_law_decay_slower_than_exponential_for_old_content` | REWRITE | The assertion `power_law >= exponential * 0.5` does **not** verify the stated claim ("slower than exponential"). It only verifies that power-law isn't more than half of exponential — a laughably weak bar. The test name promises a comparative decay-rate check; the assertion delivers a lower-bound so loose it would pass even if power-law were slightly faster than exponential. Assert `power_law >= exponential`, or use a range of ages and assert the ordering holds for each. |
| `power_law_decay_produces_different_curve_than_exponential` | REWRITE | This test verifies that two different algorithm implementations produce different numbers at least once. It would only fail if both formulas accidentally became identical implementations — an implausible regression. It asserts nothing about whether either curve is *correct*. Replace with a table of expected `(age, expected_weight)` pairs at multiple time points to verify curve shape, not mere distinctness. |
| `power_law_decay_memory_halves_when_time_quadruples` | KEEP | This is the best test in the file. It encodes the mathematical invariant of power-law decay (`t → 4t ⟹ weight halved`) and verifies three consecutive doublings. If the exponent changes from `-0.5` to anything else, this test fails immediately. |
| `power_law_decay_negative_age_returns_weight_one` | KEEP | Boundary: negative `age_days` must not panic or return < 1.0. The production path `age_days <= grace_period_days` where grace=0 gives `-5 <= 0 → 1.0`. Clear, unique, load-bearing. |
| `power_law_decay_custom_parameters_applies_correctly` | REMOVE | Calls `power_law_formula(10.0, 2.0, 0.3)` — a function defined **only** in `tests/decay_function/support.rs`. Production code hardcodes `A=1.0, B=0.5` and exposes no API that accepts custom parameters. This test verifies that the support helper's internal formula matches an inline computation of the same formula. It exercises zero production behavior. Removing it has no effect on production code coverage. |

---

## Recommendations

1. **Remove `power_law_decay_custom_parameters_applies_correctly`** outright. It tests a test-only helper with no production equivalent.
2. **Merge `never_exceeds_one` + `never_goes_below_zero`** into a single `power_law_decay_weight_is_bounded_between_zero_and_one` that checks both bounds in one loop.
3. **Merge `below_day_one_caps_at_one`** into `zero_age_returns_weight_one` — they test the same code branch with cosmetically different setups.
4. **Rewrite `slower_than_exponential`** with an explicit ordering assertion (`assert!(power_law > exponential)`) and document the configuration context so the comparison is reproducible.
5. **Rewrite `produces_different_curve`** as a table of expected values at `[1, 4, 16, 64, 256]` days vs pre-computed `effective_age^(-0.5)` values. This catches formula drift instead of mere non-identity.
6. **Parameterization opportunity** — `basic_calculation`, `at_day_one`, `zero_age`, `negative_age` and `memory_halves` all call `compute_power_law_decay` with an `age_days` scalar and assert a specific float. These could be a single `#[rstest]` table with `(age_days, grace_days, expected)` rows, reducing boilerplate while preserving all coverage.

---

## Load-Bearing Score: 7/11

`basic_calculation`, `at_day_one`, `zero_age`, `never_exceeds_one`/`never_goes_below_zero` (count as one), `memory_halves_when_time_quadruples`, and `negative_age` are load-bearing. `below_day_one` is a duplicate. `slower_than_exponential` and `produces_different_curve` have assertions too weak to catch realistic regressions. `custom_parameters` tests nothing in production.
