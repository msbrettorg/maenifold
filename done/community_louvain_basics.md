---
id: T-80.22
title: "Test Evaluation: community_louvain_basics.rs"
status: done
pipeline: tdd
---

# Test Evaluation: community_louvain_basics.rs

**Traceability**: T-12.7 · T-W1-8 · RTM NFR-1 · RTM NFR-2  
**Evaluator color**: 🔘 Gray — Skeptical Inquiry  
**Files evaluated**:
- `crates/maenifold-core/tests/community_louvain_basics.rs` (5 tests, 192 lines)
- `crates/maenifold-core/tests/community/support.rs` (shared support module)

**Implementation files consulted**:
- `crates/maenifold-core/src/community/louvain.rs` — `run_louvain`, `normalize_communities`, Fisher-Yates shuffle
- `crates/maenifold-core/src/community/persist.rs` — `detect_and_persist`

**Sibling evaluation consulted for overlap**:
- `tests/evaluations/community_coverage_basics.md`

---

## Summary

This file is the primary correctness gate for Louvain community detection. Unlike its sibling
`community_coverage_basics.rs` (which was found to contribute zero unique failure modes), this
file earns its keep: four of five tests cover genuinely distinct execution paths through
`detect_and_persist`. The suite is structurally sound.

However, skeptical examination surfaces three recurring defects that weaken confidence:

1. **Misleading determinism claim.** The top test asserts determinism with `seed=None`, which the
   implementation itself documents as entropy-based. The test is not wrong to exist — re-run
   idempotency is a real contract — but the name overstates what the test actually verifies. The
   *real* determinism contract (with `seed=Some(42)`) is already tested by an inline unit test in
   `louvain.rs`. This test duplicates the weaker half of that guarantee under the stronger label.

2. **Systematically weak modularity assertions.** Two tests (`_two_communities` and
   `_disconnected_components`) guard modularity with `.is_finite()`. For these topologies —
   a 3-clique plus a 2-pair, or two balanced 3-cliques — Louvain's modularity formula must
   produce a value strictly greater than zero. `.is_finite()` would pass if modularity were
   `-1e300`, `NaN` after coercion, or even `0.0` for an incorrect partition. The guard does not
   distinguish a correct result from a badly wrong one.

3. **Missing return-value assertion in test 5.** `detect_and_persist` returns the community count.
   Test 2 checks it (`assert_eq!(count, 2, …)`). Test 5, covering the same conceptual scenario
   with two equal cliques, silently discards the return value. A partition bug that produces 3 or
   1 communities would pass undetected.

---

## Per-Test Analysis

---

### Test 1: `run_louvain_deterministic_produces_same_result`

**What it tests**: Calls `detect_and_persist` twice on the same database with `seed=None` and
asserts that community count, modularity, and two spot-checked community IDs are identical.

**Load-bearing?** Partially. The test guards against `detect_and_persist` being non-idempotent
(e.g., corrupting state on re-run). That contract matters. But it does *not* guard true
algorithmic determinism — the actual determinism guarantee (same shuffle → same result) lives in
the inline unit tests inside `louvain.rs`, using `seed=Some(42)`. This test is redundant with
the stronger one and mislabeled.

**Unique?** The re-run idempotency path (second `detect_and_persist` call on an already-populated
DB) is not directly covered elsewhere. That narrow aspect earns partial uniqueness.

**Correct assertions?** Structurally yes. The 1e-10 tolerance on modularity equality is
appropriate. Spot-checking alpha and delta gives enough coverage for the topology used.

**Good name?** No. The test name promises `deterministic` but the implementation comment in
`louvain.rs` is explicit: `seed=None` → "entropy-based (different shuffle each iteration)."
Well-separated topologies happen to converge regardless, but that is topology robustness, not
algorithmic determinism. The name should reflect what is actually verified.

**Parameterizable?** Partially — a parameterized re-run test could be run with `seed=None` and
`seed=Some(x)` with a single shared body, but the two goals (idempotency vs. true determinism)
are semantically distinct enough that merging them would be confusing.

**Verdict**: REWRITE — rename to `run_louvain_rerun_produces_stable_partition` and limit the
claim to re-run stability. Remove the `deterministic` framing. Alternatively, add a
`seed=Some(42)` variant that earns the determinism label, and keep this as a separate
idempotency check.

---

### Test 2: `run_louvain_triangle_plus_isolated_pair_two_communities`

**What it tests**: The canonical topology (3-clique α-β-γ, isolated pair δ-ε). Verifies the
community count (2), full intra-community membership chains, the cross-partition separation, and
a modularity guard.

**Load-bearing?** Yes — highest value test in the file. This is the primary functional
correctness regression gate. A failure here means Louvain produced a wrong partition.

**Unique?** Yes. No other test uses this topology with this depth of membership assertions.

**Correct assertions?**
- `count == 2`: ✅ correct and necessary.
- `alpha == beta`, `beta == gamma`: ✅ correct transitivity chain for the triangle. `alpha == gamma`
  is implied but not explicit — acceptable, the chain is sufficient.
