# Test Evaluation: community_louvain_persistence

> **Traceability**: T-12.7, T-W1-8 · FR-23.7 · NFR-1, NFR-2  
> **File evaluated**: `crates/maenifold-core/tests/community_louvain_persistence.rs`  
> **SUT**: `maenifold_core::community::detect_and_persist`  
> **Color lens**: 🔘 Gray — Skeptical Inquiry  
> **Evaluator**: Red-team agent  

---

## Summary

This file contains **4 tests** exercising the `detect_and_persist` persistence path of the Louvain community detection pipeline. Tests are tagged `T-W1-8: RTM NFR-2` and mirror named C# Oracle tests. The suite covers persistence basics, FK integrity, idempotency, and gamma sensitivity — but one test has critically weakened assertions that no longer verify what its name claims. Three tests are genuinely load-bearing. One is a near-no-op that has quietly diverged from its Oracle counterpart.

**Key concern**: `run_louvain_higher_gamma_tends_to_more_communities` carries a 6-line apologetic comment explaining why the real assertion was weakened to `count_high >= count_low`. This is not a test — it's a checked-in admission that the behavior is non-verifiable. It must be rewritten.

**Coverage gap**: No test exercises the `seed` parameter through the full `detect_and_persist` surface, the empty-graph path, or verifies *which* nodes land in *which* community (only counts are checked).

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique Angle? | Assertions Correct? | Name Accurate? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `run_and_persist_fk_constraint_only_persists_existing_concepts` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Partially | **KEEP** |
| 2 | `run_louvain_higher_gamma_tends_to_more_communities` | ❌ No | ⚠️ Marginal | ❌ No — dangerously weak | ❌ No — name overpromises | ✅ Yes | **REWRITE** |
| 3 | `run_and_persist_persists_to_database` | ✅ Yes | ✅ Yes | ⚠️ Mostly — missing structural assertion | ✅ Yes | ⚠️ Partially | **KEEP** |
| 4 | `run_and_persist_second_run_replaces_existing` | ✅ Yes | ✅ Yes | ⚠️ Partial — no guard on `count_before > 0` | ✅ Yes | ⚠️ Partially | **KEEP** |

---

## Per-Test Analysis

### Test 1 — `run_and_persist_fk_constraint_only_persists_existing_concepts`

**What it claims**: Every row written to `concept_communities` must reference a concept that already exists in `concepts`.

**Is it load-bearing?**  
Yes, as a **regression guard**. Structurally, the production code is currently safe: `load_graph` sources `graph.concepts` from the `concepts` table, and `persist_assignments` writes those same names back. No phantom concept can appear. However, if a future refactor introduces a code path that derives community labels from an external source (e.g., inferred node names during fallback), this test would be the first line of defence. The `PRAGMA foreign_keys=ON` in `open_conn` also validates that the SQLite FK constraint is actually enforced at the DB level — a non-trivial check in SQLite where FK enforcement is off-by-default.

**Assertion quality**: High. The per-concept existence query loop is O(N) DB queries in test code, which is acceptable. Both the return value (`count == 2`) and the FK property are verified independently.

**What it does NOT check**: Whether the FK constraint violation *would* be caught by the DB at the SQL level if `PRAGMA foreign_keys` were OFF in production. That's an application configuration concern, not a test concern.

**Verdict**: **KEEP**. Unique angle, correct assertions, good name. Could be parameterized over topologies but the current single-case coverage is sufficient.

---

### Test 2 — `run_louvain_higher_gamma_tends_to_more_communities`

**What it claims**: Higher resolution parameter (`gamma`) produces more communities.

**Is it load-bearing?**  
No. The actual assertions are:
```rust
assert!(count_low >= 1, ...);   // trivially true: 8 nodes exist
assert!(count_high >= 1, ...);  // trivially true: 8 nodes exist
assert!(count_high >= count_low, ...);  // passes even if both == 2
```

With two fully disconnected 4-cliques and `seed=None` (non-deterministic):
- `gamma=0.5` → Louvain merges each clique → 2 communities  
- `gamma=2.0` → Louvain may leave nodes as singletons within a clique → fallback activates → connected-component fallback detects each fully-connected clique as one component → **still 2 communities**

Result: `count_high == count_low == 2` in the common case. The test passes while verifying nothing about gamma sensitivity. **This is a false green.**

**The comment is the tell**: A 6-line comment explaining why the Oracle assertion (`count_high > count_low`) had to be weakened is not documentation — it is an acknowledgement that the test has been hollowed out. A weakened assertion that can never distinguish a correct from an incorrect implementation is not a test.

