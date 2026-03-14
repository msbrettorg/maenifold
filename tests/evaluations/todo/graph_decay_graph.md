# Test Evaluation: graph_decay_graph
**File:** `crates/maenifold-core/tests/graph_decay_graph.rs`
**Test count:** 4
**Evaluated:** 2026-07-16
**Traceability:** T-0.3 · NFR-2 (test coverage) · NFR-4 (code quality) · FR-7.5 (decay weighting in graph operations) · NFR-7.5.4 (read-memory bypasses decay)

---

## Summary

All 4 tests are **currently passing** and all 4 address real, load-bearing behavior. This file covers the slice of decay weighting that the other two graph decay files do not: `BuildContext` and `FindSimilarConcepts` ranking, plus the `ReadMemory` non-decay invariant. The test suite is structurally sound and not redundant with `graph_decay_search.rs` or `graph_decay_access.rs`.

That said, two issues warrant attention:

1. **Test 2 uses fragile string-position ordering** (`str::find()` on raw Markdown) instead of the structured, index-based approach used in `graph_decay_search.rs` (`title_index()` on a JSON results array). A future formatting change to the `FindSimilarConcepts` Markdown output — adding a header, a footer note, or an introductory line containing a concept name — could silently invalidate the ordering assertion.

2. **Tests 1 and 2 don't assert that decay was the cause of ordering.** They assert *that* the newer concept ranks higher, not *why*. If some other factor (alphabetical, insertion order, co-occurrence count) happened to produce the same ordering, these tests would pass through a regression of the decay system. The `graph_decay_search.rs` grace-period test (`search_memories_files_within_grace_period_have_equal_ranking_weight`) provides this stronger evidence for SearchMemories by asserting on raw `decayWeight` values. No equivalent exists for BuildContext or FindSimilarConcepts.

There is also a **coverage gap**: `graph_decay_search.rs` has a grace-period boundary test for SearchMemories (two files within 10 days both scoring 1.0), but no analogous boundary test exists for BuildContext or FindSimilarConcepts. This leaves the grace-period cutoff unverified for those two code paths.

**Pass rate: 4/4 (100%)**

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `build_context_newer_related_concepts_rank_higher` | **KEEP** | Load-bearing, unique to BuildContext path, structurally correct. Weakness: no weight assertion; ordering could be driven by non-decay factors without detection. |
| `find_similar_concepts_newer_concepts_rank_higher` | **REWRITE** | Load-bearing, unique to FindSimilarConcepts path, but string-position assertion on Markdown output is fragile. Should use JSON output + index comparison as `graph_decay_search.rs` does. |
| `read_memory_bypasses_decay_and_returns_full_content_regardless_of_age` | **KEEP** | Load-bearing invariant. Establishes baseline: ReadMemory reads the file directly without requiring DB indexing. Complementary to test 4, not redundant with it. |
| `read_memory_decayed_content_remains_fully_retrievable` | **KEEP** | Load-bearing. Covers the post-sync path that test 3 does not. The search assertion is intentionally presence-only (`minScore: 0.0`), which is correct for this invariant (decay must not cause exclusion). |

---

## Findings

### Finding 1: Fragile String-Position Assertion in `find_similar_concepts_newer_concepts_rank_higher`

The test calls `find_similar_concepts_markdown()` (which returns `OutputFormat::Markdown`) and then uses `str::find()` to determine which concept name appears first in the raw string:

```rust
let modern_index = similar_result.find("modern-framework");
let legacy_index = similar_result.find("legacy-framework");
if let (Some(modern_index), Some(legacy_index)) = (modern_index, legacy_index) {
    assert!(
        modern_index < legacy_index,
        "newer framework concept should rank ahead of legacy concept: {similar_result}"
    );
```

`str::find()` returns the byte offset of the **first occurrence anywhere in the string** — not the position in the ranked list. Consider a future output format change that adds a section header, preamble, or contextual note containing a concept name:

```
## Concepts similar to software-architecture
> Including legacy-framework and modern-framework (ranked by relevance)

1. modern-framework (score: 0.91)
2. legacy-framework (score: 0.76)
```

In this output `legacy-framework` (offset ~53) appears before `modern-framework` (offset ~75) in the raw string, causing the assertion to **fail despite correct ranking**. Conversely, any output that mentions `modern-framework` in a footer or note would cause the assertion to **pass despite wrong ranking**.

The correct approach — already used in `graph_decay_search.rs` via `title_index()` — is to use `OutputFormat::Json` and find position in the structured results array:

```rust
// PREFERRED: JSON + structured index comparison
let similar_result_json = find_similar_concepts_json(&env, "software-architecture");
let modern_index = concept_index(&similar_result_json, "modern-framework");
let legacy_index = concept_index(&similar_result_json, "legacy-framework");
assert!(modern_index < legacy_index, "...");
```