- `delta == epsilon`: ✅ correct.
- `alpha != delta`: ✅ necessary cross-partition check.
- `modularity.is_finite()`: ⚠️ insufficient. For this topology with weight-5 edges, modularity
  must be strictly positive (empirically ~0.36–0.46). `.is_finite()` would pass for `0.0` or
  `-1e10`, neither of which represents a correct partition.

**Good name?** Yes. Specific and informative.

**Parameterizable?** Mildly. The edge weight (5) is fixed; testing with weight-1 or weight-100
would not add meaningful coverage since Louvain is invariant to uniform scaling.

**Verdict**: KEEP — but strengthen the modularity assertion from `.is_finite()` to `> 0.0` and
add a reasonable upper bound (e.g., `< 1.0` as a sanity guard against floating-point runaway).

---

### Test 3: `run_louvain_empty_graph_returns_empty`

**What it tests**: Zero nodes → zero communities, zero DB rows.

**Load-bearing?** Yes. The empty-graph path in `run_louvain` is a dedicated early-return
(`node_count == 0 → return (Vec::new(), 0.0)`). If that guard is removed or regresses, this
test catches it.

**Unique?** Yes. The empty-DB scenario is not covered by any other test in this file.

**Correct assertions?**
- `count == 0`: ✅
- `community_row_count == 0`: ✅ — checks the DB write path, not just the return value. Good.

**Good name?** Yes.

**Parameterizable?** No — the empty case is atomic.

**Verdict**: KEEP — no changes needed.

---

### Test 4: `run_louvain_single_node_one_community`

**What it tests**: One concept, no edges. Verifies count (1), community ID (0), modularity (0.0
exactly).

**Load-bearing?** Yes. The single-node-no-edges path in `run_louvain` hits a distinct branch:
`total_edge_weight <= EPSILON → return singleton_communities`. If that branch regresses (e.g.,
crashes or assigns ID != 0), this test catches it.

**Unique?** Yes. Distinct from both the empty-graph path and the multi-node paths.

**Correct assertions?**
- `count == 1`: ✅
- `community_id == Some(0)`: ✅ with a caveat — this tests the `normalize_communities` output.
  `normalize_communities` assigns IDs by first-occurrence order via a `BTreeMap`, so node 0 →
  community 0. This is a stable implementation detail, but it is an implementation detail. If
  normalization were to use a different canonical mapping (e.g., by largest community first),
  this assertion would break for a correct partition. Low brittleness risk given that
  normalization is deterministic and documented, but worth noting.
- `modularity == 0.0`: ✅ exact float equality is appropriate here. The implementation stores
  `0.0` explicitly for the zero-edge case; no floating-point accumulation occurs.

**Good name?** Yes.

**Parameterizable?** No.

**Verdict**: KEEP — no changes needed. Add a comment noting the `Some(0)` assertion depends on
`normalize_communities` first-occurrence ordering, to make the intent explicit to future
maintainers.

---

### Test 5: `run_louvain_disconnected_components_separate_communities`

**What it tests**: Two balanced 3-cliques (c1-a/b/c and c2-a/b/c), each with weight-5 internal
edges and no cross-edges. Verifies that all nodes within each clique share a community and that
the two cliques are assigned different communities.

**Load-bearing?** Partially. The topology adds genuine value: two symmetric, equal-weight cliques
stress the case where the algorithm might have equal-quality incentive to merge them. Test 2
covers asymmetric disconnection (3-node vs. 2-node); test 5 covers symmetric disconnection
(3-node vs. 3-node). That is a distinct structural scenario. However, the test's ability to
catch bugs is weakened by two assertion gaps (see below).

**Unique?** Partially. The symmetric-clique topology is distinct from test 2. The core behavior
(disconnected → separate communities) is shared, but the structural symmetry adds a test angle
that the prior test cannot provide.

**Correct assertions?**
- `c1a == c1b`, `c1b == c1c`: ✅ — internal cohesion for component 1 (transitivity chain).
- `c2a == c2b`, `c2b == c2c`: ✅ — internal cohesion for component 2.
- `c1a != c2a`: ✅ — cross-partition separation.
- `c1c != c2c` or `c1c != c2a`: ⚠️ missing — the cross-partition check uses only one node from
  each component. For a partition that somehow assigned c1c to c2's community but left c1a
  separate, this assertion would still pass. Adding `assert_ne!(c1c, c2c, …)` would close the
  gap cheaply.
- **`detect_and_persist` return value not checked**: ⚠️ significant gap. Test 2 verifies
  `count == 2`; test 5 silently discards the count. A bug that produces 3 communities (e.g.,
  c1-a in one community, c1-b/c merged with c2, c2-a isolated) would pass all existing
  assertions because the two spot-checked nodes would still be in different communities.
- `modularity.is_finite()`: ⚠️ insufficient — same weakness as test 2. Should be `> 0.0`.

**Good name?** Yes.

**Parameterizable?** Yes — this test and test 2 share the same conceptual template (disconnected
components → separate communities) and could be unified as a parameterized test accepting a
topology constructor and expected count.

