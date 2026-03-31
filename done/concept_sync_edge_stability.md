---
id: concept_sync_edge_stability
status: done
---

# Test Evaluation: concept_sync_edge_stability

**File:** `crates/maenifold-core/tests/concept_sync_edge_stability.rs`
**Traceability:** T-5.7, T-12.7 · FR-14, FR-14.3 · RTM (see T-5.7 / T-12.7 references)
**Support module:** `crates/maenifold-core/tests/concept_sync_edge/support.rs`
**Evaluator perspective:** 🔘 Gray — Skeptical Inquiry (demand evidence, question assumptions)

---

## Summary

This file contains **two integration tests** for edge-stability invariants in the incremental concept-sync pipeline. Both are regression guards for `rebuild_affected_graph_from_mentions` in `sync/graph_state.rs`. The file was split from a larger concept-sync edge suite (T-12.7) and is thematically coherent: both tests ask "what happens to SQLite `rowid`s when a file is re-synced?"

The production mechanism being exercised:
- **`should_write` guard** (line 82, `graph_state.rs`): skips the DB write entirely when `co_occurrence_count` and `source_files` JSON are identical — preserving the rowid by doing nothing.
- **`INSERT … ON CONFLICT DO UPDATE SET`** (lines 86–93, `graph_state.rs`): fires when data *does* change — preserves rowid via in-place UPDATE, not DELETE+INSERT.

Both tests are **load-bearing** — they guard real behavioral invariants that have no 1:1 equivalent in the lifecycle file. However, each carries at least one **materially weak assertion** and each has at least one **coverage gap** that the current assertions do not reach.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Clear? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `identical_resync_preserves_existing_edge_rows` | ✅ Yes | ⚠️ Partial | ⚠️ Weak intermediate | ✅ Yes | ✅ Yes | **REWRITE** |
| 2 | `section_change_rewrites_only_affected_edges` | ✅ Yes | ✅ Yes | ❌ Missing assertions + misleading comment | ✅ Yes | ✅ Yes | **REWRITE** |

---

## Detailed Findings

---

### `identical_resync_preserves_existing_edge_rows`

#### What it claims to test
When a file is re-indexed with identical WikiLink structure (only prose added between `[[alpha]] [[beta]]` and `[[gamma]]`), all three `concept_graph` rowids must be unchanged.

#### What it actually exercises
The `should_write = false` code path in `rebuild_affected_graph_from_mentions` — no DB write occurs for any of the three edges because both `co_occurrence_count` and `source_files` JSON are byte-for-byte identical. The rowid is preserved by *doing nothing*.

#### Issues found

**Issue 1 — Weak intermediate assertion (substantive)**
```rust
assert!(
    resync.contains("concept mentions updated"),
    "resync should process file content changes: {resync}"
);
```
The format string in `sync/mod.rs:220` is:
```
"Synced {file_uri}: {mention_count} concept mentions updated, {file_filtered} filtered, {relation_count} affected relations rebuilt"
```
The substring `"concept mentions updated"` appears in *every* successful sync output — including `"0 concept mentions updated"`. This assertion passes if zero concepts were updated and zero relations were rebuilt. It confirms only that `sync_single_file` returned `Ok(...)`, which is already proven by `.expect("sync single file")` in the support's `sync_one()`. The assertion adds no diagnostic value.

**What it should say:** The resync modified prose but preserved wikilinks, so mentions *were* processed (the file content changed). A stronger check would assert the specific URI is present and that `"0 affected relations rebuilt"` appears (confirming the `should_write = false` path was actually taken for all edges):
```rust
assert!(
    resync.contains("memory://concept-sync/file-a") && resync.contains("0 affected relations rebuilt"),
    "resync of prose-only change must not rebuild any graph edges: {resync}"
);
```
Or at minimum assert the full normalized message prefix.

**Issue 2 — Missing weight correctness verification**
The test captures and validates rowids for all three edges but never calls `query_edge` to verify weights are still correct after the resync. If the sync had a bug that reset weights to 0.0 while preserving rowids, all three rowid assertions would pass. The test title says "preserves existing edge rows" — a complete row check requires both the identifier (rowid) and the data (weight).

