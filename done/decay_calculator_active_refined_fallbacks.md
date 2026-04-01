---
id: T-80.51
title: "Test Evaluation: decay_calculator_active_refined_fallbacks"
status: done
pipeline: tdd
---

# Test Evaluation: decay_calculator_active_refined_fallbacks

**File:** `crates/maenifold-core/tests/decay_calculator_active_refined_fallbacks.rs`
**Traceability:** T-7.8 / T-7.6.1 / T-12.7 / FR-7.8 / RTM NFR-1
**Test count:** 13
**Evaluated:** 2026-07-15

## Summary

This is the strongest of the three evaluated files. Ten of thirteen tests are correctly specified and load-bearing. One test (`absent_status_uses_path_defaults`) compares two derived values against each other rather than asserting a specific expected weight — it would pass even if both computations were identically wrong. Two case-insensitivity tests are redundant and should be merged into a single parameterized test. The numeric test (`active_status_past_grace_period_decays_with_thirty_day_half_life`) is the gold standard for this suite: it explicitly encodes the mathematical formula in the assertion, making the expected value self-documenting.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `active_status_within_grace_period_returns_one` | **KEEP** | Correctly pins the grace-period boundary: age=10 ≤ grace=14 → 1.0. If the grace period override for "active" status (`grace_period_days = 14`) were removed or altered, this test fails. Load-bearing. |
| `active_status_past_grace_period_decays_with_thirty_day_half_life` | **KEEP** | The best test in the file. Encodes the formula explicitly in the fixture: `age = grace(14) + half_life(30) = 44` days yields exponent = 1.0 → weight = 0.5. The 0.01 tolerance is appropriate. Anyone who accidentally changes either constant will be caught immediately. |
| `active_status_is_case_insensitive` | **MERGE** | The case-insensitivity rule (`to_ascii_lowercase`) is tested for "active" here and for "refined" in `refined_status_is_case_insensitive`. Both go through the exact same `match status.trim().to_ascii_lowercase().as_str()` arm. Collapse into a single parameterized test with four rows: `("active", "ACTIVE")`, `("refined", "REFINED")`. |
| `refined_status_within_grace_period_returns_one` | **KEEP** | Distinct from test 1: explicitly verifies that the "refined" status keyword triggers the same grace-period override as "active". If someone forgot to add "refined" to the match arm, test 1 would pass but this would fail. |
| `refined_status_uses_same_parameters_as_active` | **KEEP** | High-value design-intent test. Verifies the explicit product decision that "refined" and "active" are aliases with identical decay parameters. If they ever diverge accidentally, this is the only test in the suite that catches it. Unique. |
| `refined_status_is_case_insensitive` | **MERGE** | See `active_status_is_case_insensitive`. Same code path, merge into one parameterized test. |
| `unknown_status_falls_back_to_path_defaults` | **KEEP** | Tests the `_ => {}` arm of the status match — an unrecognized status string should not trigger any special behavior. Load-bearing: if someone added a catch-all that returns 1.0 for unknown statuses, this would catch it (assuming the path-default weight at 60 days < 1.0). |
| `empty_status_falls_back_to_path_defaults` | **KEEP** | Distinct from test 7. An empty string after `.trim()` is still an empty string, which hits `_ => {}`. However, an implementation bug could check `status.is_some()` before trimming and treat `Some("")` as "status provided." This covers that distinct edge case. Worth keeping separate. |
| `absent_status_uses_path_defaults` | **REWRITE** | Compares two derived values against each other: `weight_for("memory://assumptions/.../no-status", Some(60), None)` vs `weight_for("memory://notes/.../default-path", Some(60), None)`. Both use the same grace period (28 days default), same half-life (30 days), same function (exponential). The result is: `0.5^((60-28)/30) ≈ 0.479`. **The test would pass even if both computations were identically wrong.** Replace with: `assert_approx_eq(absent, 0.5f64.powf((60.0 - 28.0) / 30.0), 0.01)`. This pins the actual expected numeric value derived from the configured defaults, which is the same self-documenting approach used in test 2. |
| `whitespace_status_falls_back_to_path_defaults` | **KEEP** | A whitespace-only status string (e.g., `"   "`) could plausibly bypass a `status.is_empty()` guard. After `.trim()`, it becomes empty and hits `_ => {}`. Distinct from both test 7 (non-empty unrecognized) and test 8 (empty string). |
| `missing_last_accessed_defaults_to_one` | **KEEP** | Tests the "no timestamp at all" case: both `last_accessed = None` and `created = None` → returns 1.0 (no decay without evidence of age). This is a load-bearing contract — removing the early return on `None` reference_date would break this. |
| `recent_last_accessed_keeps_weight_at_one` | **KEEP** | Tests grace period behavior for a plain notes path (not assumption-specific). `age = 1 ≤ grace = 28 → 1.0`. Distinct from test 1, which tests assumption-path + status-override grace period. This exercises the `grace_period_for_path` default branch. |
| `recent_last_accessed_produces_higher_weight_than_old_last_accessed` | **KEEP** | Monotonicity invariant: `weight(age=1) > weight(age=60)`. This is a broad sanity check that catches an inverted formula or a broken exponential. Correctly uses a structured error message. |

## Recommendations

1. **`absent_status_uses_path_defaults` — rewrite** to assert the specific expected value rather than comparing two derived outputs. Use:
   ```rust
   let expected = 0.5f64.powf((60.0 - 28.0) / 30.0);
   assert_approx_eq(absent, expected, 0.01);
   ```
   This is the same approach as `active_status_past_grace_period_decays_with_thirty_day_half_life` and makes the test independently verifiable.

2. **Merge `active_status_is_case_insensitive` + `refined_status_is_case_insensitive`** into one parameterized test:
   ```rust
   #[rstest]
   #[case("active", "ACTIVE")]
   #[case("active", "Active")]
   #[case("refined", "REFINED")]
   #[case("refined", "Refined")]
   fn status_matching_is_case_insensitive(#[case] lower: &str, #[case] upper: &str) {
       // assert weight(lower, 50) ≈ weight(upper, 50)
   }
   ```

3. **Consider adding:** a test for the `validated` status (returns 1.0 always, no decay) — that's the sibling of "active/refined" in the same match arm and belongs in `decay_calculator_validated_invalidated.rs`, but confirming it's absent here is good hygiene. This suite (active/refined/fallbacks) correctly focuses on its named behaviors.

4. **Test 2 is the template for this entire file.** The formula `age = grace + half_life → weight = 0.5` is self-documenting. Tests 1, 4, 11, 12 correctly use exact `== 1.0` assertions. Test 9 should follow test 2's pattern of deriving the expected value from the configured constants.

## Load-Bearing Score: 10/13

Tests 3 and 6 are load-bearing but redundant with each other (same rule, two separate functions). Test 9 is structurally correct but its assertion strategy gives false confidence. The remaining 10 are correctly specified and would individually catch distinct regressions.

---
## SWE Remediation
- MERGE active_status_is_case_insensitive + refined_status_is_case_insensitive → status_matching_is_case_insensitive: 4-case loop (active/ACTIVE, active/Active, refined/REFINED, refined/Refined) since rstest not available
- REWRITE absent_status_uses_path_defaults: replaced derived-vs-derived comparison with `assert_approx_eq(absent, 0.5f64.powf((60.0 - 28.0) / 30.0), 0.01)` pinning to configured defaults (grace=28, half_life=30)
- KEEP: 10 tests unchanged
- Test count: 13 → 12 (all pass)
- Commit: cd6836e
