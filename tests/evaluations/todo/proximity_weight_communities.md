# Test Evaluation: proximity_weight_communities

**File:** `crates/maenifold-core/tests/proximity_weight_communities.rs`
**Test count:** 5
**Evaluated:** 2026-05-29
**Traceability:** T-4.3 / RTM FR-23.6, FR-23.7 · T-14.6 / RTM FR-15
**Reviewer stance:** Gray (skeptical inquiry) — demand evidence, challenge assumptions.

---

## Summary

Five tests spanning three behavioral contracts: (1) Louvain community clustering is driven by
proximity-weighted edges, (2) `compute_nmi` is numerically correct, and (3) community drift emits
a warning. All five are load-bearing. However, three carry defects that undermine their evidential
value: the two pure `compute_nmi` unit tests acquire a global file-lock they do not need (cargo-cult
safety), one NMI test checks only `< 1.0` when the specific inputs yield a mathematically
computable `≈ 0.0`, and the drift test uses `seed: None` making Louvain non-deterministic. Two
integration tests (`clusters_hubs_together`, `treats_empty_source_files`) are structurally sound
but have undocumented coupling choices. The net result: the contracts are real, the coverage is
meaningful, but three of the five tests need rewriting before they qualify as specifications rather
than smoke checks.

---

## Per-Test Analysis

### Test 1 — `detect_and_persist_clusters_hubs_together_when_proximity_sum_is_strongest`

**Traceability:** T-4.3 / RTM FR-23.6

**What it actually tests:**
Creates a 9-node graph with two hubs (a, b) sharing a proximity-100 edge, each hub also connected
to 3 satellite nodes via proximity-1 edges. Calls `detect_and_persist(gamma=0.5)` and asserts
`community_id(a) == community_id(b)`.

**Is it load-bearing?**
Yes. This is the primary evidence that proximity-weighted edges (not raw co-occurrence counts) drive
community assignment. Without this test, a regression that silently ignores `source_files` weights
would pass all other tests.

**Is it unique?**
Yes. No other test exercises the full pipeline with a strongly-biased proximity ratio.

**Correct assertions?**
Partially. The test confirms that a and b are co-located but never verifies where the satellites
land. With gamma=0.5 (low resolution), Louvain will likely merge everything into one community,
making the hub assertion trivially true for reasons unrelated to proximity. A more
discriminating graph — where the hub edge is strong enough to keep a+b together against a
competing partition — would isolate the claim. As written, the test might pass even if proximity
weights were ignored entirely, because any reasonable Louvain run would merge two well-connected
hubs.

**Secondary concern — implicit decay=1.0:**
The test URIs (`memory://test/prox-h-heavy.md`, `memory://test/prox-h-sat.md`) have no rows in
`file_content`. `get_decay_weight_for_file_with_connection` returns 1.0 for missing files (confirmed
by `community_edge_weight_uses_source_file_weights_when_no_db_row` in `data.rs`). This assumption
is load-bearing for the test to work at its intended proximity ratio, but it is entirely implicit.

**Non-determinism:**
`seed: None` means random Louvain node-visit ordering. The 100:1 proximity ratio makes the result
highly stable in practice, but there is no theoretical guarantee. `seed: Some(0)` costs nothing.

**Good name?**
Yes — descriptive and accurate.

**Parameterizable?**
Moderately. The gamma value and weight ratio are magic constants. Parameterizing over gamma would
reveal whether the assertion holds across resolution scales.

---

### Test 2 — `detect_and_persist_treats_empty_source_files_as_isolated_edges`

**Traceability:** T-4.3 / RTM FR-23.6

**What it actually tests:**
Creates an anchor pair (with a real edge) and an isolated pair (with `source_files="{}"`,
`co_occurrence_count=5.0`). Asserts the isolated pair ends up in different communities.

**Is it load-bearing?**
Yes. This tests the critical boundary: `{}` source_files → `parse_source_file_weights` returns
empty map → `graph_edge_weight` sums over nothing → 0.0 → edge is dropped from the adjacency list
→ two nodes become singletons. If this pathway silently broke, isolated-x and isolated-y would
merge even when they share no meaningful co-occurrence.

**Is it unique?**
Yes. The only test that directly pins the zero-weight edge exclusion path.