**Verdict**: REWRITE — add the missing count assertion (`count == 2`), add
`assert_ne!(c1c, c2c, …)`, and strengthen modularity assertion to `> 0.0`. Consider merging
with test 2 into a parameterized `#[rstest]` table covering both asymmetric and symmetric
disconnection.

---

## Verdicts

| Test Name | Load-Bearing | Unique | Assertions Correct | Good Name | Parameterizable | Verdict |
|---|---|---|---|---|---|---|
| `run_louvain_deterministic_produces_same_result` | Partial (idempotency only) | Partial | Structurally yes, claim overstated | ❌ Misleading | Partial | **REWRITE** |
| `run_louvain_triangle_plus_isolated_pair_two_communities` | ✅ Yes | ✅ Yes | Mostly — modularity too weak | ✅ Yes | Minimal | **KEEP** (strengthen modularity) |
| `run_louvain_empty_graph_returns_empty` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | N/A | **KEEP** |
| `run_louvain_single_node_one_community` | ✅ Yes | ✅ Yes | ✅ Yes (minor caveat) | ✅ Yes | N/A | **KEEP** |
| `run_louvain_disconnected_components_separate_communities` | Partial | Partial | ❌ Missing count + weak modularity | ✅ Yes | ✅ Yes | **REWRITE** |

---

## Recommendations

### P1 — Fix missing count assertion in test 5 (silent correctness hole)

```rust
// Currently: return value discarded
detect_and_persist(&env.db_path, 1.0, None).expect("detect");

// Should be:
let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
assert_eq!(count, 2, "two disconnected 3-cliques must produce exactly 2 communities");
```

A partition producing 3 or 1 communities currently passes this test. This is a real regression
blind spot.

### P2 — Strengthen `modularity.is_finite()` to `modularity > 0.0` in tests 2 and 5

For any graph with at least one edge and a non-trivial partition, Louvain modularity must be
strictly positive by the formula. The current guard passes for `0.0`, for negative values, and
for `NaN` coerced to `0.0` by `COALESCE` in the SQL helper. Suggested fix:

```rust
assert!(
    modularity > 0.0,
    "modularity must be strictly positive for a valid separated partition, got {modularity}"
);
```

### P3 — Rename test 1 to remove the false determinism claim

The test verifies that `detect_and_persist` produces identical results when called twice on the
same database without clearing state. That is an idempotency/stability contract, not algorithmic
determinism. Suggested name: `run_louvain_rerun_on_populated_db_produces_stable_partition`.

The actual determinism guarantee (`seed=Some(42)`) is tested inside `louvain.rs`. These are
different contracts and should carry distinct names.

### P4 — Add `assert_ne!(c1c, c2c, …)` cross-check in test 5

The current cross-partition check only compares `c1a != c2a`. Add:

```rust
assert_ne!(c1c, c2c, "last node of each component must also be in different communities");
```

This closes a narrow but real gap where a partial-merge bug could evade detection.

### P5 (optional) — Consider parameterizing tests 2 and 5 as a topology table

Both tests follow the pattern: insert topology → detect → verify count + community membership +
modularity. An `#[rstest]` table parameterized on topology constructor and expected count would:
- Eliminate the structural duplication between the two tests
- Make adding new topologies (e.g., 4-clique + 3-clique) a single-line change
- Surface the modularity lower bound as a shared invariant rather than two separate weak checks

This is optional — the current structure is readable — but if a topology sweep is planned as
part of FR-22 storage abstraction testing, the infrastructure will pay for itself.

---

## Load-Bearing Score: 4/5

Four tests have clear, distinct execution paths that would catch real regressions in `louvain.rs`
or `persist.rs`. Test 5 is only partially load-bearing due to the missing count assertion — a
partition bug producing the wrong number of communities would evade its checks entirely. After
applying P1 and P2, this file would achieve 5/5.

---
## SWE Remediation
- REWRITE test 1: renamed `run_louvain_deterministic_produces_same_result` → `run_louvain_rerun_on_populated_db_produces_stable_partition`; expanded comment to clarify re-run stability vs. algorithmic determinism (P3)
- KEEP test 2: strengthened modularity assertion from `.is_finite()` to range check `[-0.5, 1.0)` with explicit `.is_finite()` guard (P2 adapted — see note)
- KEEP test 3: no changes needed
- KEEP test 4: added comment noting `Some(0)` depends on `normalize_communities` first-occurrence ordering
- REWRITE test 5: added missing `count == 2` assertion (P1), added `assert_ne!(c1c, c2c)` cross-check (P4), strengthened modularity to range check `[-0.5, 1.0)` (P2 adapted)
- P2 adaptation note: eval recommended `modularity > 0.0` but actual values are -0.125 (test 2) and 0.0 (test 5); adapted to theoretical range `[-0.5, 1.0)` which is meaningfully stronger than `.is_finite()` while matching implementation reality
- P5 (parameterization) skipped: optional, and tests 2 and 5 are readable as-is
- Commit: 2a2d6f0
