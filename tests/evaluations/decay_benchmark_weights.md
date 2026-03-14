# Test Evaluation: decay_benchmark_weights

**File:** `crates/maenifold-core/tests/decay_benchmark_weights.rs`
**Support:** `crates/maenifold-core/tests/decay_benchmark/support.rs`
**Test count:** 3
**Evaluated:** 2025-07-21
**Traceability:** T-12.7 · RTM NFR-1
**Color:** 🔘 Gray — Skeptical Inquiry

---

## Summary

This suite is the only integration-level validation of the decay weight pipeline end-to-end.
It sits above the unit tests in `scoring.rs` (which test the math in isolation) and above the
config tests in `config_decay_weights.rs` (which test runtime tunability). Its unique
mandate is to verify that the full pipeline — file creation, sync, DB write, weight query,
and access-boost round-trip — produces the policy-level shape specified by NFR-1.

All three tests carry genuine load. They are not duplicates of each other, and they cover
territory that no unit test touches: curve shape across nine age checkpoints, the
`read_memory`→`last_accessed` DB update path, and path-based tier selection through the
full graph sync. Eliminating any one would leave a meaningful gap in coverage.

However, skeptical examination reveals four structural problems that erode confidence.
First, **five of nine created data points in Test 1 are never asserted on** — they consume
setup cost and produce no signal. Second, **directional assertions (`<`, `>`) are
trivially weak**: none test *how much* separation exists, so a nearly-flat decay curve
would pass every check. Third, **threshold values are under-tight**: `boost_ratio > 2.0`
passes when the actual ratio is ~5.65×; `stale_weight_after < 0.5` passes when the actual
value is ~0.18. These gaps mean a partial regression (curve flattening, incomplete boost)
could pass undetected. Fourth, **isolated environments do not pin the decay function via
env-var override**, meaning a shell-level `MAENIFOLD_DECAY_FUNCTION=exponential` silently
swaps the formula under test without any assertion detecting the mismatch.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Clear? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `benchmark_decay_weight_distribution_produces_meaningful_separation` | ✅ | ✅ | ⚠️ Weak | ✅ | ✅ | **REWRITE** |
| 2 | `benchmark_access_boosting_rescues_actively_used_old_content` | ✅ | ✅ | ⚠️ Under-tight | ✅ | ⚠️ | **REWRITE** |
| 3 | `benchmark_tiered_decay_episodic_decays_faster_than_semantic` | ✅ | ✅ | ⚠️ Weak | ⚠️ Partial | ✅ | **REWRITE** |

---

## Per-Test Analysis

---

### Test 1 — `benchmark_decay_weight_distribution_produces_meaningful_separation`

**What it does.** Creates nine files with ages `[1, 7, 14, 30, 60, 90, 120, 180, 365]`
days, syncs the graph, fetches each decay weight, then makes assertions on four of them
(ages 1, 30, 90, 365).

**Is it load-bearing?** Yes. No unit test exercises the decay curve across this range in
the integration pipeline. Removing it would leave the curve shape entirely unverified at
the system level.

**Is it unique?** Yes. `config_decay_weights.rs` tests configurability, `scoring.rs` tests
the math, but no other test validates the policy-level shape (`fresh ≥ 0.95`, `year < 0.2`,
suppression ratio).

**Are the assertions correct?**

With the default config (`power-law`, grace=28d), the expected weights are:

| Age (days) | Effective Age | Expected Weight | Asserted? |
|:---:|:---:|:---:|:---:|
| 1   | (grace) | 1.0000 | ✅ (≥ 0.95) |
| 7   | (grace) | 1.0000 | ❌ created, never checked |
| 14  | (grace) | 1.0000 | ❌ created, never checked |
| 30  | 2       | 0.7071 | ✅ (< fresh, > 0.3) |
| 60  | 32      | 0.1768 | ❌ created, never checked |
| 90  | 62      | 0.1270 | ✅ (< month) |
| 120 | 92      | 0.1042 | ❌ created, never checked |
| 180 | 152     | 0.0811 | ❌ created, never checked |
| 365 | 337     | 0.0545 | ✅ (< 0.2, > 0.0) |

**Problems identified:**

1. **Five of nine data points are unchecked.** Ages 7, 14, 60, 120, and 180 are created,
   synced, and queried, but their weights are never asserted. The computation cost is paid
   with no test benefit. A broken decay formula that returns 1.0 for all content ≤ 90 days
   and 0.01 for anything older would pass all current assertions.

