---
id: decay_calculator_power_law
status: done
---

# Test Evaluation: decay_calculator_power_law

**File:** `crates/maenifold-core/tests/decay_calculator_power_law.rs`
**Test count:** 6
**Evaluated:** 2026-03-13
**Traceability:** T-12.7 (RTM NFR-1) · T-7.5.5 (RTM NFR-7.5.5 — see note)

> **Traceability note:** `NFR-7.5.5` does not appear in the current `RTM.md` (active items FR-22 through FR-27). It is not present in `CHANGELOG.md` either. All six tests carry this tag. Either the requirement predates the current RTM format, the tag is carried over from a refactor, or it references a deleted requirement. The T-12.7 / NFR-1 tag is consistent with the `TEST_MUTEX` isolation contract used across the decay suite.

---

## Summary

This file tests `compute_decay_weight` — the full calculator integration layer — with power-law configuration active. It is the correct level at which to verify that env-var routing, grace-period dispatch, and the power-law formula compose correctly end-to-end. The sibling file `decay_function_power_law.rs` tests the pure math helper at a lower level; these tests are not redundant with it.

Three tests are unambiguously load-bearing: the boundary guard (T2), the golden-value formula assertion (T3), and the case-insensitivity check (T5). Two tests have correctness problems that dilute their signal. One test has an assertion so weak it cannot distinguish a routing bug from correct behavior.

**Critical finding — Test 1:** `memory://notes/power-law-within-grace` routes to the **default** 28-day grace period. The test uses `age_days=5`, which is less than every grace period in the system (7 sequential, 14 workflow, 28 default). The assertion `weight == 1.0` passes regardless of which grace period is returned. If `grace_period_for_path` were broken and returned the sequential 7-day value instead of 28, the test would not detect it. `decay_calculator_path_grace.rs` already pins this routing with `regular_path_uses_default_grace_period`, but that test uses exponential config; there is no power-law equivalent at a discriminating age.

**Critical finding — Test 4:** The assertion `(power_law - exponential).abs() > 0.01` is non-directional. For effective age 60, power-law ≈ 0.129 and exponential = 0.25, so power-law is the *slower* decay as expected — but the test does not assert this ordering. A regression that accidentally made power-law decay *faster* than exponential would pass the test.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `power_law_within_grace_period_returns_one` | REWRITE | Uses `age_days=5` against the `memory://notes/...` path (default 28-day grace). Since 5 < every grace period (7 / 14 / 28), the assertion `weight == 1.0` passes even if path routing returns the wrong grace period. Cannot distinguish correct routing from any other category. Change to `age_days=20`: that age is within the default 28-day grace but past the sequential 7-day and workflow 14-day grace, making the assertion discriminating. |
| `power_law_at_grace_boundary_returns_one` | KEEP | Tests the exact boundary `age_days == grace_period_days` (14 == 14) for the workflow path. Load-bearing: a change from `<=` to `<` in the production implementation immediately fails this test. The only integration-level test that pins the inclusive semantics of the grace-period check for power-law. |
| `power_law_past_grace_period_calculates_effective_age_decay` | KEEP | The core mathematical golden-value test at integration level. `effective_age = (18 − 14).max(1) = 4`; `4^(−0.5) = 0.5` exactly in IEEE 754. The `1e-9` tolerance is appropriate and justified. No equivalent test exists at this layer in any sibling file. If the formula exponent, the effective-age calculation, or the grace subtraction changes, this test fails immediately. |
| `power_law_produces_different_result_from_exponential` | REWRITE | The assertion `(power_law - exponential).abs() > 0.01` does not verify which function decays faster. For effective age 60: power-law ≈ 0.129, exponential = 0.25. The correct invariant is `power_law < exponential` (power-law is the heavier-tailed function and decays slower). Replace the absolute-value check with `assert!(power_law < exponential, ...)` to encode the directional invariant. Also: both calls reuse `env.cfg` across guard swaps relying on dynamic env-var reads — add a comment explaining this assumption to prevent future readers from misunderstanding the test's structure. |
| `power_law_function_string_is_case_insensitive` | KEEP | The only integration-level test that exercises the `eq_ignore_ascii_case("power-law")` call in `scoring.rs`. Tests all three variants: `"power-law"` (lower), `"POWER-LAW"` (upper), `"Power-Law"` (mixed). If `eq_ignore_ascii_case` is replaced with `==`, the upper and mixed variants silently fall through to the exponential path and return a different value, failing this test immediately. No analog exists in any sibling file. |
| `power_law_weight_is_bounded_between_zero_and_one` | KEEP | Property invariant test over eight representative ages including inside grace (0, 1, 14), just past grace (15), and deeply past grace (30, 100, 365, 3650). The `.clamp(0.0, 1.0)` in production makes this trivially pass today, but the test remains a valid regression guard against clamp removal or a coefficient change that makes the function exceed 1.0 before clamping. Note: `age_days=15` yields `effective_age=1`, `1^(−0.5)=1.0` — still at the upper bound, which is correct and worth knowing. Correctly implements the merge recommendation issued against the sibling's split `never_exceeds_one` / `never_goes_below_zero` pair. |

