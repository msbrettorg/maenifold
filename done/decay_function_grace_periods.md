# Test Evaluation: decay_function_grace_periods

**File**: `crates/maenifold-core/tests/decay_function_grace_periods.rs`
**Traceability**: T-12.6 · NFR-1 · NFR-2
**Evaluator**: Red Team (Gray — Skeptical Inquiry)
**Date**: 2025-07-22

---

## Summary

This file contains 4 tests targeting the grace period logic of the exponential decay function across
three path-type tiers: sequential thinking (`/thinking/sequential/`), workflow
(`/thinking/workflow/`), and default (everything else). The declared intent — per test names — is to
verify that each path type routes to its correct grace period constant.

**The intent and the implementation diverge in every test except Test 4.** The shared helper
`compute_exponential_decay` (in `decay_function/support.rs`) collapses all three grace-period env
vars to a single value, making the path passed to Tests 1–3 **irrelevant to the assertion**. The
routing function `grace_period_for_path` is exercised but not discriminated. No test in this file
would fail if the routing function was deleted and replaced with a fixed constant equal to the
injected value.

Additionally, `DEFAULT_GRACE_PERIOD_DAYS = 14` in `decay_function/support.rs` mismatches the
production default in `Config::decay_grace_days_default()` which is **28** (confirmed:
`env_i64("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", 28)`). Test 3 asserts behavior at grace=14 while
claiming to test "default memory" — it tests a non-default configuration under a misleading name.

The correctly-named neighboring suite `decay_calculator_path_grace.rs` provides superior routing
coverage (using `DecayEnvGuard::exponential_defaults()` which correctly uses 7/14/28), making this
file's first three tests substantially redundant.

---

## Findings by Test

---

### Test 1 — `exponential_decay_sequential_thinking_uses_7_day_grace_period`

**Traceability annotation**: `T-12.6: RTM NFR-1`

**What the name promises**: Sequential thinking paths route to the 7-day grace period.

**What the code does**:

```rust
let at_grace = compute_exponential_decay(
    &env.cfg,
    "memory://thinking/sequential/2026/03/at-grace",
    SEQUENTIAL_GRACE_PERIOD_DAYS,   // 7
    SEQUENTIAL_GRACE_PERIOD_DAYS,   // 7  ← grace_days passed to helper
);
```

`compute_exponential_decay` calls `DecayEnvGuard::set("exponential", 30, grace_days, grace_days,
grace_days)` — all three grace-period env vars are set to 7. Because all paths now return 7,
`grace_period_for_path` produces the same result for a sequential path, a workflow path, or a
notes path. The path string is a spectator.

The assertions verify:
- `age = grace (7)` → score = 1.0 ✓  (boundary gate: `age_days <= grace_period_days`)
- `age = grace + 1 (8)` → score < 1.0 ✓  (one-day past grace: decay begins)

Both assertions are mathematically correct. **Neither catches a routing bug.**

**Comparison with `decay_calculator_path_grace.rs`**: `windows_backslash_sequential_path_uses_sequential_grace_period` and `sequential_path_has_shorter_grace_than_default_path` cover sequential routing with correct defaults (7/14/28), making this test's routing contribution redundant.

**Unique contribution**: Exercises the grace-period boundary gate (`<=` vs `<`) at grace=7. This is distinct from any test in `decay_calculator_path_grace.rs`, which does not test age=grace exactly.

---

### Test 2 — `exponential_decay_workflows_uses_14_day_grace_period`

**Traceability annotation**: none (inherits file-level `T-12.6`)

**What the name promises**: Workflow paths route to the 14-day grace period.

**What the code does**: Identical structure to Test 1. `compute_exponential_decay` is called with
`grace_days = WORKFLOWS_GRACE_PERIOD_DAYS = 14`, setting ALL three grace env vars to 14. The
workflow path `memory://thinking/workflow/2026/03/at-grace` routes to `config.decay_grace_days_workflows()` = 14 (from env), but default also = 14, sequential also = 14. Routing is untested.

The assertions:
- `age = 14` → 1.0 ✓
- `age = 15` → < 1.0 ✓

**Structural duplicate of Test 3**: Both tests pass `grace_days = 14` to the same helper with
different paths. Since the helper normalises all graces to 14, both tests produce identical
computations through identical code paths. Together they contribute one unique datum: the 14-day
boundary gate.

