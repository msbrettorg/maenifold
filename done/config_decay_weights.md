---
id: config_decay_weights
status: done
pipeline: tdd
---

# Test Evaluation: config_decay_weights
**File:** `crates/maenifold-core/tests/config_decay_weights.rs`
**Support:** `crates/maenifold-core/tests/config_decay/support.rs`
**Test count:** 7
**Evaluated:** 2026-07-07
**Traceability:** T-GRAPH-DECAY-001.2-6 · T-GRAPH-DECAY-001.4 · T-GRAPH-DECAY-001.6 · RTM NFR-7.5.x · NFR-7.5.3 · NFR-7.5.5

---

## Summary

The suite tests that `memory::compute_decay_weight` respects runtime configuration — grace
periods, half-life, and function selection — rather than baking in defaults. That is a
legitimate and important responsibility. However, skeptical examination reveals three
structural problems that undermine confidence:

1. **Tests 1–3 are structurally identical.** All three set age = config grace period,
   assert weight = 1.0, and differ only in the URI path. They are prime parameterization
   targets whose current form adds noise without proportional signal.

2. **Two tests hardcode the expected value `0.1826`.** This is `30^(-0.5)`, which is
   correct only because the default `MAENIFOLD_DECAY_HALF_LIFE_DAYS` is 30. The tests
   use `env.cfg.decay_half_life_days()` to drive the *input* age, but not to derive the
   *expected output*. If the default changes, the assertion silently breaks. The tolerance
   (`0.01`) is wide enough to mask a partially wrong formula.

3. **Test 6 mutates the environment mid-body.** After acquiring the mutex and clearing env
   vars, it calls `set_var` in the middle of the test function rather than via a guard.
   The cleanup happens correctly (the `_env` guard restores to pre-clear state), but the
   in-body mutation is a code smell: it relies on `decay_function()` being a live env
   read and on the test author knowing that `env.cfg` carries no cached copy. Both
   conditions happen to hold today; neither is guaranteed to hold after a refactor.

Additionally, Test 6's divergence assertion (`abs() > 0.01`) is weak. It would pass even
if both formulae were computing nonsense, provided they happened to disagree. The
specificity already present in Tests 4 and 5 makes Test 6 redundant — if those are
correct, divergence is a logical consequence.

The support module (`support.rs`) is well-structured: the `DecayEnvGuard` RAII pattern,
`acquire_test_lock`, and `isolated_env` correctly serialize env mutations. These are
positive, and any rewrite should preserve them.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `compute_decay_weight_sequential_path_uses_config_grace_period` | **MERGE** | Identical structure to the workflow and default variants; all three should become one parameterized test |
| `compute_decay_weight_workflow_path_uses_config_grace_period` | **MERGE** | Same as above — duplicate structure, different URI only |
| `compute_decay_weight_default_path_uses_config_grace_period` | **MERGE** | Same as above — retain as canonical form, fold the others in |
| `compute_decay_weight_uses_config_half_life` | **REWRITE** | Hardcodes `0.1826` without deriving it from `decay_half_life_days()`; name does not indicate power-law |
| `compute_decay_weight_exponential_function_applies_correctly` | **KEEP** | Mathematically sound anchor: at exactly one half-life, exponential decay must give 0.5. Rename for clarity. |
| `compute_decay_weight_decay_function_selection_changes_result_curve` | **REMOVE** | Weak divergence check (`abs() > 0.01`) is already implied by Tests 4 and 5; mid-body env mutation is fragile |
| `empty_decay_function_falls_back_to_power_law` | **REWRITE** | Asserts three independent concerns in one body; `0.1826` hardcoded; split whitespace-fallback config check from weight computation |

---

## Detailed Findings

### T1–T3: Grace Period Trio — MERGE

```rust
// All three follow this pattern, differing only in path and accessor:
let last_accessed = rfc3339_days_ago(env.cfg.decay_grace_days_sequential()); // or _workflows / _default
let weight = memory::compute_decay_weight("memory://thinking/sequential/...", ...);
assert_approx_eq(weight, 1.0, 0.0001);
```

**What they prove:** That `grace_period_for_path` dispatches to the correct config
accessor for each URI namespace, and that the boundary condition (`age_days ==
grace_period_days`) returns `1.0`.

