# Test Evaluation: sync_perf_graph
**File:** `crates/maenifold-core/tests/sync_perf_graph.rs`
**Test count:** 4
**Evaluated:** 2026-06-26
**Traceability:** T-0.1 · NFR-3 · NFR-2
**Color:** 🔘 Gray — Skeptical Inquiry

---

## Summary

`sync_perf_graph.rs` covers the `concept_graph` edge-building pipeline across four scenarios: basic edge creation with `source_files` JSON, proximity/section weighting, UPSERT idempotency under re-sync, and scale behavior at >1 000 concepts. All four tests are nominally load-bearing — each covers a scenario absent from the other three tests in this file and from the related `sync_perf_baseline.rs` / `concept_sync_edge_*` evaluations.

The core concern is **Test 4**, where the assertions are so permissive that a catastrophic partial failure (e.g., only a single edge written out of 604 450 expected) would still pass. The remaining three tests are well-constructed, though two have minor naming and assertion-precision gaps worth addressing.

Gray-lens disclaimer: the test infrastructure (`TestEnv` serializing all tests via `TEST_MUTEX`) means failures are isolated but also means these tests cannot run in parallel. This is a known trade-off, not a defect — but it is worth flagging for future parallelization work.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Accurate? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `build_graph_edges_creates_expected_edges_with_correct_source_files_json` | ✅ Yes | ✅ Yes | ⚠️ Partial (`count > 0` is weak) | ✅ Yes | 🔲 Minor | **KEEP** |
| 2 | `build_graph_edges_proximity_weight_same_section_vs_cross_section` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Partial (preamble nuance hidden) | 🔲 Minor | **KEEP** |
| 3 | `build_graph_edges_resync_does_not_duplicate_edges_co_occurrence_count_correct` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Partial (rowid stability omitted) | ❌ No | **KEEP** |
| 4 | `build_graph_edges_with_over_1000_concepts_creates_edges_without_error` | ✅ Yes | ✅ Yes | ❌ No (`edge_count > 0` is nearly meaningless) | ✅ Yes | 🔲 Minor | **REWRITE** |

---

## Findings

### Test 1 — `build_graph_edges_creates_expected_edges_with_correct_source_files_json`

**What it does:** Creates three concepts in a single flat markdown note, syncs it, then asserts all three ordered edge pairs exist in `concept_graph` with `source_files[uri] == 1.0`.

**What it validates correctly:**
- All C(3,2) = 3 edges are created on first sync.
- `source_files` JSON is populated with the correct URI key.
- `source_files[uri]` weight is exactly `1.0` — meaningful because the concepts appear in the same flat section (section_index=0), which `flush_relation_counts` stores as `0.5` weight per pair. But wait: all three concepts are in a single `## Shared Section` heading — section_index > 0 → weight 1.0. This is correct.

**Concern:** The assertion `assert!(count > 0.0)` on `co_occurrence_count` is weak. The implementation stores `entry.0 = entry.1.len() as f64` (number of distinct source files). With exactly one source file, the correct value is `1.0`. The test should assert `count == 1.0` to guard against future regressions in the count-vs-weight distinction introduced in T-14.4 (RTM FR-11).

**Suggested improvement:**
```rust
assert_eq!(
    count, 1.0,
    "co_occurrence_count should be 1.0 (one contributing file) for ({left}, {right})"
);
```

**Verdict: KEEP.** The `source_files` JSON structure check is the unique and irreplaceable assertion here. The `count > 0` weakness is low risk.

---

### Test 2 — `build_graph_edges_proximity_weight_same_section_vs_cross_section`

**What it does:** Constructs a document with concepts across a flat preamble and a named section. Asserts that:
- (a, b): 0.5 — cross-section (a is preamble-only, b is named-section-only)
- (a, c): 0.5 — preamble sections share `section_index=0`, but `flush_relation_counts` only grants 1.0 for `section_index > 0`
- (b, c): 1.0 — both in `## Section One` (section_index=1 > 0)

**What it validates correctly:** The full proximity weighting path through `flush_relation_counts` and `write_relation_counts` / `rebuild_affected_graph_from_mentions`.

**Concern 1 — Float equality:** `assert_eq!` on `f64` is used directly for `0.5` and `1.0`. Both values are exactly representable in IEEE 754 double precision, and the implementation stores them via `or_insert(weight)` without arithmetic. This is acceptable in practice, but it is an assumption. An `(actual - expected).abs() < f64::EPSILON` guard would be more defensive.