**Comparison with `decay_calculator_path_grace.rs`**: `thinking_workflow_path_uses_workflow_grace_period` tests `age=14` → `1.0` with correct defaults, making this test's routing contribution redundant. `decay_calculator_path_grace.rs` does not, however, test `age=15`, so the "just past grace" assertion has marginal uniqueness.

---

### Test 3 — `exponential_decay_default_memory_uses_14_day_grace_period`

**Traceability annotation**: none

**What the name promises**: Default (non-special) memory paths use a 14-day grace period.

**What the code does**: Same helper, `grace_days = DEFAULT_GRACE_PERIOD_DAYS = 14`.

**Critical defect — wrong constant**:

| Source | Value |
|--------|-------|
| `decay_function/support.rs`: `DEFAULT_GRACE_PERIOD_DAYS` | **14** |
| `Config::decay_grace_days_default()` in `config.rs` | **28** (`env_i64("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", 28)`) |
| `decay_calculator/support.rs`: `exponential_defaults()` | **28** ✓ |

The constant is wrong. The test does not verify the production default at all — it forces grace=14
via env var while claiming to test "default memory." A developer reading this test would believe the
default grace period is 14 days, which is false. Removing the grace-period env override and running
against the actual default (28) would produce a completely different result at age=14 (score=1.0
for the wrong reason — still within grace).

**Structural duplicate of Test 2**: Both test the `grace=14` boundary. Together they add zero
incremental assertion coverage that the other doesn't already provide.

---

### Test 4 — `exponential_decay_tiered_grace_periods_sequential_decays_faster`

**Traceability annotation**: none (inherits file-level `T-12.6`)

**What the name promises**: At the same age, sequential content decays faster than workflow/default.

**What the code does**: This test correctly sets differentiated grace periods:

```rust
let _env = DecayEnvGuard::set(
    "exponential",
    DEFAULT_HALF_LIFE_DAYS,
    SEQUENTIAL_GRACE_PERIOD_DAYS,  // 7
    WORKFLOWS_GRACE_PERIOD_DAYS,   // 14
    DEFAULT_GRACE_PERIOD_DAYS,     // 14 ← still wrong constant, but env is set
);
// age = 10 days for all three paths
```

At age=10:
- Sequential (grace=7): `age > grace` → decaying → score < 1.0 ✓
- Workflow (grace=14): `age <= grace` → score = 1.0
- Default (grace=14): `age <= grace` → score = 1.0

Assertions:
- `sequential < workflow` ✓ — genuinely load-bearing: catches routing inversion
- `sequential < default` ✓ — genuinely load-bearing
- `assert_approx_eq(workflow, default, 1e-9)` ← **weak assertion**

**Problem with `assert_approx_eq(workflow, default, 1e-9)`**: Both workflow and default are
`1.0` exactly (within their grace period at day 10). The tolerance `1e-9` is applied to the
comparison `|1.0 - 1.0| = 0.0`. This is equivalent to `assert_eq!(1.0, 1.0)` and confirms nothing
about whether workflows and defaults have equivalent *decay curves* — only that they share a grace
period at this specific age. If someone changed the workflow grace period to 11 days, this assertion
would still pass at age=10 (both still in grace).

**Missed coverage**: There is no assertion at an age *past* grace (e.g., age=20) verifying that
workflow and default produce equal decay scores after the grace window. The equivalence of their
decay curves is untested for the portion that actually decays.

**Fragile constant dependency**: `DEFAULT_GRACE_PERIOD_DAYS = 14` is used in the env guard.
While the env guard correctly overrides the env var, the wrong constant means this test silently
tests a non-production configuration. If someone corrects the constant to 28, this test
would need to be restructured (age=10 with default grace=28 still places default in grace at day 10 — assertion still passes — but the intent should reflect the real constant).

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:-------------:|:-------:|:-------------------:|:----------:|:----------------:|---------|
| 1 | `exponential_decay_sequential_thinking_uses_7_day_grace_period` | ⚠ Partial | ⚠ Partial | ✅ Yes | ❌ Misleading | ✅ Yes | **REWRITE** |
| 2 | `exponential_decay_workflows_uses_14_day_grace_period` | ⚠ Partial | ❌ Duplicate of T3 | ✅ Yes | ❌ Misleading | ✅ Yes | **MERGE** |
| 3 | `exponential_decay_default_memory_uses_14_day_grace_period` | ❌ Low | ❌ Duplicate of T2 | ❌ Wrong constant | ❌ Misleading | ✅ Yes | **REWRITE** |
| 4 | `exponential_decay_tiered_grace_periods_sequential_decays_faster` | ✅ Yes | ✅ Yes | ⚠ Weak tolerance | ✅ Good | ⚠ Partial | **REWRITE** |