This requires adding a `find_similar_concepts_json()` and `concept_index()` helper to `support.rs`, analogous to `search_memories_json()` and `title_index()`. The effort is small; the robustness gain is real.

---

### Finding 2: No Assertion That Decay Was the Cause of Ordering

Tests 1 and 2 verify ordering but not mechanism. If the decay weight calculation were silently set to `1.0` for all files (a neutral no-op regression), these tests would still pass — provided no other factor inverted the ordering. This creates a gap: a full decay regression might go undetected here.

`graph_decay_search.rs` closes this gap for SearchMemories by asserting raw `decayWeight` values:

```rust
// From graph_decay_search.rs — grace period test
support::assert_approx_eq(
    day1["decayWeight"].as_f64().expect("day1 decay weight"),
    1.0,
    0.000_001,
);
```

For BuildContext and FindSimilarConcepts, no equivalent exists. The gap is partially mitigated by the unit tests inside `decay/scoring.rs` (which verify the formula directly) and by the `config_decay_weights.rs` tests. However, no integration test confirms that the scoring output is *actually applied* to BuildContext or FindSimilarConcepts relations — only that the ordering happened to come out right on this run.

**Recommended addition** (new test or augmentation of existing):

```rust
// Assertion pattern for BuildContext — verify actual decay weight values differ
let new_pattern = relations.iter().find(|e| e["Name"] == "new-pattern").unwrap();
let old_pattern = relations.iter().find(|e| e["Name"] == "old-pattern").unwrap();
assert!(
    new_pattern["DecayWeight"].as_f64().unwrap() > old_pattern["DecayWeight"].as_f64().unwrap(),
    "decay weights should differ: new={}, old={}",
    new_pattern["DecayWeight"], old_pattern["DecayWeight"]
);
```

Whether `BuildContext` JSON output exposes `DecayWeight` per relation needs to be checked — if it does not, this suggestion is blocked pending a format change, and the ordering-only approach remains the practical choice.

---

### Finding 3: Coverage Gap — Grace Period Boundary for BuildContext and FindSimilarConcepts

`graph_decay_search.rs` contains:

```rust
fn search_memories_files_within_grace_period_have_equal_ranking_weight()
```

This verifies that two files within the default grace period (day 1 and day 10) both score exactly `1.0` and are not artificially penalized. **No equivalent test exists for BuildContext or FindSimilarConcepts.** The grace-period logic path in `decay/scoring.rs` (`if age_days <= grace_period_days { return 1.0; }`) is called by all three tools, but only SearchMemories has a test that exercises the boundary.

This is a meaningful gap: the grace period is a configurable parameter (`decay_grace_days_default()`). If it were accidentally set to 0 in the default config, tests 1 and 2 would still pass (the 60-day file decays, the 2-day file scores 1.0 → ordering is correct), but the grace-period guarantee would be broken for all three tools with only SearchMemories catching it.

---

### Finding 4: Shared Tag `T-GRAPH-DECAY-001.1` Across Multiple Test Files

Both `graph_decay_graph.rs` and `graph_decay_search.rs` tag multiple tests with `// T-GRAPH-DECAY-001.1: RTM FR-7.5`. This tag appears on 5 tests across the two files (tests 1–2 here, tests 1–3 in the search file). If `T-GRAPH-DECAY-001.1` is intended to be a unique test case identifier, this is incorrect. If it is a grouping label for "decay weighting affects ranking," it is acceptable — but ambiguous, and any tooling that maps T-IDs to individual tests would produce a collision.

The mixed usage (some tags like `T-GRAPH-DECAY-001.5` appear to be unique per scenario, while `T-GRAPH-DECAY-001.1` is shared broadly) suggests the tagging scheme was not applied consistently at this granularity. This is low-severity but worth noting for traceability hygiene.

---

### Finding 5: Tests 3 and 4 Are Complementary, Not Redundant

At first glance, tests 3 and 4 both assert that `ReadMemory` returns full content for a decayed file — suggesting potential redundancy. On closer inspection, they are complementary:

- **Test 3** (`read_memory_bypasses_decay...`): No `sync_graph` call. Proves `ReadMemory` reads the file directly from disk at the filesystem level, regardless of database state. 365-day age.
- **Test 4** (`read_memory_decayed_content_remains_fully_retrievable`): Calls `sync_graph` explicitly. Proves `ReadMemory` works correctly after the file has been indexed, scored, and had decay metadata written to the database. 90-day age. Also adds search presence verification.

The distinction matters: test 3 ensures the feature does not consult decay metadata at all; test 4 ensures it continues to work correctly even when decay metadata *exists* for the file in the database. Both invariants are independently worth testing.