**Concern 2 — Hidden semantic:** The (a, c) = 0.5 result is non-obvious: concepts *a* and *c* co-appear in the preamble, so a reader might expect `1.0`. The test relies on `section_index=0` being treated as cross-section by design (T-4.3 comment in source: *"flat docs keep section_index=0 for extraction, but sync must treat that implicit root as cross-section"*). The test name "same_section_vs_cross_section" does not surface the preamble exception. A descriptive inline comment at the (a, c) assertion would prevent future confusion.

**Verdict: KEEP.** The test is correct and load-bearing. Naming and documentation improvements only.

---

### Test 3 — `build_graph_edges_resync_does_not_duplicate_edges_co_occurrence_count_correct`

**What it does:** Syncs a two-concept file, captures the edge's `rowid`, backdates `last_indexed` to force re-sync, modifies the file, re-syncs, then asserts: (1) exactly one row in `concept_graph`, (2) the `rowid` is unchanged (true UPSERT, not DELETE + INSERT), (3) `source_files[uri] == 1.0` (weight not doubled).

**What it validates correctly:** The `ON CONFLICT(concept_a, concept_b) DO UPDATE` semantics in `rebuild_affected_graph_from_mentions`. This is the strongest test in the file — the `rowid` stability assertion proves the implementation used SQLite UPSERT rather than a naive delete-then-insert strategy, which would have the same observable count but different rowid.

**Concern — Name gap:** The function name ends in `co_occurrence_count_correct` but the most distinctive assertion is `rowid` stability. The name misleads future maintainers about which invariant is primary.

**Suggested rename:** `build_graph_edges_resync_is_true_upsert_rowid_stable_and_weight_not_doubled`

**Verdict: KEEP.** No functional defects. Rename is a quality-of-life improvement.

---

### Test 4 — `build_graph_edges_with_over_1000_concepts_creates_edges_without_error`

**What it does:** Creates 1 100 concepts in one markdown note, syncs it, then asserts:
1. `edge_count > 0` from `COUNT(*) FROM concept_graph`
2. `query_edge_row_count(&env.cfg, &first, &second) == 1` (concepts[0] ↔ concepts[1])
3. `query_edge_row_count(&env.cfg, &penultimate, &last) == 1` (concepts[1098] ↔ concepts[1099])

**Critical concern — `edge_count > 0` is nearly meaningless:**

With 1 100 concepts, the expected edge count is C(1100, 2) = **604 450 edges**. An assertion of `edge_count > 0` would pass even if only a single edge was written. A fault that silently truncated the graph at edge 500 would be completely invisible to this test.

The path `perf-bt-002/` in the fixture name suggests this was written to guard against a specific scale bug (likely a SQLite `SQLITE_MAX_VARIABLE_NUMBER`-class issue or the `chunks(900)` pattern visible in `vector_embeddings.rs`). Even granting that intent, the assertion must bound the output more tightly to serve as a regression guard.

**What the boundary pair checks don't cover:** Concepts[0..1] and concepts[1098..1099] are adjacent pairs, trivially likely to appear together. They don't validate the cross-file, cross-position structure of the full combinatorial output. A bug that only processed the first 10 or last 10 concepts would still pass both checks.

**Suggested rewrite:**
```rust
// C(1100, 2) = 604_450
let expected_edges: i64 = (1100 * 1099) / 2;
assert_eq!(
    edge_count, expected_edges,
    "concept_graph should contain all C(1100,2) = {expected_edges} edges; got {edge_count}"
);
```

If exact count is too brittle (e.g., due to intentional pruning strategies), use a tight lower bound:
```rust
// At minimum, all adjacent pairs must exist (1099 edges at minimum)
// For full coverage: assert_eq!(edge_count, (1100 * 1099) / 2)
assert!(
    edge_count >= 1099,
    "expected at least 1099 sequential adjacent-pair edges, got {edge_count}"
);
```

However, the correct fix is the exact count. There is no pruning strategy in the current implementation for `flush_relation_counts` + `write_relation_counts` that would legitimately reduce C(1100, 2).

**Verdict: REWRITE** to assert `edge_count == 604_450`. The existing assertion cannot detect a wide class of regressions that would be catastrophic at production scale.

