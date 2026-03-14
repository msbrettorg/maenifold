# Test Evaluation: proximity_decay_search

**File**: `crates/maenifold-core/tests/proximity_decay_search.rs`
**Support**: `crates/maenifold-core/tests/proximity_decay/support.rs`
**Test count**: 4
**Traceability annotation in file**: `T-12.0 · RTM NFR-1`
**Evaluator**: 🔘 Gray — Skeptical Inquiry
**Date**: 2025-07-23

---

## Summary

This file contains 4 tests spanning two distinct levels of abstraction that have no business
sharing a file — three pure unit tests on `compute_decay_weight` (no search involved) and one
genuine integration test that drives `SearchMemories` end-to-end. The filename
`proximity_decay_search.rs` accurately describes only Test 4; the other three are misclassified
residents that belong alongside `decay_calculator_path_grace.rs` or
`decay_function_power_law.rs`.

**The most important observation**: Tests 1–3 use `DecayEnvGuard::with_defaults()`, which sets
`MAENIFOLD_DECAY_FUNCTION = "power-law"`. This is a non-trivial distinction. The entire
`decay_calculator_path_grace.rs` suite uses `DecayEnvGuard::exponential_defaults()`.
There is **no other test file** that exercises path-specific grace period routing under the
power-law decay function. Tests 1–3 are therefore more load-bearing than they appear, but
they fail to state their actual uniqueness in either their names or traceability comments.

**Test 4 is the strongest test in the file**. It is the only test in the entire decay suite
that simultaneously asserts rank order, `fusedScore` field values, and `decayWeight` field
values from a `SearchMemories` JSON response. The `graph_decay_search.rs` suite covers similar
rank-ordering scenarios but never inspects score fields. Test 4 stands alone.

### Structural Problems

1. **Filename–content mismatch**: `proximity_decay_search.rs` implies all tests relate to
   search. Tests 1–3 test a pure computation function. Readers expecting search tests will
   find unit tests; readers auditing unit test coverage won't think to look here.

2. **Tests 1 and 2 are under-specified**: Both use the sequential path (`thinking/sequential/`)
   but their names say nothing about which path type is under test, which decay function is
   active (power-law via the guard), or what the expected numeric magnitude is. Test 2
   asserts only `weight < 1.0` — a claim so weak it would pass if decay returned `0.9999`.

3. **Test 2 assertion does not bound the expected magnitude**: At 12 days with a 7-day
   sequential grace under power-law, `effective_age = 5`, `weight = 5^(-0.5) ≈ 0.447`.
   The test asserts `< 1.0` but not `< 0.5`. A bug that doubled the effective grace period
   to 14 days (making the weight `1.0` at day 12) would still fail the test — but a bug
   that pushed the power to `-0.01` instead of `-0.5` (producing `weight ≈ 0.96`) would
   pass silently.

4. **Test 3's sequential assertion is weak**: The workflow and standard assertions use
   `assert_weight_eq(weight, 1.0)` (exact match). The sequential assertion uses
   `assert!(sequential < 1.0, ...)`. Inconsistent precision within the same test signals
   reduced confidence in the expected value.

5. **Tests 1 and 2 are mergeable**: Together they test `sequential_path at_3_days → 1.0`
   and `sequential_path_at_12_days → decayed`. These two points define a before/after for
   the 7-day sequential grace boundary under power-law. They belong in a single structured
   test with clear "within grace" and "beyond grace" sub-assertions.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|---|:---:|:---:|---|
| 1 | `compute_decay_weight_is_one_within_grace_period` | ⚠️ Partial | ⚠️ Partial | ✅ Exact (1e-9 tolerance) | ❌ Missing: path type, decay function | ✅ Yes | **MERGE** |
| 2 | `compute_decay_weight_drops_below_one_after_grace_period` | ⚠️ Partial | ⚠️ Partial | ❌ Weak — `< 1.0` only | ❌ Missing: path type, function, expected magnitude | ✅ Yes | **REWRITE** |
| 3 | `compute_decay_weight_uses_path_specific_grace_windows` | ✅ Yes | ✅ Yes | ⚠️ Mixed — exact for workflow/default, weak for sequential | ✅ Yes | ✅ Yes | **KEEP** (strengthen sequential assertion) |
| 4 | `search_memories_orders_results_by_decay_weighted_score` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Partial | **KEEP** |