**What they assume:** That the boundary is inclusive (`<=`). The production code confirms
`if age_days <= grace_period_days { return 1.0; }` — so the assumption holds, but the
tests do not also verify `age_days = grace_period_days + 1` returns `< 1.0`. That
off-by-one gap is left uncovered.

**Recommended replacement:**
```rust
#[test]
fn compute_decay_weight_path_type_uses_its_config_grace_period() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    let cases = [
        ("memory://thinking/sequential/2026/03/s", env.cfg.decay_grace_days_sequential()),
        ("memory://thinking/workflows/2026/03/w", env.cfg.decay_grace_days_workflows()),
        ("memory://notes/n",                       env.cfg.decay_grace_days_default()),
    ];
    for (path, grace_days) in cases {
        let last_accessed = rfc3339_days_ago(grace_days);
        let weight = memory::compute_decay_weight(path, Some(&last_accessed), &env.cfg, None);
        assert_approx_eq(weight, 1.0, 0.0001, "path={path}");
    }
}
```

This collapses three identical tests into one without losing coverage.

---

### T4: Half-Life Value — REWRITE

```rust
assert_approx_eq(weight, 0.1826, 0.01);
```

**Problem:** `0.1826 ≈ 30^(−0.5)`. The test hardcodes `30` but reads the actual grace
days from config. If `MAENIFOLD_DECAY_HALF_LIFE_DAYS` default ever changes, the input age
is still `grace + new_half_life` but the expected output no longer matches `0.1826` — the
test will fail for the wrong reason, or worse, the tolerance of `0.01` may silently absorb
the discrepancy.

**Also:** The name omits "power-law". A reader cannot tell which function is under test
without reading the body.

**Recommended replacement:**
```rust
#[test]
fn power_law_weight_at_one_half_life_matches_formula() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let half_life = env.cfg.decay_half_life_days() as f64;
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);
    let expected = half_life.powf(-0.5); // power-law formula made explicit

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note", Some(&last_accessed), &env.cfg, None
    );

    assert_approx_eq(weight, expected, 0.0001);
}
```

---

### T5: Exponential at Half-Life — KEEP (rename)

This test correctly anchors the exponential formula: at `age = grace + half_life`, the
exponent is 1.0, so `0.5^1.0 = 0.5` exactly. No magic number — the expected value is
derivable from first principles.

**Minor:** The name "applies_correctly" is underdetermined. Suggest:
`compute_decay_weight_exponential_at_half_life_gives_point_five`.

---

### T6: Divergence Check — REMOVE

```rust
assert!(
    (power_law - exponential).abs() > 0.01,
    "power-law and exponential decay should diverge: ..."
);
```

**Why it adds no value:** If Tests 4 and 5 are green:
- Power-law at one half-life ≈ `half_life^(-0.5)` (≈ 0.183 for default=30)
- Exponential at one half-life = `0.5`

The divergence is `|0.183 − 0.5| ≈ 0.317`. The `> 0.01` threshold would be satisfied even
if both formulae were badly wrong, provided they remained distinguishable. This test does
not verify correctness; it verifies non-identity, which is already implied by the specific
value checks.

**Mid-body env mutation is also a concern.** The test calls `std::env::set_var` directly
in the function body after the `_env` guard is active, bypassing the `apply_decay_env`
helper pattern used everywhere else. The SAFETY comment is correct (mutex is held), but
the pattern is inconsistent and fragile across refactors.

---

### T7: Whitespace Fallback — REWRITE

This test asserts three distinct behaviors in a single function body:

1. `assert_eq!(env.cfg.decay_function(), "power-law")` — config accessor trims and
   rejects whitespace (a property of `env_string`, not of `compute_decay_weight`)
2. `assert!(summary.contains("Function: power-law"))` — config tool output formatting
3. `assert_approx_eq(weight, 0.1826, 0.01)` — decay weight correctness

These are orthogonal concerns. Failure in (3) could be caused by (1) being wrong, but
neither helps diagnose the other. The `0.1826` hardcoding carries the same risk as T4.