**Correct assertions?**
Yes — `assert_ne` with an explanatory message. However, the anchor pair (`anchor-a`/`anchor-b`)
is unexplained dead weight. These nodes are never asserted on. Removing them does not change the
test outcome because isolated-x and isolated-y become singletons regardless of whether other
concepts exist in the graph. The presence of anchors implies they matter when they do not, which
misleads future readers.

**Good name?**
Yes.

**Parameterizable?**
Slightly. An array-format source_files (`"[]"`) is a parallel empty-weight code path in
`parse_source_file_weights` that is not covered here or elsewhere in this file.

---

### Test 3 — `compute_nmi_returns_one_for_identical_clusterings`

**Traceability:** T-4.3 / RTM FR-23.7

**What it actually tests:**
Passes a 4-concept, 2-cluster `BTreeMap` as both `left` and `right` arguments to `compute_nmi`.
Asserts result ≈ 1.0.

**Is it load-bearing?**
Yes. Tests the mathematical identity: NMI(C, C) = 1.0. Foundational correctness for the
convergence criterion.

**Is it unique?**
Yes (within this file).

**Correct assertions?**
Yes. `assert_approx_eq` with 1e-9 tolerance. For identical inputs the math is:
`MI = H_L = H_R` → `NMI = 1.0` exactly. Floating-point is exact here; the tolerance is
conservative but harmless.

**Critical defect — unnecessary global lock:**
`acquire_test_lock()` serializes this test against every other test in the file, including the two
integration tests. `compute_nmi` is a **pure function** — no database, no filesystem, no global
state. The lock buys nothing and communicates a false assumption to the reader: that this call
has side effects requiring serialization. It also extends the wall-clock time of the test run
needlessly. This is cargo-cult safety copied from the integration tests above.

**Good name?**
Yes.

**Parameterizable?**
Yes. Tests 3 and 4 follow identical structure and should be merged into a single `rstest`-parameterized
test covering at minimum: identical partition (NMI=1.0), perfectly orthogonal partition (NMI=0.0),
and a partially overlapping partition (0 < NMI < 1).

---

### Test 4 — `compute_nmi_returns_value_below_one_for_different_clusterings`

**Traceability:** T-4.3 / RTM FR-23.7

**What it actually tests:**
Two 4-concept, 2-cluster partitions that assign the same four concepts in incompatible ways:
`clustering_a` groups {alpha,beta} and {gamma,delta}; `clustering_b` groups {alpha,gamma} and
{beta,delta}. Asserts `nmi < 1.0` and `nmi ∈ [0.0, 1.0]`.

**Is it load-bearing?**
Yes. Without this, the NMI function could return 1.0 unconditionally and no test would catch it.

**Is it unique?**
Yes.

**Correct assertions?**
**No — dangerously weak.** The two partitions used here are maximally orthogonal: every joint
(left_cluster, right_cluster) combination appears exactly once in a 2×2 contingency table with
equal marginals. The mutual information is exactly **0.0**, giving **NMI = 0.0**. The test
asserts `nmi < 1.0`, which would also pass if the function returned 0.9999. A regression that
computed inflated NMI values (e.g., a sign error in the entropy term) would survive this test
undetected. The assertion should be:

```rust
assert_approx_eq(nmi, 0.0);
```

The `assert!((0.0..=1.0).contains(&nmi))` range check is fine as a secondary guard but is
redundant given the `.clamp(0.0, 1.0)` in the implementation.

**Critical defect — unnecessary global lock:**
Same problem as Test 3. `compute_nmi` is pure; the lock is meaningless and misleading.

**Good name?**
Yes.

**Parameterizable?**
Yes — see Test 3 analysis.

---

### Test 5 — `detect_and_persist_logs_warning_when_community_drift_exceeds_threshold`

**Traceability:** T-14.6 / RTM FR-15

**What it actually tests:**
Builds two isolated triangles, detects communities (first run), destroys the graph, installs a
perfect bipartite matching between the two triangles, detects communities again (second run), and
asserts that stderr contains `"drift"`, `"DRIFT"`, or `"NMI"`.

**Is it load-bearing?**
Yes. This is the **only** test in this file covering the drift warning pathway (T-14.6 / RTM FR-15).
If it were removed, the warning mechanism could be deleted silently.

**Is it unique?**
Yes.