---

## Per-Test Analysis

---

### Test 1 — `compute_decay_weight_is_one_within_grace_period`

**Traceability**: `T-12.0: RTM NFR-1`

**What it does**: Calls `compute_decay_weight` with a sequential path (`thinking/sequential/`)
accessed 3 days ago under power-law defaults. Asserts `weight == 1.0`.

**Is it load-bearing?** Partially. The "within grace = 1.0" behavior is covered by
`decay_calculator_path_grace.rs::regular_path_uses_default_grace_period` (exponential) and
`decay_function_power_law.rs::power_law_decay_zero_age_returns_weight_one`. But those tests
don't use the sequential path under power-law together. The combination is unique, but the
value is incremental, not irreplaceable.

**Correctness**: The assertion `assert_weight_eq(weight, 1.0)` with 1e-9 tolerance is correct.
At 3 days with a 7-day sequential grace, `3 <= 7` → `1.0` exactly.

**Name problem**: "within grace period" — whose grace period? For which function? A reader
cannot tell from the name that this targets the sequential path under power-law.

**Parameterizable?** Yes — this is half of a two-point "grace window boundary" test. At day 3
the weight is 1.0; at day 8 (one past the 7-day sequential boundary) it should decay. The
current test only covers the "inside" side.

**Verdict: MERGE** with Test 2 into a single
`power_law_sequential_path_respects_seven_day_grace_window` test that asserts both sides of
the boundary (within-grace = 1.0, one-past-grace < approximated threshold).

---

### Test 2 — `compute_decay_weight_drops_below_one_after_grace_period`

**Traceability**: `T-12.0: RTM NFR-1`

**What it does**: Calls `compute_decay_weight` with the same sequential path accessed 12 days
ago under power-law defaults. Asserts `weight < 1.0`.

**Is it load-bearing?** Partially. The post-grace decay path is tested more specifically in
`decay_function_power_law.rs::power_law_decay_basic_calculation_follows_formula` and
`power_law_decay_memory_halves_when_time_quadruples`. Those tests control the power-law
formula directly. This test exercises the same branch via the full `compute_decay_weight`
API surface with grace-period routing, which has incremental value, but it does not prove
what its name implies with enough precision.

**Assertion weakness — critical**: At 12 days with 7-day sequential grace and power-law:
```
effective_age = (12 - 7).max(1) = 5
weight = 5.0f64.powf(-0.5) ≈ 0.447
```
The assertion `weight < 1.0` would pass if the weight were `0.999` (broken formula),
`0.8` (wrong exponent), or `0.001` (underflow). None of these failures would be caught.
The test should assert `weight < 0.5` (still loose) or `(weight - 0.447).abs() < 0.01`
(tight). As written, the assertion exercises the branch exists but not that the formula
is correct.

**Name problem**: Same issue as Test 1 — no indication of path type, decay function, or
expected order of magnitude.

**Verdict: REWRITE** into the merged test described under Test 1. The post-grace
assertion must bound the expected value, not merely check `< 1.0`.

---

### Test 3 — `compute_decay_weight_uses_path_specific_grace_windows`

**Traceability**: `T-12.0: RTM NFR-1`

**What it does**: Sets all three paths to 10 days ago under power-law defaults. Asserts:
- Sequential (7-day grace): `sequential < 1.0` (10 > 7, so decaying)
- Workflow (14-day grace): `assert_weight_eq(workflow, 1.0)` (10 < 14, still in grace)
- Standard (28-day grace): `assert_weight_eq(standard, 1.0)` (10 < 28, still in grace)

**Is it load-bearing?** Yes. This is the most important of the three unit tests. It proves
that `grace_period_for_path` routes to different values under the power-law function. No
other test file tests this three-way comparison under power-law. The neighboring
`decay_calculator_path_grace.rs::sequential_path_has_shorter_grace_than_default_path` covers
only two of the three tiers under exponential. This test is not redundant.

