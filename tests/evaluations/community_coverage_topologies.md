# Test Evaluation: community_coverage_topologies

**Traceability**: T-12.7 (community detection implementation) · T-W1-8 (NFR-2 test coverage)
**Requirements**: NFR-2 (test coverage at all 3 layers) · NFR-1 (C# oracle parity)
**File**: `crates/maenifold-core/tests/community_coverage_topologies.rs`
**Tests evaluated**: 2
**Evaluator color**: 🔘 Gray (skeptical inquiry)

---

## Summary

This file is the smallest community test module in the suite — two tests covering graph
topology scenarios. Both tests pass and run fast (0.03 s combined). Both tests cover
behavioral territory not exercised elsewhere in the community suite, making them
genuinely load-bearing despite their small count.

However, the skeptical lens reveals problems:

1. **Test 1's name is a C# artifact.** `_improved_flag_set` names an internal C# flag
   (`IsImproved`) that does not exist in the Rust implementation. The test body makes
   zero assertions about any flag. The name misleads every future reader.

2. **Test 1's modularity assertion is epistemically weak.** `modularity.is_finite()`
   accepts `0.0`, negative values, and values close to but wrong. For two well-separated
   dense cliques with only a weak inter-community bridge, modularity should be strongly
   positive. The test cannot distinguish a correct partition from one that silently
   produces the wrong modularity score.

3. **Both tests omit the `detect_and_persist` return value assertion.** The return value
   carries the community count; neither test captures it. Every sibling test that
   exercises more than trivial graphs captures and asserts on this value.

4. **Test 2 is largely superseded on behavior** — three tests in `community_coverage_basics.rs`
   already verify isolated concepts get distinct communities (2-node, 3-node, 5-node).
   Test 2's unique contribution is its **exact sorted-list equality** `[0, 1, 2, 3]`,
   which is the only precise contiguous-ID normalization guard in the entire community
   suite.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Good? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `run_louvain_two_cliques_weak_bridge_improved_flag_set` | ✅ Yes | ✅ Yes | ⚠️ Partial | ❌ No | ✅ Yes | **REWRITE** |
| 2 | `run_louvain_isolated_concepts_normalized_contiguous_ids` | ✅ Yes | ⚠️ Partial | ⚠️ Partial | ✅ Yes | ✅ Yes | **KEEP** |

---

## Test-by-Test Analysis

---

### Test 1 — `run_louvain_two_cliques_weak_bridge_improved_flag_set`

**What it exercises**: Two symmetric 3-cliques (strong internal edges, weight=10) joined by
a single weak inter-community bridge (weight=1). Asserts all 6 nodes are persisted, each
clique's 3 members share a community, and the two cliques land in different communities.
Also asserts modularity is finite.

**Is it load-bearing?**
Yes. `community_louvain_basics.rs::run_louvain_disconnected_components_separate_communities`
is the closest sibling — but that test uses *completely disconnected* components (zero
bridge). The weak bridge is the harder Louvain case: the algorithm must resist a modularity
gain from merging two internally dense clusters when the inter-cluster edge is present but
sparse. This specific topology is not covered anywhere else.

**Is it unique?**
Yes. No other community test uses a *connected* graph whose components have significantly
different intra-vs-inter edge weights. The bridge weight ratio (10:1) is the key variable
being implicitly exercised.

**Are the assertions correct?**

| Assertion | Assessment |
|-----------|-----------|
| `community_row_count == 6` | ✅ Sound guard — confirms all nodes were persisted |
| `clq1x == clq1y, clq1y == clq1z` | ✅ Correct — verifies clique cohesion by transitivity |
| `clq2x == clq2y, clq2y == clq2z` | ✅ Correct — verifies clique cohesion by transitivity |
| `clq1x != clq2x` | ✅ Correct — verifies inter-clique separation |
| `modularity.is_finite()` | ❌ Weak — accepts 0.0, negative values, subtly wrong values |
| return value of `detect_and_persist` | ❌ Missing — should assert `count == 2` |

**The `_improved_flag_set` name problem**: The C# test `RunLouvain_TwoCliquesWeakBridge_ImprovedFlagSet`
was testing that the internal Louvain iteration loop set `IsImproved = true` at least once —
an internal state machine check. The Rust implementation does not expose this flag; the
concept does not translate. The name is pure dead weight from the oracle. It tells the
reader they should expect a flag assertion, and then delivers none. A reader diagnosing a
failure would be led astray.

**Missing assertions**:
- `let count = detect_and_persist(...); assert_eq!(count, 2, "weak bridge topology → 2 communities");`
- `assert!(modularity > 0.0, "well-separated cliques must produce positive modularity, got {modularity}");`

**Parameterizable?** Yes. The 10:1 weight ratio is the whole point of this test — it is the
only test of bridge-strength thresholds in the suite. A `#[rstest]` table at ratios like
(5,1), (10,1), (50,1) would probe whether the algorithm holds at lower contrast. At minimum,
the weight values should be named constants (e.g., `CLIQUE_WEIGHT = 10`, `BRIDGE_WEIGHT = 1`)
so the contrast ratio is self-documenting.

**Verdict: REWRITE** — the topology is worth keeping (unique and load-bearing), but the
name must be corrected to remove the C# artifact, the return value must be asserted, and
the modularity check must be strengthened to `> 0.0`.

---

### Test 2 — `run_louvain_isolated_concepts_normalized_contiguous_ids`

**What it exercises**: Four isolated concepts (no edges) are inserted, Louvain runs, and
the resulting community IDs are verified to be exactly `[0, 1, 2, 3]` — contiguous,
0-based, covering all slots.

**Is it load-bearing?**
Yes, but narrowly. The behavioral property (isolated concepts each get their own community)
is already covered by three tests in `community_coverage_basics.rs`. This test's unique
load-bearing contribution is the **exact sorted-list equality assertion**, which is the only
test in the entire community suite that verifies the ID normalization invariant with
precision. The `community_coverage_basics.rs` variant for 3-node isolation only checks
`max_id < 3` — a weaker bound that would pass even if IDs were `[0, 0, 2]` (as long as
max < 3 and count = 3, which isn't even fully consistent). The exact equality `ids == vec![0, 1, 2, 3]`
is a stronger guard on the normalization step.

**Is it unique?**
Partially. The ID normalization guard is unique. The behavioral guarantee (isolation → own community) is not.

**Are the assertions correct?**

| Assertion | Assessment |
|-----------|-----------|
| `ids == vec![0, 1, 2, 3]` | ✅ Strong — checks 0-based start, contiguity, and exact count |
| Row count (implicit via vec equality) | ✅ Implicitly verified (4 rows ↔ 4 elements) |
| return value of `detect_and_persist` | ❌ Missing — should assert `count == 4` |
| `modularity == 0.0` | ❌ Missing — no edges means modularity must be 0.0; this is asserted in sibling tests for similar topologies and is absent here |
| `ORDER BY community_id` + `sort_unstable()` | ⚠️ Minor redundancy — sorting after an `ORDER BY` is harmless but noise |

**Naming**: `normalized_contiguous_ids` is accurate and unique within the test suite. The
name precisely communicates what property is being evaluated, which is more than can be said
for Test 1.

**Parameterizable?** Yes. The test is trivially generalizable: `fn test_contiguous_ids(n: usize)`
that inserts `n` isolated concepts and asserts `ids == (0..n as i64).collect::<Vec<_>>()`.
Given that `community_coverage_basics.rs` already has 2-, 3-, and 5-node isolation variants,
this could be merged into a single parameterized test that also checks contiguity.

**Verdict: KEEP** — the exact ID normalization assertion is the only precise guard of its
kind in the suite. Add the `count == 4` return value assertion and `modularity == 0.0`.

---

## Recommendations

Listed in priority order.

### P1 — Rename Test 1 (misleading C# artifact)

```rust
// BEFORE:
fn run_louvain_two_cliques_weak_bridge_improved_flag_set()

// AFTER:
fn run_louvain_two_cliques_weak_bridge_resists_merge()
```

The current name implies a flag is being tested. None is. Rename to describe the actual
behavioral invariant under test: that Louvain resists merging two internally dense cliques
when connected only by a weak bridge.

### P2 — Add `detect_and_persist` return value assertions (both tests)

Both tests omit a check on the return value of `detect_and_persist`. This value is the
community count — a first-class output of the function. Every other community test that
exercises multi-community graphs captures and asserts this value. Adding it costs one line
and catches a family of regressions (e.g., over-merging or over-splitting) before the
per-node assertions do.

```rust
// Test 1:
let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
assert_eq!(count, 2, "two-clique weak-bridge topology must produce exactly 2 communities");

// Test 2:
let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
assert_eq!(count, 4, "four isolated concepts must each receive their own community");
```

### P3 — Strengthen the modularity assertion in Test 1

```rust
// BEFORE:
assert!(modularity.is_finite(), "...");

// AFTER:
assert!(
    modularity > 0.0,
    "two well-separated dense cliques must produce positive modularity, got {modularity}"
);
```

`is_finite()` is a regression-era guard (see T-15.9 comment: "Gain formula fixed"). The
formula is no longer broken. A partition of two symmetric 3-cliques with a 10:1 weight
contrast should yield modularity well above 0. Accepting 0.0 as a valid result silently
passes a degenerate partition.

### P4 — Add `modularity == 0.0` to Test 2

```rust
assert_eq!(get_modularity(&conn), 0.0, "no edges → modularity must be 0.0");
```

All comparable tests in `community_coverage_basics.rs` include this assertion. Its absence
here is an inconsistency.

### P5 — Consider parameterizing or merging the isolation tests across files

`community_coverage_basics.rs` has three isolated-concept tests (2-node, 3-node, 5-node),
none of which check contiguous IDs. Test 2 here checks contiguous IDs for 4 nodes without
checking any of the behavioral properties the basics tests check.

Option A (preferred): Make Test 2 the canonical isolation test by adding the missing
behavioral assertions and parameterizing with `#[rstest]` over `n ∈ [1, 2, 3, 4, 5]`,
collapsing the redundant basics variants.

Option B (minimal): Keep Test 2 as-is but add the missing `count` and `modularity`
assertions so it is self-contained.

### P6 — Remove redundant `ORDER BY` + `sort_unstable()` in Test 2

The SQL already returns rows in ascending `community_id` order. The `sort_unstable()` call
is harmless but misleads the reader into thinking the order from the DB is unpredictable.
Either remove the `ORDER BY` from the SQL (and rely on the sort) or remove the
`sort_unstable()` call (and rely on the `ORDER BY`). Prefer the latter: it is cheaper
and the intent is clearer.

---

## Load-Bearing Score: 2/2

Both tests are load-bearing. Neither can be removed without creating a gap in the
community suite's topology coverage. However, both tests have assertion deficiencies
that reduce their detection power for regressions in areas they nominally cover.

| Test | Load-Bearing | Reason |
|------|:---:|--------|
| `run_louvain_two_cliques_weak_bridge_improved_flag_set` | ✅ | Only test of weak-bridge (connected graph) topology in the entire suite |
| `run_louvain_isolated_concepts_normalized_contiguous_ids` | ✅ | Only test with exact sorted-list ID equality — the sole precise guard on the ID normalization step |