---

## Recommendations

Ordered by descending risk.

1. **[Critical] Rewrite Test 4's `edge_count` assertion** from `> 0` to `== 604_450`. The current form cannot detect bulk insertion failures. This is the single most important change in this file.

2. **[Low] Strengthen Test 1's `co_occurrence_count` assertion** from `count > 0.0` to `count == 1.0`. This guards the T-14.4 invariant (count = distinct source files, not sum of weights) at a zero cost.

3. **[Low] Rename Test 3** to `build_graph_edges_resync_is_true_upsert_rowid_stable_and_weight_not_doubled` to surface the `rowid`-stability invariant, which is the test's most distinctive contribution.

4. **[Informational] Add an inline comment in Test 2** at the `(a, c) == 0.5` assertion explaining that preamble (`section_index=0`) co-occurrence is intentionally treated as cross-section per T-4.3 / FR-23.2.

5. **[Informational] `TEST_MUTEX` serialization** is necessary given shared `Config` global state. This limits parallelism. The correct long-term fix is scoped config isolation (i.e., no global state), but that is out of scope for this evaluation.

---

## Load-Bearing Score: 4/4

All four tests cover scenarios not replicated elsewhere in this file or in the adjacent `sync_perf_baseline.rs` or `concept_sync_edge_*` evaluations. However, **Test 4's weak assertions reduce its effective regression value to near-zero** for the bulk-insertion scenario it claims to guard. It occupies a load-bearing slot in the matrix but delivers load-bearing coverage only for the "does not panic" case — not for "produces correct output."

Adjusted effective coverage: **3.1 / 4.0** (Test 4 contributes ≈ 0.1 for the "no panic" signal only).

---
## SWE Remediation
- REWRITE Test 4 (`build_graph_edges_with_over_1000_concepts_creates_edges_without_error`): replaced `edge_count > 0` with `edge_count == 604_450` (C(1100,2)) per Critical P0 recommendation. Retained boundary-pair spot checks.
- KEEP Test 1: strengthened `co_occurrence_count` assertion from `> 0.0` to `== 1.0` per P1 recommendation; guards T-14.4 invariant (count = distinct source files).
- KEEP Test 3: renamed to `build_graph_edges_resync_is_true_upsert_rowid_stable_and_weight_not_doubled` per P1 recommendation to surface rowid-stability invariant.
- KEEP Test 2: added inline comment at `(a, c) == 0.5` assertion explaining preamble cross-section behavior (T-4.3 / FR-23.2) per P2 recommendation.
- All 4 tests pass: `cargo test --test integration_tests -p maenifold-core -- sync_perf_graph` → 4 passed, 0 failed.
- Commit: def612f

---
## Red-team Audit
- ✅ REWRITE Test 4 (`build_graph_edges_with_over_1000_concepts_creates_edges_without_error`): `edge_count > 0` replaced with `edge_count == (1100 * 1099) / 2` (i.e. 604_450). Exact match to eval Critical P0 recommendation. Boundary-pair spot checks retained.
- ✅ KEEP Test 1 (`build_graph_edges_creates_expected_edges_with_correct_source_files_json`): `assert!(count > 0.0, …)` replaced with `assert_eq!(count, 1.0, …)`. Exact match to eval P1 recommendation. T-14.4 invariant now guarded.
- ✅ KEEP Test 3: renamed from `…resync_does_not_duplicate_edges_co_occurrence_count_correct` to `…resync_is_true_upsert_rowid_stable_and_weight_not_doubled`. Exact match to eval P1 recommendation. Functional body unchanged.
- ✅ KEEP Test 2 (`build_graph_edges_proximity_weight_same_section_vs_cross_section`): three-line inline comment added at `(a, c) == 0.5` assertion explaining preamble cross-section behavior (T-4.3 / FR-23.2). Exact match to eval P2 recommendation.
- ✅ No unrelated changes: commit `def612f` touches only `crates/maenifold-core/tests/sync_perf_graph.rs` (+14 / −5). No production code, no other test files, no Cargo.toml.
- ✅ Tests pass: independently confirmed 4/4 pass (`cargo test --test integration_tests -p maenifold-core -- sync_perf_graph`).
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- sync_perf_graph`: 4 passed, 0 failed
- Verdict: PASS
