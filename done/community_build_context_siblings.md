---
id: community_build_context_siblings
status: done
---

# Test Evaluation: community_build_context_siblings.rs

**Traceability**: T-W1-8 | NFR-2 | FR-13.7 | FR-13.8 | FR-13.9  
**Evaluator**: Red Team (Gray — Skeptical Inquiry 🔘)  
**File**: `crates/maenifold-core/tests/community_build_context_siblings.rs`  
**Tests examined**: 2  
**Date**: 2025

---

## Summary

This file contains two integration tests for the `CommunitySiblings` section of `BuildContext`.
Both sit at the boundary layer (in-process, against a real SQLite DB) and exercise `FR-13.7`
(siblings appear in output), `FR-13.8` (scoring fields populated), and `FR-13.9` (threshold
filtering). The file is small, focused, and well-commented with FR references.

**Skeptical posture applied**: *What does each assertion actually prove? What would survive
if the implementation were subtly wrong? What boundary conditions are never reached?*

### Context: What This File Lives Next To

The community-sibling surface is covered by at least four test files:

| File | Focus |
|------|-------|
| `community_build_context_siblings.rs` (this file) | Happy-path presence + scoring; threshold filtering |
| `community_sibling_scope.rs` | Community-boundary enforcement (T-15.7) |
| `community_build_context_degradation.rs` | Empty/no-data graceful paths (FR-13.10) |
| `crates/maenifold-cli/tests/system_tools_cli_graph.rs` | CLI-layer CommunitySiblings smoke tests |

This context matters because substantial topology setup (dense cluster with 4 shared neighbors,
run `detect_and_persist`, assert same community, assert sibling appears) is replicated between
**this file's** `build_context_returns_community_siblings` and
`community_sibling_scope.rs`'s `sibling_candidates_restricted_to_same_community`.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `build_context_returns_community_siblings` | ✅ Yes | ⚠️ Partially | ⚠️ One assertion is trivially satisfied | ✅ Yes | ✅ Yes | **KEEP** (with assertion fix) |
| 2 | `build_context_sibling_thresholds_filter` | ✅ Yes | ✅ Yes | ❌ Incomplete — only one of two filters tested; off-boundary values; no env isolation | ⚠️ Misleading plural | ✅ Yes | **REWRITE** |

---

## Test 1: `build_context_returns_community_siblings`

### What it does

Builds a 6-node dense graph (sib-query and sib-target share 4 strong neighbors, no direct edge;
neighbors fully connected internally). Runs `detect_and_persist` to assign communities.
Invokes `BuildContext { conceptName: "sib-query", depth: 1 }`. Asserts:

1. `sib-target` is **not** in `DirectRelations` — because no direct edge exists.
2. `sib-target` **is** in `CommunitySiblings` (FR-13.7).
3. `NormalizedOverlap >= 0.4` for `sib-target` (FR-13.8).

### Load-Bearing?

**Yes.** This is the primary test that the detect → BuildContext pipeline produces siblings at
all. Without it, the sibling presence guarantee (FR-13.7) has no in-process boundary coverage.
The negative `DirectRelations` assertion is the only test across the entire suite that verifies
a community sibling is **excluded from** direct relations, which is a distinct correctness
requirement.

### Unique?

**Partially.** The core topology and community detection sequence is structurally replicated in
`community_sibling_scope.rs::sibling_candidates_restricted_to_same_community` — same dense
cluster, same `detect_and_persist` call, same "same community → sibling appears" assertion.
The two unique contributions of this test are:

- `!direct_names.contains(&"sib-target")` — **found nowhere else**.
- `overlap >= 0.4` — the only numerical scoring assertion (though see below).

### Correct Assertions?

**Mostly, with one trivially-satisfied assertion.**

**Assertion 1** (`not in DirectRelations`): Correct and meaningful. If the implementation
erroneously promoted siblings to direct relations, this would catch it.

**Assertion 2** (`sib-target in CommunitySiblings`): Correct.

**Assertion 3** (`overlap >= 0.4`): **Trivially satisfied.** The implementation computes
`NormalizedOverlap = overlap_numerator / sibling_total_degree`. With this setup:

