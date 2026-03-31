---
id: community_sibling_scope
status: done
---

# Test Evaluation: community_sibling_scope.rs

**File**: `crates/maenifold-core/tests/community_sibling_scope.rs`
**Traceability**: T-15.7 · FR-13.7 · RTM (no discrete RTM entry; embedded in T-15.7 delivery)
**Evaluator color**: 🔘 Gray — Skeptical Inquiry
**Tests evaluated**: 3

---

## Summary

This file guards a single behavioral invariant: sibling candidates surfaced by `BuildContext → CommunitySiblings` must be restricted to concepts that share the same `community_id` as the query concept. The old all-concepts fallback (which surfaced cross-community concepts) was removed in T-15.7; these three tests are the regression fence.

All three tests are at the **integration level**: they invoke `ToolRegistry::global()` and exercise the full stack (SQL, Louvain, `build_context`, JSON serialization). None of the tests call `load_community_siblings()` directly. That makes them robust against refactors but slow and mutation-fragile in a different way — they cannot isolate which layer introduced a regression.

Key implementation facts consulted during evaluation (from `traversal.rs`):
- `load_community_sibling_candidates` filters by `WHERE community_id = ?1 AND concept_name != ?2` — the hard gate.
- `load_community_siblings` applies two post-filter thresholds: `MAENIFOLD_COMMUNITY_MIN_SHARED` (default 3) and `MAENIFOLD_COMMUNITY_MIN_OVERLAP` (default 0.4). A candidate that passes the community gate but scores below these thresholds is also excluded.
- Tests 1 and 3 structurally depend on threshold defaults. Neither test documents this dependency.

---

## Verdicts

| # | Test Name | Load-Bearing | Unique | Correct Assertions | Good Name | Parameterizable | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `sibling_candidates_restricted_to_same_community` | ✅ High | ✅ Yes | ⚠️ Mostly | ✅ Yes | ⬜ No | **KEEP** |
| 2 | `no_community_returns_empty_siblings` | ✅ High | ✅ Yes | ✅ Yes | ✅ Yes | ⬜ No | **KEEP** |
| 3 | `different_community_concepts_not_siblings` | ⚠️ Medium | ⚠️ Partial | ⚠️ Weak | ✅ Yes | ⬜ No | **REWRITE** |

---

## Per-Test Analysis

---

### Test 1 — `sibling_candidates_restricted_to_same_community`

**Purpose**: Builds two tight, equal-weight clusters via `detect_and_persist` (Louvain), then asserts `peer-a` appears in siblings and `alien-b` does not.

**Load-bearing?** YES. This is the primary end-to-end regression guard for T-15.7. It exercises the full path: graph insertion → community detection → `BuildContext` → JSON deserialization → sibling list inspection.

**Unique?** YES. The only test that uses real Louvain community detection rather than manual SQL injection. This exercises the `detect_and_persist` → `concept_communities` → sibling query pipeline, which Test 3 deliberately bypasses.

**Correct assertions?**
- `comm_count >= 2` — **weak**. It does not assert that `query-a` and `peer-a` land in the *same* community, only that two communities exist. The follow-up `assert_eq!(query_comm, peer_comm)` is what actually validates the structural precondition. The `>= 2` check could be tightened to `== 2` given the symmetric two-cluster topology, but this would risk brittleness if Louvain sub-divides one cluster (possible with resolution changes).
- `assert!(sibling_names.contains(&"peer-a"))` — correct and meaningful.
- `assert!(!sibling_names.contains(&"alien-b"))` — correct and meaningful.
- **Missing**: No assertion on `peer-a`'s `normalized_overlap` or ranking position. No guard against `peer-a` accidentally being a direct relation (which would exclude it from siblings by the `direct_relation_names` filter in `load_community_siblings`). The test does not insert a direct edge between `query-a` and `peer-a`, so this holds — but it is an undocumented assumption.
- **Missing**: No assertion on siblings list size. If the list contains unexpected extras (e.g., `na1`–`na4`), the test would pass silently. Checking `na1`–`na4` are absent would strengthen the test.

**Good name?** YES. Unambiguous statement of the contract being verified.

**Parameterizable?** The edge weight (10) and shared-neighbor count (4) are hardcoded. The count of 4 is specifically above the `MAENIFOLD_COMMUNITY_MIN_SHARED` threshold of 3. If that threshold changes, Test 1 could silently stop testing what it claims. Documenting the threshold dependency inside the test (a `// requires: shared_count(4) > MIN_SHARED(3)` comment) would make the contract explicit.

**Verdict: KEEP** — high-value integration test. Opportunistic strengthening welcome but not required.

---

### Test 2 — `no_community_returns_empty_siblings`

**Purpose**: Creates concepts with edges but deliberately skips `detect_and_persist`, asserting that `CommunitySiblings` returns an empty list with no all-concepts fallback.