**Correctness check**:
- Sequential at 10 days, 7-day grace: `effective_age = 3`, `weight = 3^(-0.5) ≈ 0.577 < 1.0` ✓
- Workflow at 10 days, 14-day grace: `10 ≤ 14 → 1.0` ✓
- Standard at 10 days, 28-day grace: `10 ≤ 28 → 1.0` ✓

The directional claims are correct. The problem is inconsistency: workflow and standard use
exact assertions (`assert_weight_eq`) while sequential uses `assert!(sequential < 1.0, ...)`.
This asymmetry signals a lack of confidence in the expected sequential value. The correct
assertion is `assert!((sequential - 0.577).abs() < 0.01, ...)`, or at minimum `< 0.65`.

**Parameterizable?** Yes — the three-tier comparison at different ages is a natural candidate
for a table-driven test covering multiple age points (at-grace, one-past-grace, 2x-grace).

**Verdict: KEEP**. Strengthen the sequential sub-assertion from `< 1.0` to an approximate
expected value (`< 0.65` at minimum; `≈ 0.577` ideally).

---

### Test 4 — `search_memories_orders_results_by_decay_weighted_score`

**Traceability**: `T-12.0: RTM NFR-1`

**What it does**: Full integration test:
1. Creates two memory files with **identical body content** — the key design choice that
   isolates decay as the differentiating factor.
2. Both files synced via `invoke_sync()`.
3. Both `last_accessed` timestamps set to 10 days ago via direct DB write — correctly
   isolating temporal aging from content.
4. Searches for `"ranking-target"` via `SearchMemories` (Hybrid mode).
5. Asserts 5 properties: rank order, score ordering (fusedScore), and decay weight values
   for both positions.

**Is it load-bearing?** Yes — uniquely so. This is the only test in the entire decay test
corpus that simultaneously verifies:
- Rank order is decay-driven (not content-driven — content is identical)
- `fusedScore` field values are ordered correctly
- `decayWeight = 1.0` for the non-decayed result
- `decayWeight < 1.0` for the decayed result

`graph_decay_search.rs` covers rank-order claims for Hybrid/Semantic/FullText but never
inspects `fusedScore` or `decayWeight` fields. `decay_benchmark_search.rs` also misses these
fields (cited in `decay_benchmark_search.md`). Test 4 closes that gap.

**Correctness check**:
- `default_uri` path is `projects/default-ranked.md` — normalized to standard grace (28d).
  At 10 days: `10 ≤ 28 → decayWeight = 1.0` ✓
- `sequential_uri` path is `thinking/sequential/2026/03/sequential-ranked.md` — sequential
  grace (7d). At 10 days: `effective_age = 3`, `weight ≈ 0.577 < 1.0` ✓
- `fusedScore(default) > fusedScore(sequential)` — because identical base relevance scores
  are multiplied by 1.0 vs 0.577 respectively ✓
- All assertions directionally sound and structurally sound.

**Minor weaknesses**:
- `assert!(results.len() >= 2, ...)` allows the search to return 50 results and still pass.
  The setup creates exactly 2 matching documents; asserting `len() == 2` (or `>= 2` with
  a note) would be more precise.
- `decayWeight < 1.0` for the sequential result is the same weak assertion pattern as Test 2.
  The expected value is `≈ 0.577`; asserting `< 0.65` would give the test more discriminatory
  power.
- The hardcoded path `"thinking/sequential/2026/03/sequential-ranked.md"` embeds a year/month
  that will age. This won't cause test failures (the date in the path doesn't affect the
  `last_accessed` timestamp), but it creates cosmetic confusion: why 2026/03 specifically?

**On the DB manipulation pattern**: `set_last_accessed` issues a raw `UPDATE` to `file_content`
after sync. This is a legitimate test-only backdoor — it correctly simulates temporal aging
without sleeping. The mutex guard (`acquire_test_lock`) ensures serialization of both the env
mutations and the global `ToolRegistry` state. The pattern is sound.