**Recommended split:**
```rust
#[test]
fn whitespace_decay_function_falls_back_to_power_law_config_value() {
    // Only tests that env_string trims and defaults correctly
    ...
    assert_eq!(env.cfg.decay_function(), "power-law");
    assert!(summary.contains("Function: power-law"));
}

#[test]
fn whitespace_decay_function_produces_power_law_weight() {
    // Tests that the fallback actually routes through power-law formula
    let half_life = env.cfg.decay_half_life_days() as f64;
    let expected = half_life.powf(-0.5);
    ...
    assert_approx_eq(weight, expected, 0.0001);
}
```

---

## Positive Observations

- **`acquire_test_lock()` + `DecayEnvGuard` pattern** is correct and necessary. Process-wide
  env vars are a global mutable state hazard in parallel Rust test runs; the mutex
  serialization approach is the right solution.
- **`isolated_env()` / `Config::override_root`** provides genuine filesystem isolation.
  Tests do not bleed state into each other via config or file paths.
- **All tests read grace-period and half-life values from `env.cfg`** (not hardcoded) for
  their *input* construction. Only the *expected output* values are hardcoded — that
  asymmetry is the precise defect identified above.
- **RTM traceability tags** (`// T-GRAPH-DECAY-001.2-4`, etc.) are present on every test
  and align to corresponding tags in `config.rs`. The traceability discipline is good.

---

## Recommendations

1. **Merge T1–T3** into a single parameterized test. Saves 40 lines, no coverage loss,
   and adds the missing `grace_period + 1` off-by-one sub-case.

2. **Rewrite T4** to derive its expected value from `decay_half_life_days()` rather than
   hardcoding `0.1826`. Tighten tolerance from `0.01` to `0.0001`.

3. **Keep T5**, rename to `..._at_half_life_gives_point_five` for precision.

4. **Remove T6.** Its weak divergence assertion is logically redundant with T4+T5, and
   the mid-body env mutation is inconsistent with the rest of the module.

5. **Rewrite T7** as two separate tests: one asserting the config-value whitespace
   fallback, one asserting the resulting weight (with a derived, not hardcoded, expected
   value).

6. **Add a missing boundary test**: `age_days = grace_period + 1` should return `< 1.0`
   for all path types. Currently no test verifies the transition out of the grace window.

---

## Load-Bearing Score: 5/7

| # | Test | Load-Bearing? |
|---|---|---|
| T1 | sequential grace period | ✅ Yes — but duplicated |
| T2 | workflow grace period | ✅ Yes — but duplicated |
| T3 | default grace period | ✅ Yes — canonical of the trio |
| T4 | half-life value | ✅ Yes — formula anchor (hardcoding risk) |
| T5 | exponential half-life | ✅ Yes — clean anchor |
| T6 | divergence check | ❌ No — weak assertion, redundant |
| T7 | whitespace fallback | ✅ Yes — unique edge case (multi-concern smell) |

Net after applying verdicts: 4 distinct behaviors remain testable with 5 tightly-scoped
tests (3 merged → 1, T4 rewritten, T5 kept, T6 removed, T7 split → 2).

---
## SWE Remediation
- MERGE T1–T3 → `compute_decay_weight_path_type_uses_its_config_grace_period`: parameterized over sequential/workflow/default paths with grace boundary (=1.0) and grace+2 off-by-one (<1.0) sub-cases. Note: grace+1 returns 1.0 because power-law `effective_age=1` gives `1^(-0.5)=1.0` (mathematical identity, not a grace inclusion).
- REWRITE T4 → `power_law_weight_at_one_half_life_matches_formula`: expected value derived from `decay_half_life_days()` via `half_life.powf(-0.5)` instead of hardcoded `0.1826`; tolerance tightened from 0.01 to 0.0001.
- KEEP T5 → `compute_decay_weight_exponential_at_half_life_gives_point_five`: renamed from `..._applies_correctly` for precision; body unchanged.
- REMOVE T6 (`compute_decay_weight_decay_function_selection_changes_result_curve`): weak divergence assertion redundant with T4+T5; mid-body env mutation eliminated.
- REWRITE T7 → split into `whitespace_decay_function_falls_back_to_power_law_config_value` (config accessor + summary assertions) and `whitespace_decay_function_produces_power_law_weight` (derived expected value, tolerance 0.0001).
- Final test count: 5 (down from 7), covering 4 distinct behaviors with stronger assertions.
- Commit: 9277798