**Issue 3 — Partial overlap with `sync_perf_graph.rs`**
`build_graph_edges_resync_does_not_duplicate_edges_co_occurrence_count_correct` covers the same `should_write = false` invariant for one edge pair, additionally checking `query_edge_row_count == 1`. The stability test adds value by covering three pairs simultaneously (including cross-section edges), but the overlap should be noted in comments to help future maintainers understand which file owns which angle.

**What is done well:**
- The initial sync assertion `first_sync.contains("Synced memory://concept-sync/file-a")` correctly checks the URI-specific substring — this is a meaningful discriminating check.
- The `BTreeMap` of rowids is a clean pattern for verifying all three pairs in a loop.
- Isolation via `isolated_config()` + `TEST_MUTEX` is correct and necessary.

---

### `section_change_rewrites_only_affected_edges`

#### What it claims to test
When beta moves from Section One to Section Two, the alpha-gamma edge (unaffected by the move) retains its rowid; alpha-beta weight drops to 0.5; beta-gamma weight rises to 1.0.

#### What it actually exercises
Two distinct code paths:
- **`should_write = false` path** for alpha-gamma — because both `co_occurrence_count` (1.0) and `source_files` JSON (`{uri: 0.5}`) are unchanged after the section move.
- **`ON CONFLICT DO UPDATE` path** for alpha-beta and beta-gamma — because `source_files` JSON changes (`{uri: 1.0}` → `{uri: 0.5}` and `{uri: 0.5}` → `{uri: 1.0}` respectively). The rowid is preserved *by SQLite UPSERT mechanics*, not by skipping the write.

#### Issues found

**Issue 1 — Misleading comment (correctness risk)**
```rust
assert_eq!(
    query_edge_rowid(&cfg, "alpha", "gamma"),
    Some(alpha_gamma_rowid_before),
    "alpha-gamma should keep its rowid because the edge weight stayed unchanged"
);
```
The comment claims the rowid is preserved *because the weight is unchanged*. This is wrong at two levels:

1. The rowid is preserved because the `should_write = false` guard fires — specifically because *both* `co_occurrence_count` AND `source_files` are unchanged. Weight is only one component.
2. More importantly: even if the weight *had* changed, the rowid would still be preserved via `ON CONFLICT DO UPDATE`. The comment implies a broken invariant — "if weight changes, rowid changes" — that the production code does *not* have. A future developer reading this comment will misunderstand the contract.

**Correct comment:** `"alpha-gamma keeps its rowid because co_occurrence_count and source_files are identical — the should_write guard fires false, no DB write occurs"`

**Issue 2 — `ON CONFLICT DO UPDATE` rowid preservation is never verified (coverage gap)**
The test saves only `alpha_gamma_rowid_before` (the unchanged-edge path). It does **not** save `alpha_beta_rowid_before` or `beta_gamma_rowid_before`. The `ON CONFLICT DO UPDATE` code path (lines 86–93 in `graph_state.rs`) is the mechanism that preserves rowids for *changed* edges. If this path were accidentally replaced with `INSERT OR REPLACE` — which would delete and reinsert, assigning a new rowid — this test would not catch the regression.

To close this gap:
```rust
let alpha_beta_rowid_before = query_edge_rowid(&cfg, "alpha", "beta").expect("alpha-beta rowid");
let beta_gamma_rowid_before = query_edge_rowid(&cfg, "beta", "gamma").expect("beta-gamma rowid");
// ... (after resync) ...
assert_eq!(
    query_edge_rowid(&cfg, "alpha", "beta"),
    Some(alpha_beta_rowid_before),
    "alpha-beta rowid must be preserved via ON CONFLICT DO UPDATE even when weight changes"
);
assert_eq!(
    query_edge_rowid(&cfg, "beta", "gamma"),
    Some(beta_gamma_rowid_before),
    "beta-gamma rowid must be preserved via ON CONFLICT DO UPDATE even when weight changes"
);
```

**Issue 3 — Missing alpha-gamma weight verification**
The test verifies that the alpha-gamma rowid is preserved but never asserts that the alpha-gamma weight is still 0.5 after the section change. If the incremental rebuild had a bug that corrupted the alpha-gamma source_files while preserving the rowid (e.g., via the `ON CONFLICT DO UPDATE` firing spuriously), this test would pass while behavioral parity was broken.