**Correct assertions?**
Weak. The assertion accepts any message containing "drift", "DRIFT", or "NMI". This would pass
if an unrelated warning happened to appear in stderr. The actual warning format from
`maybe_warn_on_community_drift` is:
```
Community drift warning: NMI {:.4} fell below threshold {:.4}\n
```
Both "NMI" and a numeric value are present. The test does not verify the reported NMI value
falls within the expected range (≈ 0.0 given the maximally reorganized graph).

**Mathematical grounding of the scenario:**
The second-run communities after the bipartite-matching restructure are three 2-node components
(via `fallback_to_connected_components`): `{a0,b0}`, `{a1,b1}`, `{a2,b2}`. Against the
first-run clustering of `{a0,a1,a2}` vs `{b0,b1,b2}`, this produces a 2×3 contingency table
with uniform cell counts (each cell = 1/6) and equal marginals — MI = 0, NMI = 0.0. The warning
will always fire with threshold 0.95. The scenario is logically sound.

**Non-determinism — critical defect:**
Both `detect_and_persist` calls use `seed: None`. The second run on the bipartite graph is
deterministic in practice (three disconnected components → `fallback_to_connected_components` is
canonical), but the first run on two triangles is driven by random node-visit order. If the
triangles happened to merge into one community, the "previous_assignments" would not match
expectation and the NMI computation would differ. This is unlikely but possible. `seed: Some(42)`
on both calls eliminates the risk.

**Unsafe stderr capture:**
`capture_stderr` in `support.rs` uses raw POSIX `pipe`/`dup`/`dup2` FD manipulation. This is
sound within the lock scope (only one test runs at a time), but it is brittle:
- A panic inside `operation()` would leave stderr redirected permanently for the process.
- It couples the test mechanism to the implementation detail that drift warnings use `write(2, ...)` 
  (the `#[cfg(unix)]` unsafe block in `metrics.rs`) rather than `eprintln!`.
- The `#[cfg(not(unix))]` codepath in `metrics.rs` uses `eprintln!` but is not tested anywhere.

**Good name?**
Yes.

**Parameterizable?**
No — scenario-specific.

---

## Verdicts

| # | Test Function | Load-Bearing | Unique | Correct Assertions | Good Name | Verdict |
|---|---|---|---|---|---|---|
| 1 | `detect_and_persist_clusters_hubs_together_when_proximity_sum_is_strongest` | ✅ | ✅ | ⚠️ Partial | ✅ | **KEEP** |
| 2 | `detect_and_persist_treats_empty_source_files_as_isolated_edges` | ✅ | ✅ | ✅ | ✅ | **REWRITE** *(minor — remove anchor noise)* |
| 3 | `compute_nmi_returns_one_for_identical_clusterings` | ✅ | ✅ | ✅ | ✅ | **REWRITE** *(remove spurious lock; merge into parameterized suite)* |
| 4 | `compute_nmi_returns_value_below_one_for_different_clusterings` | ✅ | ✅ | ❌ Too weak | ✅ | **REWRITE** *(fix assertion to `≈ 0.0`; remove spurious lock)* |
| 5 | `detect_and_persist_logs_warning_when_community_drift_exceeds_threshold` | ✅ | ✅ | ⚠️ Loose | ✅ | **REWRITE** *(add seed; strengthen assertion)* |

---

## Recommendations

### R-1 · Fix the weak NMI assertion in Test 4 (HIGH)

The orthogonal 2×2 partition used in `compute_nmi_returns_value_below_one_for_different_clusterings`
yields MI = 0 → NMI = 0.0 mathematically. The assertion should be:

```rust
assert_approx_eq(nmi, 0.0);
assert!((0.0..=1.0).contains(&nmi), "NMI must remain normalized, got {nmi}");
```

As written, a badly broken `compute_nmi` that returns 0.999 would pass. This is a specification
failure, not a style concern. **Traceability:** RTM FR-23.7.

### R-2 · Seed all Louvain calls in this file (HIGH)

Tests 1, 2, and 5 call `detect_and_persist(..., None)`. Tests 1 and 5 are sensitive to node-visit
ordering even if the probability of flipping is low. Use `seed: Some(0)` uniformly:

```rust
detect_and_persist(&env.cfg.database_path, 0.5, Some(0)).expect("detect communities");
```