---

## Positive Observations

- **Zero redundancy across the three `graph_decay_*.rs` files.** This file covers BuildContext and FindSimilarConcepts; `graph_decay_search.rs` covers SearchMemories (Hybrid, Semantic, FullText); `graph_decay_access.rs` covers weight updates on ReadMemory and no-change on SearchMemories. The split is clean and intentional.
- **`TEST_MUTEX` + `isolated_env()` pattern is correctly applied to all 4 tests.** `Config::override_root` is global state; the mutex prevents parallel-test races. This is load-bearing infrastructure.
- **The `find_result` / `panic!` idiom in test 4 is correctly calibrated for the invariant being tested.** A missing result causes a panic (the file *must* appear); partial results or low scores are acceptable.
- **The `decayed_uri` variable in test 4** correctly captures the return value of `create_test_file_with_age` (the memory URI) and reuses it for both `read_memory_markdown` and `search_memories_json`, avoiding string duplication.
- **Test names are accurate and descriptive.** Each name specifies the subject (tool), condition (decayed/newer/older), and expected behavior (ranks higher / bypasses decay / remains retrievable).

---

## Recommendations

**Priority 1 (test correctness):**

1. **Rewrite `find_similar_concepts_newer_concepts_rank_higher`** to use `OutputFormat::Json` and structural index comparison. Add `find_similar_concepts_json()` helper to `support.rs`. The current `str::find()` approach is a latent false-positive/negative waiting on a Markdown format change.

**Priority 2 (coverage completeness):**

2. **Add a grace-period boundary test for BuildContext:** create two files at day 1 and day 10, sync, call `build_context`, and assert both `new-pattern` and `old-pattern` concepts appear with `DecayWeight ≈ 1.0` (if the JSON output exposes this) — or at minimum assert their relative order is indeterminate (not reliably sorted by age within the grace window).
3. **Add a grace-period boundary test for FindSimilarConcepts:** same pattern as above, verifying that two concepts within the grace period are not penalized relative to each other.

**Priority 3 (assertion strength):**

4. **Augment tests 1 and 2 with weight assertions** (if `BuildContext` and `FindSimilarConcepts` JSON output exposes decay weight per result). Even a simple `assert!(new_weight > old_weight)` would confirm decay is the mechanism, not just a side-effect of other ordering factors.
5. **Clarify the `T-GRAPH-DECAY-001.1` tag scope.** If it's a group label, document this. If it should be unique per test, assign distinct sub-IDs (e.g., `T-GRAPH-DECAY-001.1a`, `T-GRAPH-DECAY-001.1b`) to avoid traceability collisions with the 3 tests in `graph_decay_search.rs` sharing the same tag.

---

## Load-Bearing Score: 4/4

| Test | Conceptually Necessary | Currently Functioning | Assertion Quality |
|---|---|---|---|
| `build_context_newer_related_concepts_rank_higher` | ✅ Yes | ✅ Passing | ⚠️ Ordering only — no causal verification |
| `find_similar_concepts_newer_concepts_rank_higher` | ✅ Yes | ✅ Passing | ❌ Fragile `str::find()` on Markdown |
| `read_memory_bypasses_decay_and_returns_full_content_regardless_of_age` | ✅ Yes | ✅ Passing | ✅ Sound |
| `read_memory_decayed_content_remains_fully_retrievable` | ✅ Yes | ✅ Passing | ✅ Sound |

**All 4 tests are conceptually necessary and currently passing.** No test should be removed. One should be rewritten (test 2). Two could be strengthened with weight assertions (tests 1 and 2). The file would benefit from two additional grace-period boundary tests to close the coverage gap that `graph_decay_search.rs` already addresses for SearchMemories.

---
## SWE Remediation
- REWRITE `find_similar_concepts_newer_concepts_rank_higher`: replaced fragile `str::find()` on Markdown with `OutputFormat::Json` + structured `concept_index()` comparison per P1 recommendation. Added `find_similar_concepts_json()` and `concept_index()` helpers to `graph_decay/support.rs`.
- Added `OutputFormat::Json` support to `find_similar_concepts` in `similar/mod.rs`: extracted `rank_candidates()` shared ranking logic, added `format_ranked_results_json()` producing `{name, similarity, decayWeight, weightedSimilarity}` per result.
- KEEP `build_context_newer_related_concepts_rank_higher`: unchanged
- KEEP `read_memory_bypasses_decay_and_returns_full_content_regardless_of_age`: unchanged
- KEEP `read_memory_decayed_content_remains_fully_retrievable`: unchanged
- P2/P3 recommendations (grace-period boundary tests, weight assertions, tag clarification) deferred — not required by eval verdicts
- All 4 tests pass
- Commit: 861bae0