**Root cause**: The fallback-to-connected-components mechanism (which is correct behaviour) makes the high-gamma case collapse back to 2 communities. The test needs to be redesigned around an observable that IS sensitive to gamma with this fallback in place — or use a topology where high gamma provably exceeds low gamma even after fallback.

**Verdict**: **REWRITE**. See Recommendations for a concrete alternative.

---

### Test 3 — `run_and_persist_persists_to_database`

**What it claims**: `detect_and_persist` writes the correct number of rows, returns the correct community count, stores a finite modularity, and stores the exact gamma value as resolution.

**Is it load-bearing?**  
Yes. This is the **primary contract test** for `detect_and_persist`. The four assertions each cover a distinct contract dimension:

| Assertion | Contract |
|-----------|----------|
| `count == 2` | Algorithm correctly detects 2 communities in the triangle+pair topology |
| `community_row_count == 5` | All 5 nodes are persisted; no partial write, no extras |
| `modularity.is_finite()` | Regression guard from T-15.9 gain-formula fix (NaN/Inf protection) |
| `(resolution - 1.0).abs() < 1e-10` | `gamma` parameter is faithfully stored as `resolution` |

**Missing assertion**: The test verifies *how many* communities exist but not *which nodes are in which community*. For the triangle+pair topology, alpha/beta/gamma must co-cluster and delta/epsilon must co-cluster. Without this, the test cannot distinguish a correct partition from a degenerate one (e.g., all 5 nodes in one community, which would fail the `count == 2` check, but a 3-2 split with wrong members would pass). The support module provides `get_community_id()` — it should be used here.

**Verdict**: **KEEP**. Core persistence test. Recommend adding structural membership assertions using `get_community_id()`.

---

### Test 4 — `run_and_persist_second_run_replaces_existing`

**What it claims**: Running `detect_and_persist` twice on the same data replaces rather than appends community rows.

**Is it load-bearing?**  
Yes. The `DELETE FROM concept_communities` + re-insert transaction in `persist_assignments` is an idempotency contract. Without it, every Sync cycle would accumulate stale assignments — a silent data corruption bug. This test would catch that regression immediately.

**Assertion quality**: Correct but thin. `count_after == count_before` confirms no duplication. However:
1. There is no assertion that `count_before > 0`. If the first `detect_and_persist` silently wrote 0 rows, the test would trivially pass (0 == 0) while masking a persistence failure.
2. The test doesn't verify data *content* equality — only row count. A buggy second run that deletes and rewrites nonsense data would pass. However, Test 3 already covers content correctness, so this is an acceptable scope boundary.

**Verdict**: **KEEP**. Add `assert!(count_before > 0, "first run must have written rows before idempotency check")`.

---

## Recommendations

### R-1: Rewrite `run_louvain_higher_gamma_tends_to_more_communities` (CRITICAL)

The test must be redesigned around a property that is actually gamma-sensitive after the fallback. Two options:

**Option A — Test singleton+fallback separation** (tests a real invariant):
```rust
// At gamma=100.0, Louvain leaves everything as singletons (no modularity gain).
// The fallback then assigns by connected components.
// For two disconnected cliques, this must produce exactly 2 communities.
fn high_gamma_fallback_produces_connected_components() {
    // ... setup two disconnected 4-cliques ...
    let count = detect_and_persist(&env.db_path, 100.0, Some(42)).expect("run");
    assert_eq!(count, 2, "at extreme gamma, fallback must produce one community per connected component");
}
```

**Option B — Test gamma sensitivity with a seeded run on a topology with known gamma threshold**:
```rust
// A chain graph: A—B—C—D—E has a known gamma threshold where it splits.
// Use seed=Some(42) for determinism. Verify count_high > count_low.
fn run_louvain_higher_gamma_splits_chain_topology() {
    // A-B-C-D-E chain with uniform weights
    // gamma=0.5 → one community
    // gamma=3.0 → multiple communities
    let count_low = detect_and_persist(&env.db_path, 0.5, Some(42)).expect("low");
    detect_and_persist(&env.db_path, 3.0, Some(42)).expect("high"); // re-runs on same DB
    // ...
}
```

### R-2: Add structural membership assertion to Test 3