Add:
```rust
let alpha_gamma = query_edge(&cfg, "alpha", "gamma").expect("alpha-gamma edge after section change");
assert_eq!(
    alpha_gamma.1.get(&uri).copied(),
    Some(0.5),
    "alpha-gamma weight must remain 0.5 — section move did not affect this pair"
);
```

**Issue 4 — Weak intermediate assertion**
```rust
assert!(
    output.contains("affected relations rebuilt"),
    "section-change sync should complete: {output}"
);
```
Same problem as Test 1: this substring appears in every successful sync output. "0 affected relations rebuilt" matches. A section change should rebuild exactly 2 or 3 relations. Asserting `"2 affected relations rebuilt"` (alpha-beta + beta-gamma) or `"3 affected relations rebuilt"` (if alpha-gamma counts) would confirm the incremental path took the right shape.

**What is done well:**
- The alpha-beta and beta-gamma weight assertions are numerically precise and correct for the section semantics (`1.0` same-section, `0.5` cross-section per `graph_relations.rs:150–153`).
- The test exercises a genuine edge case: a three-concept set where one concept moves sections, creating a "wedge" that increases one pair's proximity while decreasing another's.
- Isolation is correct.

---

## Recommendations

### Priority 1 — Rewrite both tests to close the identified gaps

**Test 1 (`identical_resync_preserves_existing_edge_rows`):**
- Replace `resync.contains("concept mentions updated")` with a check that `"0 affected relations rebuilt"` appears in the resync output, confirming the `should_write = false` path for all edges.
- Add `query_edge` assertions for all three pairs after resync to verify weights are preserved alongside rowids.

**Test 2 (`section_change_rewrites_only_affected_edges`):**
- Capture rowids for alpha-beta and beta-gamma *before* the section change, and assert they are preserved after — this closes the `ON CONFLICT DO UPDATE` coverage gap.
- Add `query_edge` assertion that alpha-gamma weight remains 0.5.
- Fix the comment: remove the false causal claim "because the edge weight stayed unchanged" and replace with the correct explanation referencing the `should_write` guard.
- Strengthen the intermediate `output.contains(...)` assertion to check for the expected relation count.

### Priority 2 — Extract a third test for the ON CONFLICT DO UPDATE path

The `ON CONFLICT DO UPDATE` rowid-preservation path (changed-edge, preserved rowid) is currently implicit in test 2 but never explicitly asserted. Consider a focused test:

```rust
fn changed_edge_weight_preserves_rowid_via_upsert() { ... }
```

This would explicitly document and guard the `INSERT … ON CONFLICT DO UPDATE` contract, separate from the `should_write = false` invariant.

### Priority 3 — Add cross-reference comments to `sync_perf_graph.rs`

Test 1 partially overlaps with `build_graph_edges_resync_does_not_duplicate_edges_co_occurrence_count_correct`. Add a `// See also:` comment in each file so maintainers understand the intended split: perf file owns "no duplicate rows + correct count," stability file owns "multi-edge rowid preservation across section types."

### Priority 4 — Consider parameterization

Both tests use hard-coded content strings. The section-change invariant applies to many possible movement patterns (A→B move, B→C move, cross-file merge, etc.). A table-driven structure over `(initial_content, resync_content, expected_weights)` would multiply coverage without proportionally increasing maintenance cost.

---

## Load-Bearing Score: 2/2

Both tests guard real, non-trivial production invariants with no direct duplicate elsewhere:

| Test | Core Invariant | Code Path Guarded | Unique? |
|------|---------------|-------------------|---------|
| `identical_resync_preserves_existing_edge_rows` | Prose-only resync must not touch graph rows | `should_write = false` for 3 pairs including cross-section | ✅ (perf file covers 1 pair, same section only) |
| `section_change_rewrites_only_affected_edges` | Section move recalculates weights correctly; unchanged edge row not rewritten | `should_write = false` + `ON CONFLICT DO UPDATE` mixed path | ✅ (lifecycle file tests existence/deletion, not weights or rowid) |

Both should be rewritten to close the identified assertion gaps before being trusted as regression guards. Neither should be removed.