2. **Directional assertions test direction, not magnitude.** `month_old_weight < fresh_weight`
   (0.7071 < 1.0) and `quarter_old_weight < month_old_weight` (0.1270 < 0.7071) would both
   pass if the values were 0.9999 and 0.9998 respectively — a functionally flat curve. There
   is no minimum-gap assertion ensuring *meaningful* separation at each step.

3. **`suppression_ratio > 5.0` is the suite's most meaningful assertion.** Actual value is
   ~18.3×, so the threshold is very conservative. A ratio of 4× would indicate a seriously
   degraded curve but still fail to trigger.

4. **Magic numbers lack documentation.** The constants 0.95, 0.3, 0.2, 5.0 are not
   explained. A reader cannot determine whether they derive from the C# oracle, from domain
   policy, or from reverse-engineering the current implementation.

5. **Decay function is not pinned.** `isolated_env()` does not set
   `MAENIFOLD_DECAY_FUNCTION`. A developer's shell env (`MAENIFOLD_DECAY_FUNCTION=exponential`)
   silently changes the formula under test. All assertions still pass under both functions,
   masking the fact that a different formula is being exercised.

**Is the name clear?** Yes.

**Is it parameterizable?** Yes. The age checkpoints and assertion thresholds are all
hardcoded but could be expressed as a data table or parameterized test.

**Verdict: REWRITE**

Suggested changes:
- Assert all nine age checkpoints against computed expected values derived from the formula.
- Replace bare directional checks with minimum-gap assertions:
  `assert!(fresh_weight - month_old_weight > 0.15, ...)`.
- Pin `MAENIFOLD_DECAY_FUNCTION` to `"power-law"` explicitly in `isolated_env()` or at
  test head, document the policy rationale.
- Document the threshold constants with inline comments referencing NFR-1 or the C# oracle.

---

### Test 2 — `benchmark_access_boosting_rescues_actively_used_old_content`

**What it does.** Creates two 60-day-old files, verifies they have equal initial weights,
calls `read_memory_markdown` on one, then asserts the read file has near-full weight and
the unread file remains decayed, with a >2× ratio between them.

**Is it load-bearing?** Yes. This is the **only test** that verifies the full
`read_memory` → `last_accessed` DB update → decay recalculation round-trip. Unit tests in
`scoring.rs` test the math given a recent timestamp, but nothing else confirms that
`read_memory` actually writes `last_accessed` to the database. Removing this test would
make that entire subsystem invisible to the test suite.

**Is it unique?** Yes. `access_boosting_reads.rs` exercises boost via search rankings;
this test exercises boost via direct weight comparison — a complementary but distinct check.

**Are the assertions correct?**

With `power-law`, grace=28d, a 60-day-old file has:
- effective_age = 32, weight = 32^(−0.5) ≈ **0.1768**

After reading, `last_accessed` = now → within any grace period → weight = **1.0**.

| Assertion | Actual Values | Threshold | Slack |
|-----------|:---:|:---:|:---:|
| `active_weight_before ≈ stale_weight_before (±0.01)` | 0.1768 ≈ 0.1768 | ±0.01 | adequate |
| `active_weight_after >= 0.95` | 1.0 | 0.95 | 0.05 |
| `stale_weight_after < 0.5` | 0.1768 | 0.50 | **0.323 — too lenient** |
| `boost_ratio > 2.0` | ~5.65× | 2.0× | **3.65× — too lenient** |

**Problems identified:**

1. **`stale_weight_after < 0.5` is 2.8× too lenient.** The actual value is ~0.18. A
   broken access boost that inflates the stale weight to 0.4 (maybe from a cache
   invalidation bug) would still pass this assertion. A threshold of `< 0.25` or even
   `< 0.22` would catch realistic regressions.

2. **`boost_ratio > 2.0` is dangerously conservative.** At 60 days, a correct
   implementation produces ~5.65×. A threshold of `> 4.0` would still give margin while
   catching partial-boost regressions (e.g., boost applies but doesn't fully reset to
   `last_accessed = now`).

3. **The pre-condition guard `assert_approx_eq(before, before, 0.01)` uses 0.01
   tolerance.** At weight 0.1768, a delta of 0.01 represents a 5.7% relative difference.
   If one file's created timestamp is off by a few seconds due to `rfc3339_days_ago` race
   conditions, the test could flap. Zero tolerance (both files created atomically in the
   same second) would be more reliable; if that's not guaranteed, the comment should
   explain the tolerance rationale.