**Load-bearing?** YES. This is the sole test for the "no fallback" requirement stated in T-15.7. Without this test, the guard against reintroducing the all-concepts fallback path would be absent.

**Unique?** YES. Tests 1 and 3 both involve concepts that *have* communities (just different ones). Test 2 is the only test for the `community_id IS NULL` path — which routes through `include_community_data = false` → `load_community_siblings` is never called → result is `[]`.

**Correct assertions?**
- `assert!(siblings.is_empty())` — correct, precise, and sufficient.
- The test setup is appropriately minimal: creates `orphan-t` as a concept that shares 4 neighbors with `orphan-q` (which would qualify as a sibling if community detection had run), but deliberately withholds detection. This makes the negative result meaningful — it is not trivially empty because there are no candidates; it is empty because no community was assigned.

**Good name?** YES. Precisely describes the condition and expected output.

**Parameterizable?** No meaningful axes of parameterization.

**Verdict: KEEP** — clean, well-targeted negative test with no issues.

---

### Test 3 — `different_community_concepts_not_siblings`

**Purpose**: Manually assigns `cross-query` to community 100 and `cross-alien` to community 200 via direct SQL, then asserts `cross-alien` does not appear in siblings despite sharing 4 common neighbors (which would score `normalized_overlap = 1.0` if filtering were removed).

**Load-bearing?** MEDIUM. The behavioral claim (cross-community exclusion) is the same as Test 1. What differs is the *mechanism*: Test 3 tests the SQL `WHERE community_id = ?1` filter in isolation from the Louvain path, providing a layer-specific regression guard. This is genuinely useful — if the Louvain path is refactored or replaced, Test 1's community assignments could silently change, making Test 3 the only reliable gate.

**Unique?** PARTIALLY. The cross-community exclusion behavior is shared with Test 1. The unique contribution is testing manual/static community assignment rather than algorithm-derived placement. However, the test conflates two distinct scenarios: (a) the filter excludes cross-community candidates at the SQL level, and (b) same-community candidates are still scored and returned correctly. Test 3 only validates (a) through a negative assertion.

**Correct assertions?**
The core assertion — `!sibling_names.contains(&"cross-alien")` — is **valid but weak**:

1. **The negative assertion is semantically correct.** If the community filter is removed, `cross-alien` would score `normalized_overlap = 1.0` and `shared_neighbor_count = 4` (both above thresholds), so it *would* appear. The assertion correctly fails in that case.

2. **Missing positive companion.** The test graph is structured so that `cn1`–`cn4` (same community as `cross-query`) have zero overlap with `cross-query` (no shared neighbors among them, since no cn-to-cn edges are defined). As a result, the siblings list is *empty* when filtering is correct. An empty list trivially does not contain `cross-alien`. This means the test cannot distinguish between:
   - "Filtering worked correctly and excluded cross-alien from candidates" ✅
   - "Filtering broke but cross-alien happened to be excluded by the overlap threshold (e.g., if thresholds were raised)" ❌
   
   Adding a sibling that *is* in the same community and *would* score positively — or explicitly asserting `siblings.is_empty()` — would close this ambiguity.

3. **Undocumented threshold dependency.** The test relies on `cross-alien` having `shared_neighbor_count = 4 ≥ MIN_SHARED(3)` and `normalized_overlap = 1.0 ≥ MIN_OVERLAP(0.4)`. Neither default is referenced in the test or a comment.

4. **The `cn1`–`cn4` community assignments are necessary but the rationale is undocumented.** A reader unfamiliar with `load_community_sibling_candidates` will not understand why `cn1`–`cn4` need community IDs at all — the comment could note that without them, the candidate list would be unexpectedly small (only `cross-query` itself is excluded, not `cross-alien`... actually `cross-alien` is excluded at the SQL WHERE clause, so the cn1-cn4 assignments are correct context for the candidate pool, not strictly necessary for the filter to work).

**Good name?** YES, though `different_community_concepts_not_siblings_despite_full_overlap` would more precisely communicate that the test is meaningful because the concepts *would* qualify by overlap alone.

**Parameterizable?** Community IDs (100, 200) are arbitrary; no value in parameterizing.

**Verdict: REWRITE** — add a positive assertion that demonstrates the test is non-trivial:

```rust
// Option A: assert siblings list is empty (making intent explicit)
assert!(
    siblings.is_empty(),
    "T-15.7: when same-community concepts have no shared neighbors, siblings must be empty; got: {sibling_names:?}"
);

// AND the existing negative check
assert!(
    !sibling_names.contains(&"cross-alien"),
    "T-15.7: cross-alien (community 200) must NOT appear..."
);
```

Or alternatively, add a second concept to community 100 that shares ≥3 neighbors *with each other* (not just with the query), so the siblings list is non-empty and the cross-alien exclusion is tested against a populated result set.

---

## Structural Concerns (Cross-Test)