```
sib-target neighbors: {sib-n1: 10, sib-n2: 10, sib-n3: 10, sib-n4: 10}
sibling_total_degree = 40
overlap_numerator    = min(10,10)*4 = 40
NormalizedOverlap    = 40/40 = 1.0
```

The computed value is **1.0**, meaning the assertion `>= 0.4` gives no signal about the
threshold boundary. An implementation with `min_overlap` accidentally set to 0.01 would still
pass this test. The assertion provides false confidence that the threshold is enforced.

**What would be better**: Assert `overlap > 0.9` to document the expected computed value, **and**
in a separate scenario, test a candidate whose normalized_overlap is just below 0.4 to verify
it gets filtered. Neither currently exists.

### Good Name?

**Yes.** `build_context_returns_community_siblings` accurately describes the happy-path assertion
and mirrors the C# oracle test name in the `Mirrors:` comment.

### Parameterizable?

**Yes.** The topology (number of shared neighbors, edge weights) is hardcoded with
`sib-` prefixed names. A table-driven approach could parameterize neighbor count and
edge weight to verify the overlap computation formula rather than just pass/fail presence.

---

## Test 2: `build_context_sibling_thresholds_filter`

### What it does

Creates 7 concepts with manual community assignments (all community 0, no `detect_and_persist`).
Sets up:
- `thresh-weak` with **2** shared neighbors with `thresh-query` (below default min_shared=3).
- `thresh-strong` with **4** shared neighbors (above threshold).

Invokes `BuildContext`. Asserts `thresh-weak` excluded, `thresh-strong` included.

### Load-Bearing?

**Yes.** This is the **only** test that exercises the `min_shared_neighbors` filter path
(FR-13.9). If this test were removed, no test would catch a regression that set the threshold
to 0 or 1.

### Unique?

**Yes.** No other test in the suite explicitly constructs a below-threshold candidate
and verifies its exclusion.

### Correct Assertions? ❌

**Incomplete in three meaningful ways:**

#### Problem 1: Boundary is not on the boundary

The implementation threshold is `shared_neighbor_count < min_shared_neighbors` (default: 3).
The test uses 2 vs 4. The exact boundary case — **exactly 3 shared neighbors** — is never
tested. An implementation that set `min_shared_neighbors = 4` would still pass all assertions
here while silently over-filtering. The test should include a concept with exactly 3 shared
neighbors and assert it **passes**, to verify the boundary is inclusive.

#### Problem 2: Only one of two filters is exercised

The implementation filters on **two independent conditions** (short-circuit `||`):

```rust
if shared_neighbor_count < min_shared_neighbors || normalized_overlap < min_overlap {
    continue;
}
```

`thresh-weak` is filtered by `shared_neighbor_count < 3` (count=2, overlap=1.0).
The `normalized_overlap < min_overlap` path is **never exercised**. A concept with
3+ shared neighbors but overlap below 0.4 would be filtered, but no test covers this.
The test name says "thresholds" (plural) but probes only one threshold.

To make this concrete: if `min_overlap` were accidentally changed from 0.4 to 0.0 in the
implementation, this test would still pass.

#### Problem 3: No environment variable isolation

