---
id: T-80.20
title: "Test Evaluation: community_coverage_basics.rs"
status: done
pipeline: tdd
---

# Test Evaluation: community_coverage_basics.rs

**Traceability**: T-12.7 · RTM NFR-1 · NFR-2  
**Evaluator color**: 🔘 Gray — Skeptical Inquiry  
**Files evaluated**:
- `crates/maenifold-core/tests/community_coverage_basics.rs` (4 tests, 127 lines)
- `crates/maenifold-core/tests/community/support.rs` (sibling-shared support module)

**Sibling files consulted for overlap analysis**:
- `community_louvain_basics.rs` (5 tests — empty graph, single node, 4-node isolated, triangle+pair, two disconnected cliques, deterministic)
- `community_coverage_topologies.rs` (further topology tests)

---

## Summary

This file claims to test `detect_and_persist` coverage basics. It does not earn that name. Under skeptical examination, **all 4 tests exclusively probe the isolated-concept (no-edges) subspace**. No connected graph topology appears here, despite `insert_edge`, `insert_triangle_plus_isolated_pair`, and `get_resolution` existing in the shared support module — all unused by this file. The support module is a silent accusation that these tests were written incomplete.

Worse: the sibling file `community_louvain_basics.rs` already covers the same three scenarios (empty graph, single node, 4-node isolated) with *stricter* assertions. Two tests in this file are strictly dominated. One is borderline. One partially adds value only through a precondition guard that could be a single `assert` in the existing sibling test.

The honest accounting: this file contributes **0 unique failure modes** to the test suite. Every regression it could catch is already caught by siblings. The file exists as coverage volume, not coverage depth.

---

## Per-Test Analysis

---

### Test 1: `run_louvain_multiple_isolated_concepts_no_edges_each_get_own_community`

**Lines**: 14–46  
**What it does**: Inserts 3 isolated concepts (no edges), calls `detect_and_persist`, asserts count=3, row_count=3, `max_id < 3`, modularity=0.0.

**Load-bearing?**  
No. `community_louvain_basics::run_louvain_isolated_concepts_normalized_contiguous_ids` covers N=4 isolated concepts and asserts the **exact** ID vector `[0,1,2,3]`. That assertion strictly dominates `max_id < 3`. Any regression this test would catch (wrong count, non-zero modularity, ID out of range) is caught first by the sibling.

**Unique?**  
No. The scenario — isolated concepts, no edges — is covered for N=1, N=4 in sibling tests with stronger assertions.