---

## Detailed Verdict Rationale

### Test 1 → REWRITE

The 7-day boundary gate (`age=grace → 1.0`, `age=grace+1 → <1.0`) is genuinely useful but the
test must be rewritten to actually isolate sequential routing:

1. Set grace periods **differently per tier** (e.g., sequential=7, workflows=28, default=28)
2. Assert that a sequential path at age=8 decays while a default path at age=8 does not
3. Rename to `sequential_path_boundary_is_7_days` — drop the claim about "uses" which implies
   routing discrimination the test doesn't provide

Alternatively, collapse Tests 1–3 into a parameterized test using `rstest` covering (path_type,
grace_days, age_at_boundary) tuples, with routing isolation verified once in Test 4.

### Test 2 → MERGE

Tests 2 and 3 are structurally identical — both call `compute_exponential_decay` with
`grace_days=14` through different path strings that produce identical results. Merge into one
parameterized case covering the 14-day boundary. The "just past grace" assertion (`age=15 → <1.0`)
has marginal uniqueness vs. `decay_calculator_path_grace.rs` and should be preserved after merge.

### Test 3 → REWRITE

**Priority fix**: Correct `DEFAULT_GRACE_PERIOD_DAYS` from `14` to `28` in
`decay_function/support.rs` to match `Config::decay_grace_days_default()`. After correction,
rewrite to test the actual 28-day default boundary:

- `age=28` → 1.0 (at grace)
- `age=29` → < 1.0 (just past grace)
- Rename to `default_path_boundary_is_28_days`

This is the highest-priority defect in this file: **a wrong constant causes the constant name and
the production value to diverge silently**, and every test downstream that uses
`DEFAULT_GRACE_PERIOD_DAYS` inherits the false belief that 14 is the default.

### Test 4 → REWRITE

Preserve the tiered differential structure; fix two issues:

1. **Change the age to post-grace for workflow/default comparison**: Use `age=20` (past all grace
   periods for a hypothetical sequential, but within grace for workflow/default if grace=14; or use
   age=30 if testing true equivalence of the decay curve after grace ends). Alternatively, add a
   second assertion block at `age=30` to verify `workflow ≈ default` during active decay.
2. **Use exact equality where appropriate**: `assert_eq!(workflow, default)` — same grace, same
   half-life, same age — the values must be bit-identical, not just within 1e-9.
3. **Correct the `DEFAULT_GRACE_PERIOD_DAYS` constant** (same fix as Test 3). At age=10 with
   grace=14 vs grace=28, both workflow and default are still in grace — the test passes either way —
   but the constant should still be truthful.

---

## Recommendations

### R-1 (High): Fix `DEFAULT_GRACE_PERIOD_DAYS` constant in `decay_function/support.rs`

```rust
// Before (wrong)
pub const DEFAULT_GRACE_PERIOD_DAYS: i64 = 14;

// After (matches Config::decay_grace_days_default)
pub const DEFAULT_GRACE_PERIOD_DAYS: i64 = 28;
```

This is a silent lie embedded in a public test constant. Any future test author using this constant
will write tests against the wrong production behavior. **Fix before other rewrites.**

### R-2 (High): Add routing isolation to at least one test in this file

None of Tests 1–3 can detect a broken `grace_period_for_path`. Either:
- Rewrite Test 1 to use differentiated grace periods (not the flat-all-same helper), or
- Add a dedicated test: `sequential_path_selects_7_day_grace_not_default_28_day_grace`

### R-3 (Medium): Merge Tests 2 and 3 into a parameterized boundary test

Using `rstest` or a loop, collapse the three "boundary at grace" tests into:

