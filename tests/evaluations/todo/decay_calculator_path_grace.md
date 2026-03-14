# Test Evaluation: decay_calculator_path_grace

**File**: `crates/maenifold-core/tests/decay_calculator_path_grace.rs`
**Traceability**: T-7.5.1, T-7.5.2 (RTM NFR-7.5.1, NFR-7.5.2, NFR-1)
**Evaluator**: Red Team — Gray (Skeptical Inquiry)
**Sister file examined**: `crates/maenifold-core/tests/decay_function_grace_periods.rs`

---

## Summary

This file contains 5 tests covering **path-based grace period routing** in `compute_decay_weight`:
how `grace_period_for_path()` selects between sequential (7d), workflow (14d), and default (28d)
grace periods based on the normalized file path.

The suite has two unique tests worth preserving, two that are **functionally redundant** with the
stronger sister file `decay_function_grace_periods.rs`, and one that partially duplicates the
tiered-comparison test there. A structural weakness runs through the entire suite: **all five tests
assert `weight == 1.0` at exactly the grace boundary** (i.e., `days_ago == grace_period_days`).
This tests the `≤` branch of `if age_days <= grace_period_days { return 1.0; }`. No test in this
file verifies that the weight actually drops below 1.0 on day `grace + 1` for the routed path —
making the path-routing assertions structurally incomplete.

Additionally, **tests 2 and 3** carry a silent false-confidence risk: a path incorrectly routed to
a *longer* grace category would still return `weight == 1.0` at its respective boundary day,
causing the test to pass despite the routing being wrong.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertion Correctness | Name Quality | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|---|:---:|:---:|---|
| 1 | `empty_path_uses_default_grace_period` | ✅ | ✅ | ⚠️ Weak — boundary only, cannot detect misrouting to a *longer* grace | ✅ | Yes (w/ test 4) | **REWRITE** |
| 2 | `windows_backslash_sequential_path_uses_sequential_grace_period` | ✅ | ✅ | ❌ Structurally unsound — at-grace assertion cannot distinguish sequential (7d) from default (28d) routing | ✅ | No | **REWRITE** |
| 3 | `thinking_workflow_path_uses_workflow_grace_period` | ❌ | ❌ | ⚠️ Weak + redundant — same logic as `exponential_decay_workflows_uses_14_day_grace_period` in sister file, which also asserts post-grace < 1.0 | ✅ | Yes (w/ test 4) | **REMOVE** |
| 4 | `regular_path_uses_default_grace_period` | ❌ | ❌ | ⚠️ Weak + redundant — same path class as `exponential_decay_default_memory_uses_14_day_grace_period` in sister file | ✅ | Yes (w/ test 1) | **REMOVE** |
| 5 | `sequential_path_has_shorter_grace_than_default_path` | ⚠️ Partial | ⚠️ Partial | ✅ Strongest assertion in the file — relative comparison, not just boundary | ✅ | Partial | **MERGE** |

---

## Per-Test Analysis

### Test 1 — `empty_path_uses_default_grace_period`

**Claim**: An empty path string `""` routes to the default grace period (28 days).

**Code path**: `normalize_decay_path("", ...)` → `""` (no backslash replacement, no `memory://`
prefix substitution). `grace_period_for_path("")` → no substring matches → returns
`config.decay_grace_days_default()` (28). `28 <= 28` → `1.0`. ✓

**Unique**: Yes — the only test exercising the empty-string edge case in `normalize_decay_path`.
The empty string bypasses both the backslash normalization _effect_ and the `memory://` prefix
branch. This code path is distinct from test 4.

**Assertion gap**: `assert_approx_eq(weight, 1.0, 1e-9)` only verifies the `≤` (at-boundary)
branch. If the empty path were misrouted to the sequential grace (7d), the test would _fail_
correctly, since `28 > 7`. But if it were misrouted to a hypothetical 30d grace period, the test
would still pass. No assertion checks `weight < 1.0` at day 29.

**Verdict**: **REWRITE** — Add a second call with `days_ago = Some(29)` and assert `weight < 1.0`
to close the post-grace hole. Optionally merge with test 4 via parameterization.

---

### Test 2 — `windows_backslash_sequential_path_uses_sequential_grace_period`

**Claim**: A Windows-style double-backslash path containing `thinking/sequential` routes to the
sequential grace period (7 days).

**Code path**: The raw string `r"C:\\Users\\user\\maenifold\\memory\\thinking\\sequential\\session.md"`
contains literal double-backslashes. `normalize_decay_path` calls `.replace('\\', "/")`, converting
each `\` to `/`, producing `c://users//user//maenifold//memory//thinking//sequential//session.md`.
This contains `/sequential/`, so `grace_period_for_path` returns `config.decay_grace_days_sequential()`
(7). `7 <= 7` → `1.0`. ✓