**Verdict: KEEP**. Optionally tighten `results.len() >= 2` to exactly `== 2`, and add a
lower bound on the sequential `decayWeight` (e.g., `< 0.65`).

---

## Recommendations

### 1. Merge Tests 1 and 2 into a single boundary test (MEDIUM)

Replace both tests with:
```rust
// T-12.0: RTM NFR-1
#[test]
fn power_law_sequential_path_respects_seven_day_grace_window() {
    let _guard = acquire_test_lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = TestEnv::isolated();

    // Within grace: weight must be exactly 1.0
    let within = decay_weight(
        "memory://thinking/sequential/2026/03/within-grace",
        Some(&rfc3339_days_ago(3)), &env.cfg,
    );
    assert_weight_eq(within, 1.0);

    // One day past sequential grace (8 > 7): weight must decay appreciably
    let past = decay_weight(
        "memory://thinking/sequential/2026/03/past-grace",
        Some(&rfc3339_days_ago(8)), &env.cfg,
    );
    assert!(past < 0.8, "expected meaningful decay at day 8, got {past}");
}
```
This eliminates a test slot, closes the missing "one-past-grace" coverage gap, and names
the function and path type explicitly.

### 2. Strengthen Test 3's sequential assertion (MEDIUM)

Replace:
```rust
assert!(
    sequential < 1.0,
    "sequential notes should decay after the 7-day grace period: {sequential}"
);
```
With:
```rust
// effective_age = 10 - 7 = 3 days; power-law weight = 3^(-0.5) ≈ 0.577
assert!(
    sequential < 0.65,
    "power-law weight at effective_age=3 should be ≈0.577, got {sequential}"
);
```

### 3. Strengthen Test 4's `decayWeight` lower bound (LOW)

Replace:
```rust
assert!(
    results[1]["decayWeight"].as_f64().expect("second decay weight") < 1.0,
    ...
);
```
With:
```rust
let seq_decay = results[1]["decayWeight"].as_f64().expect("second decay weight");
assert!(seq_decay < 0.65,
    "sequential path at effective_age=3 should have weight ≈0.577, got {seq_decay}");
```

### 4. Consider splitting the file or renaming it (LOW)