The thresholds are runtime-configurable via `MAENIFOLD_COMMUNITY_MIN_SHARED` and
`MAENIFOLD_COMMUNITY_MIN_OVERLAP` environment variables. The test implicitly assumes their
defaults (3 and 0.4). If these env vars are set in the test environment (e.g., a CI matrix or
a developer's shell), the test's pass/fail semantics silently change without any error message.

No other test in the suite isolates these env vars either. This is a systemic gap.

#### Problem 4: Control assertion (`thresh-strong`) does not verify score

`thresh-strong` appears in CommunitySiblings but its `NormalizedOverlap` value is never
checked. Comparing to Test 1, which at least verifies `NormalizedOverlap >= 0.4`, this test
treats the sibling entry as a boolean presence check only.

### Good Name?

**Misleading.** `build_context_sibling_thresholds_filter` implies plural thresholds are tested,
but only the `min_shared_neighbors` gate is exercised. A more accurate name would be
`build_context_sibling_min_shared_neighbors_filters_weak_candidate`.

### Parameterizable?

**Yes, strongly.** This is precisely the kind of test that benefits from a data-driven
approach. A table of `(shared_count, expected_in_siblings)` tuples — covering 0, 1, 2, 3, 4
neighbors — would give exhaustive boundary coverage in less code than the current two-scenario
setup:

```rust
// Pseudocode sketch:
#[rstest]
#[case(0, false)]   // zero shared — filtered
#[case(1, false)]   // one shared — filtered
#[case(2, false)]   // two shared — filtered
#[case(3, true)]    // exact boundary — passes
#[case(4, true)]    // well above — passes
fn sibling_filtered_by_shared_neighbor_count(#[case] n: usize, #[case] expected: bool) {
    ...
}
```

A parallel parametric test for the `min_overlap` filter would complete the coverage.

---

## Recommendations

### Immediate (before merging changes that touch `load_community_siblings`)

1. **Strengthen Test 1's overlap assertion** — Change `overlap >= 0.4` to `overlap > 0.9`
   (the computed value with this topology is 1.0). This documents the expected computation
   rather than accepting anything above the floor threshold.

2. **Rewrite Test 2** (`build_context_sibling_thresholds_filter`) with:
   - A concept with exactly **3** shared neighbors asserting it **passes** (boundary inclusion).
   - A concept with 3+ shared neighbors but `NormalizedOverlap < 0.4` asserting it **fails**
     (covers the overlap filter path).
   - `std::env::remove_var("MAENIFOLD_COMMUNITY_MIN_SHARED")` and
     `std::env::remove_var("MAENIFOLD_COMMUNITY_MIN_OVERLAP")` guards at test start to enforce
     default-value assumptions, or explicit env-var documentation in the test comment.

### Nice-to-Have (backlog candidates)

3. **Deduplicate topology setup** — `build_context_returns_community_siblings` and
   `community_sibling_scope.rs::sibling_candidates_restricted_to_same_community` both build the
   same dense 4-neighbor cluster and run `detect_and_persist`. Consolidating the assertion
   concerns into one test per concern reduces maintenance burden.

4. **Parameterize shared-neighbor count** — Use a table-driven approach to cover 0, 1, 2, 3, 4
   shared neighbors in one test, making the threshold boundary obvious and stable against
   future threshold changes.

5. **Add an `min_overlap` boundary test** — Construct a topology where a candidate has
   exactly 3 shared neighbors but `NormalizedOverlap` is just below 0.4, verifying it is
   filtered by the overlap gate specifically.

6. **Consider `MAENIFOLD_COMMUNITY_MIN_SHARED` env-var override test** — Verify the algorithm
   respects the env var, ensuring the configuration path is exercised.

---

## Load-Bearing Score: 2/2

Both tests carry load: one owns the happy-path pipeline (FR-13.7, FR-13.8) with the unique
`DirectRelations` negative assertion; the other is the sole guardian of the threshold filter
(FR-13.9). Neither is safe to remove. Test 2, however, guards only half of the filter
predicate it claims to represent.

---

## SWE Remediation

- KEEP `build_context_returns_community_siblings`: Strengthened FR-13.8 overlap assertion from `>= 0.4` (trivially satisfied) to `> 0.9` (documents expected computation of 1.0) per P0 recommendation #1.
- REWRITE `build_context_sibling_thresholds_filter` per P0 recommendation #2:
  - Added `CommunityEnvGuard` RAII struct that clears `MAENIFOLD_COMMUNITY_MIN_SHARED` and `MAENIFOLD_COMMUNITY_MIN_OVERLAP` env vars at test start, restores on drop (follows existing `sync_perf_support.rs` pattern with `unsafe` + SAFETY comments referencing `TEST_MUTEX`).
  - Added `thresh-boundary` (exactly 3 shared neighbors) — verifies boundary is inclusive (`< 3` not `<= 3`), fixing Problem 1.
  - Added `thresh-lowoverlap` (3 shared neighbors but NormalizedOverlap ≈ 0.091 via high-weight private neighbor) — exercises the `normalized_overlap < min_overlap` filter path, fixing Problem 2.
  - Added `NormalizedOverlap > 0.9` assertion on `thresh-strong` control candidate, fixing Problem 4.
  - Env var isolation via RAII guard addresses Problem 3.
- Commit: d4140d3