**Unique**: Yes — this is the **only** test in the entire decay suite that exercises Windows-style
path normalization. It guards a real cross-platform concern.

**Structural flaw**: `assert_approx_eq(weight, 1.0, 1e-9)` with `days_ago=Some(7)`. At day 7 with
a 7-day grace, the result is 1.0. _But_ if this path were incorrectly routed to the default grace
(28 days), the result at day 7 would _also_ be 1.0 (since 7 < 28). **The test cannot distinguish
correct routing to sequential grace from incorrect routing to any longer grace.** It would pass
even if `normalize_decay_path` silently failed to normalize backslashes at all and the path fell
through to the default bucket.

**Fix**: Add `days_ago = Some(8)` and assert `weight < 1.0`. This would catch misrouting to the
default grace (where day 8 < 28, still 1.0), so a better fix is: `days_ago = Some(29)` and assert
`weight < 1.0`. At day 29 with sequential grace (7d), weight decays significantly. At day 29 with
default grace (28d), weight is also decaying (just started). The _decisive_ test: assert that at
day 8, weight < 1.0 (only true if sequential grace is active).

**Verdict**: **REWRITE** — the test is load-bearing and unique, but structurally unsound. Must add
post-grace assertion at day 8 (one past sequential grace) to make the routing claim provable.

---

### Test 3 — `thinking_workflow_path_uses_workflow_grace_period`

**Claim**: `memory://thinking/workflow/...` routes to the workflow grace period (14 days).

**Code path**: After `memory://` prefix substitution with memory root, the path contains
`/thinking/workflow/`, matching the second branch in `grace_period_for_path`. Returns
`decay_grace_days_workflows()` (14). `14 <= 14` → `1.0`. ✓

**Not unique**: `decay_function_grace_periods.rs::exponential_decay_workflows_uses_14_day_grace_period`
tests the same path (`memory://thinking/workflow/2026/03/at-grace`) at the same boundary, **and
additionally** asserts `past_grace < 1.0` at day 15. The sister test is strictly superior.

**Same structural flaw as test 2**: at day 14, result = 1.0 regardless of whether the grace is
14 or 28. A misrouted path to default grace would pass this test silently.

**Verdict**: **REMOVE** — fully superseded by the stronger sister test. Keeping both creates
confusion about which file is authoritative for workflow grace routing.

---

### Test 4 — `regular_path_uses_default_grace_period`

**Claim**: `memory://notes/my-note` (a non-thinking path) routes to the default grace period.

**Code path**: Normalized path does not match `/sequential/`, `/compaction/`, `/thinking/workflow/`,
etc. Returns `decay_grace_days_default()` (28). `28 <= 28` → `1.0`. ✓

**Not unique**: Functionally overlaps with `decay_function_grace_periods.rs::exponential_decay_default_memory_uses_14_day_grace_period`
which also tests a plain `memory://` path at the default grace boundary, _and_ asserts decay at
`grace + 1`.

**Also near-duplicate of test 1**: Both test the default grace period via different path forms
(empty string vs `memory://notes/...`). The only distinguishing value is the different
normalization code path in `normalize_decay_path` (the `memory://` prefix substitution branch vs
the plain-text branch). That difference is worth a test, but it belongs in `test 1`'s rewrite,
not as a separate test.

**Verdict**: **REMOVE** — redundant. If test 1 is rewritten to include a post-grace assertion,
the coverage for default grace routing is already complete.

---

### Test 5 — `sequential_path_has_shorter_grace_than_default_path`

**Claim**: At day 10, a sequential path decays faster than a regular path.

**Code path**: Sequential grace = 7, default grace = 28. At age 10:
- Sequential: `10 > 7` → exponent = `(10 - 7) / 30` = 0.1 → `0.5^0.1 ≈ 0.933 < 1.0` ✓
- Regular: `10 <= 28` → `1.0` ✓

**What it proves**: The relative ordering between path categories at a single point in time.
This is the **only test in this file** that makes a _comparative_ claim and cannot pass from
boundary coincidence alone. It would correctly fail if grace periods were swapped.

**Partial overlap**: `decay_function_grace_periods.rs::exponential_decay_tiered_grace_periods_sequential_decays_faster`
covers the same scenario at day 10 for all three tiers: `sequential < workflow`, `sequential < default`,
`workflow ≈ default`. That test is more complete. However, test 5 here uses `weight_for()` (which
wraps `compute_decay_weight`) while the sister test uses `compute_with_last_accessed()`. These are
different entry points, so test 5 has minor unique integration value.

**Secondary assertion**: `assert_approx_eq(regular, 1.0, 1e-9)` is redundant — it restates what
`regular_path_uses_default_grace_period` (test 4) already claims, and is obvious from the math.

**Verdict**: **MERGE** — absorb the Windows-backslash uniqueness into a new "path routing sanity"
test and fold the comparative claim into the rewrite of test 2. Or keep as-is if the `weight_for`
entry-point distinction is considered worth preserving, but remove the redundant `regular == 1.0`
sub-assertion.