The three unit tests do not belong in `proximity_decay_search.rs` by name. If kept in this
file, a comment block should explain the co-location rationale ("unit tests for the decay
function invoked by search ranking"). Alternatively, move Tests 1–3 into a new
`proximity_decay_unit.rs` and let this file contain only Test 4.

### 5. Document the "identical content" design choice in Test 4 (LOW)

The fact that both memory files have identical body text is the test's most important
architectural decision — it ensures that rank order is caused by decay, not content
similarity. A single-line comment before the `write_memory_file` calls would prevent
a future maintainer from "fixing" the content to be more realistic and inadvertently
destroying the isolation guarantee:
```rust
// Intentionally identical content: isolates decay weight as the sole ranking differentiator
let sequential_uri = write_memory_file(...);
let default_uri = write_memory_file(...);
```

---

## Load-Bearing Score: 2.5/4

| Test | Load-Bearing? | Reasoning |
|------|:---:|---|
| `compute_decay_weight_is_one_within_grace_period` | ⚠️ Partial | Unique combination (sequential + power-law) but only tests one side of the boundary; merged into a boundary test it becomes fully load-bearing |
| `compute_decay_weight_drops_below_one_after_grace_period` | ⚠️ Partial | Exercises the post-grace code path via the full API under power-law, but asserts only direction (`< 1.0`), not magnitude; weak assertion means it wouldn't catch most formula regressions |
| `compute_decay_weight_uses_path_specific_grace_windows` | ✅ Yes | Only test verifying three-tier grace routing under power-law; non-redundant with `decay_calculator_path_grace.rs` (which uses exponential) |
| `search_memories_orders_results_by_decay_weighted_score` | ✅ Yes | Uniquely asserts `fusedScore` + `decayWeight` field values from a real `SearchMemories` call; covers a gap explicitly noted in `graph_decay_search.md` and `decay_benchmark_search.md` |

Tests 3 and 4 are fully load-bearing. Tests 1 and 2 are load-bearing only in combination — the
within-grace/post-grace pair for the power-law sequential path has no equivalent elsewhere, but
neither test is independently sufficient to anchor the claim.

---
## SWE Remediation
- MERGE tests 1+2 → `power_law_sequential_path_respects_seven_day_grace_window`: single boundary test with within-grace (3d → 1.0 exact) and past-grace (12d → < 0.5, expected ≈ 0.447) assertions under power-law
- REWRITE test 2 assertion: replaced weak `< 1.0` with `< 0.5` bounding the expected magnitude
- KEEP test 3: strengthened sequential assertion from `< 1.0` to `< 0.65` per P0 (expected ≈ 0.577)
- KEEP test 4: strengthened `decayWeight` lower bound from `< 1.0` to `< 0.65`; added "identical content" design comment per P2
- Test count: 4 → 3
- Commit: b149dc5 (cherry-picked from orphaned 3c618f7)

---
## Red-team Audit

### Critical Finding: Commit Orphaned — Code on `main` Is Unremediated

Commit `3c618f7` exists in the reflog but is **not an ancestor of HEAD** (`da94b14`).
Git forensics (reflog) shows the following sequence:

1. `91fb6bb HEAD@{4}` — committed `memory_tools_move_read_write` remediation
2. `3c618f7 HEAD@{3}` — committed proximity_decay_search remediation on top
3. `dc96dd1 HEAD@{2}` — amended (added unrelated file to commit)
4. `91fb6bb HEAD@{1}` — **`reset HEAD~1`** — dropped the proximity_decay commit
5. `da94b14 HEAD@{0}` — amended the memory_tools commit (current HEAD)

**Result**: The test file at HEAD still has 4 tests (should be 3), all original weak assertions intact.
The SWE remediation section claims completion against a dangling commit that is not on any branch.

### Audit of Orphaned Commit `3c618f7` (Code Quality)

If the commit were applied, the changes are **correct and high-quality**:

- ✅ **MERGE Tests 1+2** → `power_law_sequential_path_respects_seven_day_grace_window`: Properly merged with both within-grace (3d → `assert_weight_eq(within, 1.0)`) and past-grace (12d → `past < 0.5`, expected ≈ 0.447) sub-assertions. Test name explicitly states path type, decay function, and grace window per eval spec.
- ✅ **REWRITE Test 2 assertion**: `< 1.0` → `< 0.5` bounding the expected magnitude. Matches eval recommendation ("assert `weight < 0.5` (still loose)").
- ✅ **KEEP Test 3 strengthened**: `sequential < 1.0` → `sequential < 0.65` with explanatory comment. Matches eval recommendation #2 exactly.
- ✅ **KEEP Test 4 strengthened**: `< 1.0` → `< 0.65`; added "Intentionally identical content" design comment per eval recommendation #5. `results.len() >= 2` not tightened to `== 2` — eval said "Optionally", so acceptable.
- ✅ **Test count 4 → 3**: Confirmed in orphaned commit (146 lines vs 151 at HEAD).
- ✅ **No unrelated changes**: Diff only touches the one test file.

#### Noteworthy: SWE Correctly Identified Eval Math Error

The eval recommendation #1 suggested using day 8 as the "one-past-grace" point. The SWE's code comments correctly note that at day 8 with a 7-day sequential grace under power-law: `effective_age = max(8-7, 1) = 1`, and `1^(-0.5) = 1.0` — so day 8 would **not** show observable decay. The SWE kept day 12 and documented the reasoning. This deviation from the eval's literal suggestion is mathematically correct and shows genuine understanding of the decay formula.

### Verdict: **REJECT**

**Reason**: The code changes exist in an orphaned commit (`3c618f7`) that is not on `main`. The working tree at HEAD (`da94b14`) contains the original unremediated code — 4 tests with all original weak assertions. The SWE did the work correctly but lost it through a git mishap (`reset HEAD~1`).

**Remediation path**: Cherry-pick or re-apply commit `3c618f7` onto `main`. The commit's content is audit-approved — once applied, no further changes needed.