4. **Hardcoded 60-day age is undocumented.** 60 days was likely chosen to be clearly
   decayed (past the 28-day grace) while leaving room for the boost to be dramatic. This
   reasoning should be in a comment.

5. **The decay function is not pinned** — same issue as Test 1.

**Is the name clear?** Yes.

**Is it parameterizable?** Partially. The age (60 days) could be parameterized;
the threshold values should be derived from the formula rather than hardcoded.

**Verdict: REWRITE**

Suggested changes:
- Tighten `stale_weight_after < 0.25` to reflect the actual math.
- Tighten `boost_ratio > 4.0` to catch partial-boost regressions.
- Derive the expected pre-boost weight programmatically or document the formula.
- Pin the decay function.

---

### Test 3 — `benchmark_tiered_decay_episodic_decays_faster_than_semantic`

**What it does.** Creates one file in `thinking/sequential/` (episodic) and one plain
knowledge file (semantic), both 10 days old. Asserts the episodic weight is lower, the
semantic weight is effectively 1.0, and episodic is strictly below 1.0.

**Is it load-bearing?** Yes. The `grace_period_for_path` path-matching logic in
`scoring.rs` has no dedicated unit tests. This is the only test that exercises the tiered
grace-period dispatch end-to-end through the full sync pipeline.

**Is it unique?** Yes. No other test in the suite verifies that path-based tier selection
produces different weights for content at the same calendar age.

**Are the assertions correct?**

With `power-law`, sequential grace=7d, default grace=28d, age=10d:

| Content Type | Grace Period | Effective Age | Expected Weight |
|:---:|:---:|:---:|:---:|
| Episodic (`thinking/sequential/`) | 7 days | 10−7 = 3 | 3^(−0.5) ≈ **0.5774** |
| Semantic (knowledge doc) | 28 days | (within grace) | **1.0** |

| Assertion | Actual | Pass? |
|-----------|:---:|:---:|
| `episodic_weight < semantic_weight` | 0.5774 < 1.0 | ✅ |
| `assert_approx_eq(semantic_weight, 1.0, 0.001)` | \|1.0−1.0\| = 0.0 | ✅ |
| `episodic_weight < 1.0` | 0.5774 < 1.0 | ✅ |

**Problems identified:**

1. **No minimum-separation assertion.** The test would pass if episodic were 0.9999 and
   semantic 1.0 — a negligible difference. Adding `assert!(semantic_weight - episodic_weight > 0.3, ...)`
   ensures the tier separation is functionally meaningful, not just directional.

2. **The magic age of 10 days is fragile and undocumented.** Its correctness depends on
   `grace_days_sequential < 10 < grace_days_default` (i.e., 7 < 10 < 28). This invariant
   is never asserted. If a config refactor changes the sequential grace to 12 days, this
   test silently passes with both files in-grace (weight=1.0 each), violating the assertion
   `episodic_weight < 1.0`. The test should read the config and assert the age falls in
   the correct range:
   ```rust
   assert!(test_age_days > env.cfg.decay_grace_days_sequential(), "age must exceed episodic grace");
   assert!(test_age_days < env.cfg.decay_grace_days_default(),    "age must be within semantic grace");
   ```

3. **Only two of three tiers are tested.** The `grace_period_for_path` function
   distinguishes three tiers: sequential, workflow, and default. The workflow tier
   (`thinking/workflow/`, 14-day grace) is not exercised at all. A regression that swaps
   the workflow and sequential grace periods would be invisible.

4. **Name implies a complete tiered test but only covers two tiers.** Rename to
   `benchmark_episodic_decays_faster_than_semantic` and add a companion test
   `benchmark_workflow_decays_between_episodic_and_semantic`.

5. **`assert_approx_eq(semantic_weight, 1.0, 0.001)` tolerance is unnecessarily tight.**
   At 10 days with a 28-day grace, the weight is exactly 1.0. The tolerance of 0.001 adds
   no margin for legitimate floating-point variation and could flap on a different platform
   or compiler. However, since the result is always exactly 1.0 under the grace period,
   this is a minor concern rather than a real risk.

6. **Decay function not pinned** — same issue as Tests 1 and 2.