---

## Recommendations

### 1. Address the universal boundary-only assertion pattern (HIGH)

Every test passes `days_ago == grace_period_days` (the exact boundary) and asserts `weight == 1.0`.
This tests the `≤` equality case but proves nothing about post-grace decay for the routed path.

For **tests 1 and 2** (the keepers), add a second `weight_for` call with
`days_ago = Some(grace_period + 1)` and assert `weight < 1.0`. This makes the grace routing
claim structurally complete.

### 2. Fix the false-confidence structural hole in test 2 (HIGH)

`windows_backslash_sequential_path_uses_sequential_grace_period` cannot detect misrouting to a
_longer_ grace period. The decisive assertion is:

```rust
// At day 8 (one past sequential grace), weight MUST be < 1.0
let post_grace = weight_for(&env.cfg, windows_path, Some(8), None);
assert!(post_grace < 1.0, "expected decay at day 8 past sequential grace, got {post_grace}");
```

### 3. Remove tests 3 and 4 (MEDIUM)

`thinking_workflow_path_uses_workflow_grace_period` and `regular_path_uses_default_grace_period`
are both covered — more completely — by `decay_function_grace_periods.rs`. Keeping them adds
maintenance surface without adding signal. Remove or redirect their `#[test]` annotations to
point reviewers to the authoritative sister file.

### 4. Consider merging tests 1 and 4 into a parameterized helper (LOW)

If test 4 is kept (it shouldn't be — see above), the common pattern of
"path X at day `grace` returns 1.0" across tests 1, 3, and 4 is a clear parameterization
candidate:

```rust
// Example: table-driven
for (path, days_within_grace) in [
    ("", 28),
    ("memory://notes/my-note", 28),
    ("memory://thinking/workflow/2026/03/test", 14),
] {
    assert_approx_eq(weight_for(&env.cfg, path, Some(days_within_grace), None), 1.0, 1e-9);
}
```

### 5. Verify that test 2's Windows path normalization is the intended behavior (INFORMATIONAL)

The path `r"C:\\Users\\..."` contains **literal double-backslashes** (four backslash chars per
segment in the raw string). After `replace('\\', "/")`, every `\` becomes `/`, resulting in double
slashes: `c://users//...//thinking//sequential//session.md`. The substring match `/sequential/`
still succeeds due to `sequential` being surrounded by `/`. This works, but the double-slash
normalization is not explicitly documented. If the intent is "Windows path with single backslash
separators" (i.e., `C:\Users\...`), use `r"C:\Users\user\maenifold\memory\thinking\sequential\session.md"`
instead of `r"C:\\..."` which has double backslashes.

---

## Load-Bearing Score: 3/5

| Test | Load-Bearing? | Reasoning |
|------|:---:|---|
| `empty_path_uses_default_grace_period` | ✅ | Only test for empty-string normalization edge case |
| `windows_backslash_sequential_path_uses_sequential_grace_period` | ✅ | Only test for Windows backslash path normalization in the entire decay suite |
| `thinking_workflow_path_uses_workflow_grace_period` | ❌ | Superseded by stronger test in `decay_function_grace_periods.rs` |
| `regular_path_uses_default_grace_period` | ❌ | Superseded by stronger test in `decay_function_grace_periods.rs` |
| `sequential_path_has_shorter_grace_than_default_path` | ⚠️ Partial | Partially covered by sister file; minor unique value via `weight_for` entry point |

**3 of 5 tests contribute unique, irreplaceable signal.** The two removals (tests 3 and 4) are
safe — no coverage is lost, only redundancy is eliminated. The two rewrites (tests 1 and 2) are
required to make the remaining tests structurally sound.

---
## SWE Remediation
- REWRITE `empty_path_uses_default_grace_period`: added post-grace assertion at day 29 (`weight < 1.0`) to close boundary-only gap (P0)
- REWRITE `windows_backslash_sequential_path_uses_sequential_grace_period`: added decisive day 8 post-grace assertion proving sequential routing; fixed Windows path from double-backslash to single-backslash per recommendation 5; merged comparative claim from test 5 (P0)
- REMOVE `thinking_workflow_path_uses_workflow_grace_period`: confirmed superseded by `decay_function_grace_periods::exponential_decay_workflows_uses_14_day_grace_period`
- REMOVE `regular_path_uses_default_grace_period`: confirmed superseded by `decay_function_grace_periods::exponential_decay_default_memory_uses_14_day_grace_period`
- MERGE `sequential_path_has_shorter_grace_than_default_path`: comparative assertion folded into test 2 using Windows backslash path; redundant `regular == 1.0` sub-assertion dropped
- Tests: 2 passed, 0 failed (was 5 tests, now 2 structurally sound tests)
- Commit: 5f52699