This costs nothing and eliminates a class of intermittent failures. **Traceability:** T-15.10 / RTM FR-23.7.

### R-3 · Remove `acquire_test_lock()` from Tests 3 and 4 (MEDIUM)

`compute_nmi` is a pure function. Acquiring a global file-lock on a pure function is
misleading to readers and wastes test time on lock contention with integration tests.
Remove the lock from Tests 3 and 4. The NMI tests can run in parallel with everything.

### R-4 · Merge Tests 3 and 4 into a parameterized suite (MEDIUM)

Replace both NMI unit tests with a single `rstest`-parameterized test covering at minimum:

| Case | Expected NMI |
|---|---|
| Identical partition (2 clusters, 4 concepts) | ≈ 1.0 |
| Perfectly orthogonal 2×2 partition | ≈ 0.0 |
| Partial overlap (1 concept moved) | 0.0 < NMI < 1.0 |
| Single-concept overlap | ≥ 0.0, ≤ 1.0 |
| Disjoint concept sets | 1.0 (implementation returns 1.0 for empty intersection) |

**Traceability:** RTM FR-23.7.

### R-5 · Remove or explain the anchor pair in Test 2 (LOW)

`anchor-a` and `anchor-b` are inserted and never asserted on. Their presence implies they affect
the test outcome when they do not. Either document why they exist (`// Provides a non-empty
connected component so the graph is non-trivial`) or remove them.

### R-6 · Strengthen Test 5 drift assertion (MEDIUM)

The current assertion (`stderr.contains("drift") || stderr.contains("DRIFT") || stderr.contains("NMI")`)
is met by any incidental log output. Prefer:

```rust
assert!(
    stderr_output.contains("NMI") && stderr_output.contains("0.00"),
    "expected drift warning with NMI near 0.0 in stderr, got: {stderr_output}"
);
```

This pins both that the correct mechanism fired AND that the reported metric is in the expected
range for the constructed scenario. **Traceability:** T-14.6 / RTM FR-15.

### R-7 · Add `#[cfg(not(unix))]` drift warning test (LOW)

`metrics.rs` has two codepaths for the drift warning: the `unsafe write(2, ...)` path on Unix
and the `eprintln!` path on non-Unix. Only the Unix path is tested. The non-Unix path is dead
coverage. Either test it (possible via Rust's `io::stderr` capture) or remove the
`#[cfg(not(unix))]` branch and accept Unix-only warning behavior as documented.

---

## Deeper Concerns

### D-1 · Test 1 does not isolate the proximity mechanism

The name claims the test verifies that "proximity sum is strongest," implying a causal link
between the proximity=100 edge and the clustering outcome. But the assertion (`community_id(a) == community_id(b)`)
would also pass if proximity weights were ignored entirely and Louvain simply clustered on
topology (the a–b edge is the unique inter-hub link regardless of weight). A more rigorous test
would:

1. Run with proximity weights enabled (existing behavior) → verify a+b cluster together.
2. Run with all proximity weights forced to 1.0 (e.g., by inserting `{}` source_files) → verify
   the clustering is different (e.g., a+b separate because the hub link no longer dominates).

Without this contrast, the test is a topology test wearing a proximity-weight label.

### D-2 · File-lock mechanism is brittle across test-binary invocations

`acquire_test_lock()` creates a file at `$TMPDIR/maenifold-rs-proximity-weight-tests.lock`. If a
test panics while holding the lock, the file is cleaned up on `Drop`. But if the test process
is killed (`SIGKILL`, OOM), the file persists and subsequent `cargo test` invocations spin for
30 seconds before panicking. This is an inherited issue from the `proximity_weight_sections.rs`
shared support module. The 30-second timeout is generous enough to succeed in practice but is a
latent CI footgun.

### D-3 · `capture_stderr` panic-safety

The `capture_stderr` helper in `support.rs` redirects `fd 2` globally. If `operation()` panics,
`fd 2` remains redirected to the pipe and subsequent test failures (including Rust's panic
formatting) will be swallowed silently. Wrapping with `std::panic::catch_unwind` and always
restoring the FD would prevent this class of silent failure.

---

## Load-Bearing Score: 5/5

All five tests guard real behavioral contracts. None are redundant. The score reflects quantity
of load-bearing tests, not assertion quality — three require rewriting before they function as
precise specifications.