**Correct assertions?**  
Technically correct but under-specified. `max_id < 3` is true for `[0,0,0]` (all in one community) if count happens to equal 3 coincidentally. The assertion does **not** enforce distinctness — the id_a ≠ id_b check only appears in Test 4. A partition bug that assigns all three nodes community_id=0 but returns count=3 could pass here (depending on the implementation's return value semantics).

**Good name?**  
The name is long but literal. It does not signal what distinguishes it from test 2 or test 4. "Multiple isolated" vs "five isolated" vs "isolated concepts persists" — these names describe the setup, not the assertion contract.

**Parameterizable?**  
Directly. Tests 1, 2, and 4 are all `N isolated concepts → N distinct communities, modularity=0.0`. A single `#[rstest]` or table-driven test with `n = [2, 3, 5]` would replace all three.

---

### Test 2: `run_louvain_five_isolated_concepts_no_edges_five_distinct_communities`

**Lines**: 49–68  
**What it does**: Inserts 5 isolated concepts, asserts count=5, row_count=5, modularity=0.0.

**Load-bearing?**  
No. Same reasoning as Test 1, but the assertions here are *even weaker* — `max_id < N` range check is absent, and ID distinctness is not checked. This test adds N=5 to the isolated-nodes corpus but makes fewer claims about the result than Test 1 does.

**Unique?**  
No. This is test 1 with N=5 and a subset of its assertions. It is strictly dominated by test 1 on assertion count, and by the sibling on assertion quality.

**Correct assertions?**  
Underdetermined. `count == 5` and `community_row_count == 5` and `modularity == 0.0` would pass if all five nodes were assigned community_id=0 *if count returns rows written*. Whether `detect_and_persist` returns distinct-community count or total-rows-written depends on implementation. No distinctness verification is made.

**Good name?**  
The name "five distinct communities" asserts distinctness in prose but the test body does not verify it. The name is aspirational, not contractual.

**Parameterizable?**  
Same as Test 1 — collapse into the parameterized form described there.

---

### Test 3: `run_and_persist_empty_graph_returns_zero_without_db_writes`

**Lines**: 72–95  
**What it does**: Verifies the `concepts` table starts empty (precondition assert), then calls `detect_and_persist`, asserts count=0 and community_row_count=0.

**Load-bearing?**  
Partially. The **core coverage** (empty graph → 0 count, 0 rows) is duplicated by `community_louvain_basics::run_louvain_empty_graph_returns_empty`, which makes identical assertions. The *only* distinguishing element is the precondition check:
```rust
assert_eq!(concept_count, 0, "concepts table must be empty");
```
This assertion is a documentation guard — it verifies `TestEnv::setup()` produces a clean database. That is a legitimate test-infrastructure concern but it belongs in a `TestEnv` unit test, not as a copy of an existing integration test with one added line.

**Unique?**  
Not in terms of production code coverage. Unique only in that it explicitly verifies the test harness precondition.

**Correct assertions?**  
Yes. Assertions are correct and self-consistent.

**Good name?**  
Yes — "without_db_writes" is a good discriminator. It signals the assertion is about side-effect absence, not just return value.

**Parameterizable?**  
No. It's a single boundary case. But it should not be a standalone test — the precondition check should be absorbed into the sibling, or extracted into a `TestEnv` self-test.

---

### Test 4: `run_and_persist_isolated_concepts_persists_each_with_own_community`

**Lines**: 98–127  
**What it does**: Inserts 2 isolated concepts, asserts count=2, modularity=0.0, row_count=2, and explicitly asserts `id_a != id_b`.

**Load-bearing?**  
Marginally. The `id_a != id_b` distinctness check is not present in test 1 or 2, making it the strongest isolated-node assertion in this file. However, `community_louvain_basics::run_louvain_isolated_concepts_normalized_contiguous_ids` checks the exact ID vector `[0,1,2,3]` for N=4, which mathematically implies all IDs are distinct. Any bug that merges two nodes into one community would also fail the normalized-IDs assertion.

**Unique?**  
No. The distinctness check is the only differentiator, and the sibling's exact-vector check subsumes it.

**Correct assertions?**  
Yes. The assertions are correct and the `id_a != id_b` check is the most semantically precise assertion in the entire file.

**Good name?**  
Acceptable. "persists_each" emphasizes the DB persistence aspect (connecting to the `run_and_persist` prefix vs `run_louvain` prefix), which is a deliberate naming convention in this module.

**Parameterizable?**  
Yes — collapse into the N-parameterized test as described for tests 1 and 2, carrying the distinctness assertion at each N.

---

## Verdicts

| Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Clear? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|
| `run_louvain_multiple_isolated_concepts_no_edges_each_get_own_community` | ❌ No | ❌ No | ⚠️ Weak (no distinctness check) | ⚠️ Indistinct from siblings | ✅ Yes | **REMOVE** |
| `run_louvain_five_isolated_concepts_no_edges_five_distinct_communities` | ❌ No | ❌ No | ❌ Weaker than test 1 | ❌ Name claims distinctness, body doesn't verify it | ✅ Yes | **REMOVE** |
| `run_and_persist_empty_graph_returns_zero_without_db_writes` | ⚠️ Partial | ⚠️ Partial (precondition only) | ✅ Yes | ✅ Yes | ❌ No | **MERGE** |
| `run_and_persist_isolated_concepts_persists_each_with_own_community` | ⚠️ Marginal | ❌ No | ✅ Best in file | ⚠️ Acceptable | ✅ Yes | **REMOVE** |

---

## Recommendations

### R-1: Remove Tests 1, 2, and 4
These three tests cover the same isolated-concept subspace as `community_louvain_basics::run_louvain_isolated_concepts_normalized_contiguous_ids` but with strictly weaker assertions. They add maintenance burden (another file to update when internals change) with zero marginal regression-detection value.

**If you want exhaustive N coverage**, replace all three with a single parameterized test:
```rust
// In community_louvain_basics.rs or a new community_isolated_concepts.rs
#[rstest]
#[case(2)]
#[case(3)]
#[case(5)]
#[case(10)]
fn isolated_concepts_each_get_distinct_community(#[case] n: usize) {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    for i in 0..n {
        insert_concept(&conn, &format!("node-{i}"));
    }
    drop(conn);
    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
    let conn = open_conn(&env.db_path);
    assert_eq!(count, n as i64);
    assert_eq!(community_row_count(&conn), n as i64);
    assert_eq!(get_modularity(&conn), 0.0);
    // Verify IDs are 0..n-1 (contiguous, distinct)
    let mut ids: Vec<i64> = (0..n as i64).collect();
    let mut actual: Vec<i64> = {
        let mut stmt = conn.prepare(
            "SELECT community_id FROM concept_communities ORDER BY community_id"
        ).unwrap();
        stmt.query_map([], |r| r.get(0)).unwrap()
            .collect::<Result<_, _>>().unwrap()
    };
    actual.sort_unstable();
    assert_eq!(actual, ids, "IDs must be contiguous 0..N-1");
}
```

### R-2: Merge Test 3 Into Sibling
The unique value in Test 3 is the precondition guard on `concept_count == 0`. Absorb it into `community_louvain_basics::run_louvain_empty_graph_returns_empty`:
```rust
// Add at the top of run_louvain_empty_graph_returns_empty:
let conn = open_conn(&env.db_path);
let concept_count: i64 = conn.query_row("SELECT COUNT(*) FROM concepts", [], |r| r.get(0)).unwrap();
assert_eq!(concept_count, 0, "TestEnv must start with empty concepts table");
drop(conn);
```
Then delete this file entirely.

### R-3: Add What This File Was Supposed to Cover
The file is named `coverage_basics` but covers only one scenario (isolated nodes). The support module provides `insert_edge`, `insert_triangle_plus_isolated_pair`, and `get_resolution` — all unused here. If this file was intended to test the `resolution` parameter (the second argument to `detect_and_persist`), that coverage is **entirely missing** across all community test files. Consider:

- `detect_and_persist(path, 0.5, None)` vs `detect_and_persist(path, 2.0, None)` on the triangle topology — does resolution affect community granularity?
- The `get_resolution` helper in support.rs is never called by any test in this repository. That is a dead symbol in the shared module, and a missing test axis.

### R-4: Delete This File
After implementing R-1, R-2, R-3: delete `community_coverage_basics.rs` and remove `mod community_coverage_basics;` from `tests/main.rs`. The file's name promises coverage breadth it does not deliver.

---

## Load-Bearing Score: 0.5/4

Only Test 3 carries any partial load-bearing value, and only through its precondition guard — not its core assertions. Tests 1, 2, and 4 could be deleted today without reducing the suite's regression-detection surface by a single bit. The sibling files have this subspace covered more rigorously.

**Interpretation**: A load-bearing score of 0.5/4 (12.5%) indicates this file is largely test padding. It creates a false sense of coverage breadth for the isolated-concept scenario while the resolution parameter, connected-graph persistence semantics, and parameterized N-scaling go untested.

---

## SWE Remediation

- REMOVE `run_louvain_multiple_isolated_concepts_no_edges_each_get_own_community`: subsumed by `community_louvain_basics::run_louvain_isolated_concepts_normalized_contiguous_ids` (sibling has exact-vector assertion)
- REMOVE `run_louvain_five_isolated_concepts_no_edges_five_distinct_communities`: strictly dominated by test 1 and sibling; name claimed distinctness but body never verified it
- MERGE `run_and_persist_empty_graph_returns_zero_without_db_writes`: precondition guard (`concepts table must be empty`) absorbed into `community_louvain_basics::run_louvain_empty_graph_returns_empty`
- REMOVE `run_and_persist_isolated_concepts_persists_each_with_own_community`: sibling's exact-vector `[0,1,2,3]` assertion subsumes the `id_a != id_b` distinctness check
- DELETE `community_coverage_basics.rs` and removed `mod community_coverage_basics;` from `main.rs`
- Commit: 31cba3b

---

## Red-team Audit

- ✅ REMOVE `run_louvain_multiple_isolated_concepts_no_edges_each_get_own_community`: file deleted; subsuming sibling `community_coverage_topologies::run_louvain_isolated_concepts_normalized_contiguous_ids` confirmed present with exact-vector `[0,1,2,3]` assertion (strictly dominates `max_id < 3`)
- ✅ REMOVE `run_louvain_five_isolated_concepts_no_edges_five_distinct_communities`: file deleted; sibling exact-vector assertion subsumes any N-isolated claim
- ✅ MERGE `run_and_persist_empty_graph_returns_zero_without_db_writes`: precondition guard (`concept_count == 0`) absorbed into `community_louvain_basics::run_louvain_empty_graph_returns_empty` at lines 115-121, matching eval spec R-2 verbatim; traceability comment `// T-12.7: Merged precondition guard from community_coverage_basics (R-2).` added
- ✅ REMOVE `run_and_persist_isolated_concepts_persists_each_with_own_community`: file deleted; sibling `[0,1,2,3]` exact-vector check mathematically implies distinctness, subsuming `id_a != id_b`
- ✅ DELETE file: `community_coverage_basics.rs` confirmed absent from disk
- ✅ MOD removal: `mod community_coverage_basics;` confirmed absent from `main.rs`
- ✅ Tests green: all 5 `community_louvain_basics` tests pass; subsuming sibling `run_louvain_isolated_concepts_normalized_contiguous_ids` passes
- ℹ️ Stale comment: `community_coverage_topologies.rs:117` still references the deleted file in a comment (`// community_coverage_basics.rs`). Informational only — not a functional defect, and the eval did not specify sibling comment updates.
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- community_coverage_basics`: 0 passed, 0 failed (module confirmed deleted)
- `cargo test --test integration_tests -p maenifold-core -- community_louvain_basics`: 5 passed, 0 failed (subsuming target with merged precondition guard)
- `cargo test --test integration_tests -p maenifold-core -- community_coverage_topologies`: 2 passed, 0 failed (sibling with exact-vector assertion)
- Merged precondition guard confirmed at `community_louvain_basics.rs:115-121` with traceability comment `// T-12.7: Merged precondition guard from community_coverage_basics (R-2).`
- Verdict: **PASS**