### 1. Global singleton coupling
All three tests use `TEST_MUTEX.lock()` to serialize against `Config` and `ToolRegistry::global()`. This is a necessary constraint given the architecture but means:
- Tests run serially (no parallelism benefit from `cargo test`)
- A test that panics while holding the mutex can deadlock the suite (mitigated by `parking_lot::Mutex` which is not poisonable on panic, but the `TestEnv::Drop → Config::reset_overrides()` may not run if a test panics before `TestEnv` is bound)

Recommend: verify that `TestEnv` is always bound before the first assertion, so Drop runs on unwind.

### 2. No unit-level tests for `load_community_siblings` directly
All three tests operate above the internal function boundary. A failing test provides no signal about which layer broke — was it `detect_and_persist`, the SQL filter, the overlap scoring, or the JSON serialization? Lower-level tests against `load_community_siblings()` (or at least a separate test file) would make failure diagnosis faster.

### 3. Threshold defaults are implicit contract
`MAENIFOLD_COMMUNITY_MIN_SHARED` (default 3) and `MAENIFOLD_COMMUNITY_MIN_OVERLAP` (default 0.4) determine which same-community candidates survive the overlap filter. None of the three tests documents these defaults or tests them explicitly. If defaults change, tests could pass or fail silently for the wrong reasons. Consider adding `// Requires MIN_SHARED ≤ {shared_count}` comments at minimum.

### 4. No test for `depth = 0`
Per `build_context.rs`, siblings are only computed when `depth > 0`. A `depth=0` call should return an empty `CommunitySiblings` list regardless of community membership. This is untested here.

### 5. Tests 1 and 3 test overlapping behavior
Both Test 1 and Test 3 assert that a cross-community concept does not appear in siblings. The distinction is the mechanism (Louvain vs. manual SQL). This layering is reasonable but worth tracking — if Test 3 is rewritten with a positive assertion, the behavioral coverage between the two tests should be explicitly documented.

---

## Recommendations

1. **KEEP Test 2 as-is.** It is the cleanest test in the file.

2. **KEEP Test 1** but add:
   - A comment noting the `shared_count(4) > MIN_SHARED(3)` threshold dependency.
   - Optionally: assert that `na1`–`na4` are also absent from siblings (they are direct edges, so excluded by `direct_relation_names`, or they fail the overlap threshold — either is worth documenting).

3. **REWRITE Test 3** to close the "trivially-empty-list" ambiguity. Two paths:
   - **Minimal**: Add `assert!(siblings.is_empty(), ...)` immediately after the siblings extraction, making the intent unambiguous. Then keep the cross-alien negative check as a belt-and-suspenders guard.
   - **Stronger**: Add edges among `cn1`–`cn4` (as in Test 1's cluster structure) so that same-community candidates score above threshold, producing a non-empty siblings list. Then assert that `cross-alien` is absent from a *non-empty* list — this is the only form of the test that is truly non-trivial.

4. **Add a `depth=0` test** either here or in `community_build_context_siblings.md`'s corresponding test file, per the missing coverage gap.

5. **Consider extracting a unit test** that calls `load_community_siblings()` directly with a mocked `Connection` to provide fast, isolation-level feedback for the SQL filter path.

---

## Load-Bearing Score: 2.5/3

| Test | Score | Rationale |
|------|:---:|---|
| `sibling_candidates_restricted_to_same_community` | 1.0 | Full-value integration regression guard for T-15.7 |
| `no_community_returns_empty_siblings` | 1.0 | Only test for the no-fallback invariant; clean and precise |
| `different_community_concepts_not_siblings` | 0.5 | Valid but weakened by missing positive assertion; partially redundant with Test 1; trivially-empty result list reduces confidence |
| **Total** | **2.5/3** | |

The suite is **load-bearing but not ironclad**. Tests 1 and 2 are solid. Test 3 must be rewritten to remove the ambiguity that makes it passable for the wrong reasons. The absence of unit-level tests and threshold documentation are secondary but compounding concerns.

---
## SWE Remediation

- KEEP Test 1 (`sibling_candidates_restricted_to_same_community`): added threshold dependency comments documenting `shared_count(4) > MAENIFOLD_COMMUNITY_MIN_SHARED(default 3)` and `normalized_overlap ≈ 1.0 > MAENIFOLD_COMMUNITY_MIN_OVERLAP(default 0.4)` per P1 recommendation.
- KEEP Test 2 (`no_community_returns_empty_siblings`): no changes — cleanest test in the file.
- REWRITE Test 3 (`different_community_concepts_not_siblings`): stronger approach — added `cross-peer` concept in same community 100 sharing 4 neighbors with `cross-query` (shared_count=4, normalized_overlap=1.0), producing a non-empty siblings list. Added positive assertion (`cross-peer` must appear) alongside the existing negative assertion (`cross-alien` must not appear). This closes the trivially-empty-list ambiguity identified in the evaluation — cross-alien exclusion is now tested against a populated result set.
- Commit: 33aa547