After verifying row count and community count, assert topology correctness:
```rust
let conn = open_conn(&env.db_path);
let id_alpha = get_community_id(&conn, "alpha").expect("alpha");
let id_beta  = get_community_id(&conn, "beta").expect("beta");
let id_gamma = get_community_id(&conn, "gamma").expect("gamma");
let id_delta = get_community_id(&conn, "delta").expect("delta");
let id_eps   = get_community_id(&conn, "epsilon").expect("epsilon");
assert_eq!(id_alpha, id_beta,  "triangle members must co-cluster");
assert_eq!(id_beta,  id_gamma, "triangle members must co-cluster");
assert_ne!(id_alpha, id_delta, "triangle and pair must be distinct communities");
assert_eq!(id_delta, id_eps,   "pair members must co-cluster");
```

### R-3: Add `assert!(count_before > 0)` guard to Test 4

Prevents the test from silently passing when the first `detect_and_persist` fails to write any rows.

### R-4: Add missing coverage tests

The following scenarios have no coverage in this file (some may be covered elsewhere — verify before adding):

| Gap | Suggested Test Name | Rationale |
|-----|---------------------|-----------|
| Empty graph (no concepts, no edges) | `detect_and_persist_empty_graph_returns_zero` | Edge case: zero communities |
| Single isolated node | `detect_and_persist_single_node_returns_one_community` | Louvain with no edges |
| Seed determinism through full persistence path | `detect_and_persist_same_seed_produces_identical_assignments` | T-15.10 seed param tested in louvain.rs but not at persist level |
| Gamma stored for non-integer values | `run_and_persist_stores_arbitrary_gamma_as_resolution` | Test 3 only tests gamma=1.0 |
| Connected-components fallback explicitly | `fallback_to_connected_components_assigns_by_topology` | Fallback path has no direct persistence-level test |

---

## Positive Observations

- **Test isolation is correct**: Each test acquires `TEST_MUTEX`, uses `TempDir`, configures `Config::override_root`, and resets via `Drop`. No shared mutable state leaks between tests.
- **Foreign key enforcement**: `open_conn` consistently sets `PRAGMA foreign_keys=ON` — SQLite's default-off FK enforcement is actively opted into.
- **Oracle mirroring discipline**: Tests are tagged with their C# Oracle counterparts, enabling direct comparison. The naming convention is consistent.
- **Support module quality**: `insert_triangle_plus_isolated_pair` provides a canonical, named topology. The helpers (`get_community_id`, `community_row_count`, `get_modularity`) are appropriately scoped and reusable.
- **T-15.9 modularity regression guard**: The `modularity.is_finite()` assertion in Test 3 is a precise, named regression guard from a real bug fix. This is exactly how bug-fix tests should be written.

---

## Load-Bearing Score: 3/4

Three of four tests defend real, non-trivially-guaranteed properties of the production code. One (`run_louvain_higher_gamma_tends_to_more_communities`) has assertions so weak it cannot distinguish correct from incorrect behavior and must be rewritten before it can count as load-bearing.

| Test | Score |
|------|-------|
| `run_and_persist_fk_constraint_only_persists_existing_concepts` | ✅ Load-bearing |
| `run_louvain_higher_gamma_tends_to_more_communities` | ❌ Not load-bearing — assertions are trivially satisfiable |
| `run_and_persist_persists_to_database` | ✅ Load-bearing |
| `run_and_persist_second_run_replaces_existing` | ✅ Load-bearing |

**Overall**: The suite is structurally sound and well-organized, but carries one hollow test that provides false confidence in gamma-sensitivity behavior. Address R-1 through R-3 before T-16.2 migrates this code to `DbBackend` trait calls — migration is a high-risk refactor and these tests are the safety net.

---
## SWE Remediation
- REWRITE `run_louvain_higher_gamma_tends_to_more_communities` → `high_gamma_fallback_produces_connected_components` (R-1 Option A): replaced trivially-satisfiable `count_high >= count_low` assertions with deterministic test using `gamma=100.0, seed=Some(42)`. Asserts exactly 2 communities via `assert_eq!`, then verifies all 8 nodes' structural membership (clique-1 co-clusters, clique-2 co-clusters, cliques distinct) using `get_community_id()`. Test now verifies a real invariant: extreme gamma → singleton fallback → connected components.
- KEEP+strengthen `run_and_persist_persists_to_database` (R-2): added 5 structural membership assertions verifying triangle (alpha/beta/gamma co-cluster) and pair (delta/epsilon co-cluster) are distinct communities using `get_community_id()`.
- KEEP+strengthen `run_and_persist_second_run_replaces_existing` (R-3): added `assert!(count_before > 0)` guard to prevent false green when first `detect_and_persist` silently writes zero rows.
- KEEP `run_and_persist_fk_constraint_only_persists_existing_concepts`: unchanged.
- Commit: 16d24fd