```rust
#[test]
fn grace_period_boundary_gate_returns_1_at_exactly_grace_and_decays_at_grace_plus_1() {
    // parameterized: (7, sequential_path), (14, workflow_path), (28, default_path)
}
```

This eliminates structural duplication while preserving the assertion that each tier's boundary gate
behaves correctly.

### R-4 (Medium): Strengthen Test 4's `assert_approx_eq` at grace boundary

Replace:
```rust
assert_approx_eq(workflow, default, 1e-9);
```
With:
```rust
assert_eq!(workflow, 1.0, "workflow at day 10 should be in grace period");
assert_eq!(default, 1.0, "default at day 10 should be in grace period");
// Then add a second block at age=30 (past grace for all) to prove equal decay curves:
let workflow_post = compute_with_last_accessed(&env.cfg, "memory://thinking/workflow/...", &post_grace_accessed, None);
let default_post = compute_with_last_accessed(&env.cfg, "memory://projects/...", &post_grace_accessed, None);
assert_eq!(workflow_post, default_post, "workflow and default decay at identical rates after grace");
```

### R-5 (Low): Add `T-12.6` traceability annotation to Tests 2, 3, 4

Only Test 1 carries the `// T-12.6: RTM NFR-1` annotation. Tests 2–4 are unlabeled, breaking the
NFR-2 traceability expectation that each test can be linked to a requirement.

---

## Load-Bearing Score: 2/4

| Test | Load-Bearing Assessment |
|------|------------------------|
| Test 1 | **Partial** — covers the boundary gate math at grace=7, but routing is not isolated. Would not catch a deletion of `grace_period_for_path`. |
| Test 2 | **Partial** — covers boundary gate at grace=14, but is structurally identical to Test 3. Together they contribute one unit of load. |
| Test 3 | **Low** — structural duplicate of Test 2, wrong constant, tests non-default configuration under a misleading name. |
| Test 4 | **Yes** — the only test that differentiates grace periods by path type. Would catch routing inversions and incorrect tier assignment. |

Tests 1 and 4 carry distinct, non-redundant load. Tests 2 and 3 together carry one shared unit of
load (the 14-day boundary gate), with Test 3 additionally carrying a correctness defect that
undermines its own contribution.

---

## SWE Remediation

### Changes

- **R-1 (P0)**: Fixed `DEFAULT_GRACE_PERIOD_DAYS` from `14` to `28` in `decay_function/support.rs` to match `Config::decay_grace_days_default()`.
- **MERGE Tests 1–3 → `grace_period_boundary_gate_per_tier`**: Collapsed three structurally-duplicate boundary-gate tests into one parameterized test using differentiated grace periods (7/14/28). Uses `DecayEnvGuard::set` directly instead of the flat `compute_exponential_decay` helper that masked routing bugs. Includes routing isolation assertion: sequential at age=8 decays while default at age=8 remains in grace.
- **REMOVE**: `exponential_decay_sequential_thinking_uses_7_day_grace_period` (subsumed by merged test), `exponential_decay_workflows_uses_14_day_grace_period` (structural duplicate), `exponential_decay_default_memory_uses_14_day_grace_period` (wrong constant, structural duplicate).
- **REWRITE Test 4 → `tiered_grace_periods_sequential_decays_faster_with_post_grace_ordering`**: Replaced weak `assert_approx_eq(workflow, default, 1e-9)` (trivially true — both were 1.0) with exact equality assertions. Added Block 2 at age=30 verifying full tier ordering in active decay: `sequential < workflow < default`.
- **R-5**: Added `T-12.6: RTM NFR-1, NFR-2` traceability annotations to all tests and file header.

### Test count: 4 → 2

| Old Test | Verdict | New Test |
|----------|---------|----------|
| Test 1 `exponential_decay_sequential_thinking_uses_7_day_grace_period` | MERGE | `grace_period_boundary_gate_per_tier` |
| Test 2 `exponential_decay_workflows_uses_14_day_grace_period` | MERGE | `grace_period_boundary_gate_per_tier` |
| Test 3 `exponential_decay_default_memory_uses_14_day_grace_period` | MERGE | `grace_period_boundary_gate_per_tier` |
| Test 4 `exponential_decay_tiered_grace_periods_sequential_decays_faster` | REWRITE | `tiered_grace_periods_sequential_decays_faster_with_post_grace_ordering` |

- Commit: ddafda1