**Is the name clear?** Partially — it implies full tiered coverage but only tests two
of three tiers. See item 4 above.

**Is it parameterizable?** Yes. Age and tier paths are obvious parameterization axes.

**Verdict: REWRITE**

Suggested changes:
- Assert `env.cfg.decay_grace_days_sequential() < test_age_days < env.cfg.decay_grace_days_default()` as a pre-condition.
- Add minimum-separation assertion: `semantic_weight - episodic_weight > 0.3`.
- Add a third sub-case (or sibling test) for the workflow tier.
- Rename to remove the implication of complete tiered coverage.

---

## Cross-Cutting Findings

### ENV-VAR LEAKAGE — `MAENIFOLD_DECAY_FUNCTION` is never pinned

`isolated_env()` sets `Config::override_root` and `Config::set_database_path`, but
**does not override** `MAENIFOLD_DECAY_FUNCTION`, `MAENIFOLD_DECAY_HALF_LIFE_DAYS`, or
`MAENIFOLD_DECAY_GRACE_DAYS_*`. All three tests inherit the calling shell's environment.
If a developer has `MAENIFOLD_DECAY_FUNCTION=exponential` set (e.g., from a local
debugging session), every test silently exercises the wrong formula. All assertions still
pass under exponential because the policy-level thresholds (≥0.95, <0.2, etc.) hold for
both functions. The tests would report green while exercising unintended behavior.

**Recommended fix:** Add to `isolated_env()` or to each test's setup block:
```rust
// Pin decay config to a known baseline so tests are hermetic
std::env::set_var("MAENIFOLD_DECAY_FUNCTION", "power-law");
// ... plus reset in TestEnv::drop via Config::reset_overrides
```
Or, better, extend `Config::override_root` to reset all decay-related env vars to defaults.

### GLOBAL MUTEX SERIALIZATION

All three tests acquire `TEST_MUTEX`. This forces serial execution and may significantly
slow CI when this suite runs alongside other mutex-guarded tests. The mutex exists because
`Config::override_root` writes global state — a smell in the production code, not just
the tests. A future refactor to pass `Config` by value through the call graph would
eliminate this bottleneck.

### MAGIC NUMBERS WITHOUT DOCUMENTATION

Threshold values (0.95, 0.3, 0.2, 5.0, 2.0, 0.01, 0.001) appear without comments
explaining their origin. Each should be annotated with one of:
- `// C# oracle: [value]` — derived from parity target
- `// NFR-1 policy minimum` — defined by product requirement
- `// conservative bound: actual ≈ [value]` — chosen to give debugging room

Without this, future maintainers cannot distinguish a policy requirement from an
accidentally coincidental threshold.

---

## Recommendations

Prioritized by impact:

1. **[High] Pin `MAENIFOLD_DECAY_FUNCTION` in test setup.** Removes silent env-var
   leakage. One line in `isolated_env()`.

2. **[High] Add minimum-separation gap assertions in Tests 1 and 3.** Replaces weak
   directional checks with meaningful signal. Prevents flat-curve regressions.

3. **[High] Tighten thresholds in Test 2.** Change `< 0.5` → `< 0.25` and `> 2.0` →
   `> 4.0` to catch partial-boost regressions.

4. **[Medium] Assert all nine data points in Test 1.** Either assert on all nine or
   remove the five that are created-but-unchecked. Currently paying sync and DB cost for
   zero test benefit on five of nine checkpoints.

5. **[Medium] Add config pre-condition guard in Test 3.** Assert
   `grace_sequential < age < grace_default` before running the decay comparison.

6. **[Medium] Add workflow tier to Test 3** (or a sibling test). The third tier of
   `grace_period_for_path` is entirely untested.

7. **[Low] Document all magic number thresholds** with inline comments.

8. **[Low] Rename Test 3** to remove implication of complete tiered coverage.

---

## Load-Bearing Score: 3/3

All three tests are load-bearing — each covers a unique behavioral contract not verified
elsewhere in the suite. However, all three carry correctness deficiencies that reduce their
actual protective value. The suite is **structurally complete** but **assertively weak**.
It will catch catastrophic failures (no decay, wrong sign, zero output) but will miss
moderate regressions (curve flattening, partial boost, tier mis-dispatch). Rewrites are
warranted before this suite can be considered production-grade confidence coverage for
[[NFR-1]] [[decay]] in [[maenifold-rs]].