---

## Recommendations

1. **Rewrite Test 1** (`power_law_within_grace_period_returns_one`): Change `age_days` from `5` to `20` and path stays `memory://notes/...`. Age 20 is within the 28-day default grace but past the 14-day workflow and 7-day sequential grace. This makes the assertion `weight == 1.0` load-bearing: it would fail if the path were incorrectly routed to any non-default grace category.

2. **Rewrite Test 4** (`power_law_produces_different_result_from_exponential`): Replace `assert!((power_law - exponential).abs() > 0.01, ...)` with:
   ```rust
   assert!(
       power_law < exponential,
       "power-law should decay slower than exponential for old content: power={power_law}, exp={exponential}"
   );
   ```
   Add a comment before the guard swaps noting that `env.cfg` reads decay configuration dynamically from env vars at call time, not at `Config::current()` construction time.

3. **Add a missing coverage test** — `power_law_assumption_path_forces_exponential`: Create a test that sets decay function to power-law, uses an assumption path (`memory://assumptions/...`), provides a status of `None`, and verifies the result is NOT the power-law value. The production code hard-switches assumption paths to exponential. This invariant is currently untested in this file and not covered by any sibling in the `decay_calculator_*` suite.

4. **Add a sequential-grace test** — `power_law_sequential_path_uses_sequential_grace_period`: Use a `memory://thinking/sequential/...` path with `age_days=10`. That age is past sequential grace (7) but within workflow (14) and default (28). Assert `weight < 1.0`. This closes the coverage gap in the `grace_period_for_path` three-way branch: the current file exercises workflow and default grace but never sequential grace with power-law config.

5. **Verify NFR-7.5.5 traceability**: All six tests carry `T-7.5.5: RTM NFR-7.5.5` but this identifier does not appear in `RTM.md` or `CHANGELOG.md`. If this is a live requirement, add it to the RTM. If it is obsolete, replace the tag with the current traceability format used in sibling files (`T-12.7: RTM NFR-1`).

6. **Parameterization opportunity**: Tests 1 (post-rewrite), 2, and 3 all call `weight_for` with a scalar age and a fixed expected value. These could be collapsed into a single `#[rstest]`-style table:
   ```
   (path, age_days, expected_weight, tolerance)
   ("memory://notes/...",           20, 1.0, 0.0)    // default grace, within
   ("memory://thinking/workflow/...", 14, 1.0, 0.0)  // workflow grace, at boundary
   ("memory://thinking/workflow/...", 18, 0.5, 1e-9) // past grace, formula value
   ```
   This would reduce boilerplate while adding more points to the curve without adding test slots.

---

## Load-Bearing Score: 4/6

**Fully load-bearing** (4): `power_law_at_grace_boundary_returns_one` (T2) pins the `<=` semantics; `power_law_past_grace_period_calculates_effective_age_decay` (T3) verifies the formula; `power_law_function_string_is_case_insensitive` (T5) guards the `eq_ignore_ascii_case` call; `power_law_weight_is_bounded_between_zero_and_one` (T6) guards the clamp invariant.

**Not adequately load-bearing** (2): `power_law_within_grace_period_returns_one` (T1) cannot distinguish a grace-period routing bug from correct behavior — its assertion is trivially true at `age_days=5` regardless of which grace period is returned; `power_law_produces_different_result_from_exponential` (T4) uses a non-directional assertion that would pass even if power-law decayed faster than exponential (the wrong direction).
