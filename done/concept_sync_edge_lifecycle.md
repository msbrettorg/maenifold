---
id: concept_sync_edge_lifecycle
status: done
pipeline: tdd
---

# Test Evaluation: concept_sync_edge_lifecycle

**File**: `crates/maenifold-core/tests/concept_sync_edge_lifecycle.rs`
**Evaluator**: Red Team (Gray — Skeptical Inquiry)
**Traceability**: T-5.7, T-12.7 · FR-14.3
**Related files**: `concept_sync_edge_stability.rs`, `concept_sync_edge/support.rs`

---

## Summary

This file is one half of a deliberate split introduced by T-12.7: stability-oriented regressions live in `concept_sync_edge_stability.rs`; lifecycle-oriented regressions live here. The split is justified — the two files test orthogonal concerns (rowid stability / weight fidelity vs. contribution add/remove/delete). With 3 tests, this is a lean file that earns its keep.

All three tests exercise the incremental sync engine's edge mutation logic against a real SQLite database in a temporary directory. They are integration-level, not unit-level — they call `sync_single_file` end-to-end and assert on raw DB state. That is intentional and appropriate for regression-preventing coverage of FR-14.3.

**Critical structural concern**: every test acquires `TEST_MUTEX` because `Config::override_root()` and `Config::set_database_path()` write to process-global state, making tests inherently serial despite the per-test `TempDir` isolation. This is a design smell in the test harness, not in the tests themselves, but it means any future parallel test execution will silently break this entire file if the mutex is removed.

**Secondary concern**: all three tests call `set_last_indexed(..., "2000-01-01T00:00:00+00:00")` to defeat the mtime-based sync cache. This is tight coupling to an implementation detail of the cache-invalidation strategy. If that strategy changes (e.g., content-hash instead of timestamp), every test here silently stops exercising a real re-sync.

**Assertion gap in test 1**: `concept_removed_cleans_stale_edge_contributions` checks that the removed concept's edges *lose the file's contribution* (`!contains_key(&uri)`) but never asserts that those edges are *fully deleted* when the file was the sole contributor. In this isolated config, with one file, that is the only valid state — a zombie row would be a real bug that test 1 would silently miss. Test 3 covers full-row deletion, but only for a simpler two-concept setup. The gap is partially compensated, not fully closed.

**Missing weight assertions in tests 1 and 2**: neither test verifies the `co_occurrence_count` or per-file weight of any edge. The stability file (`concept_sync_edge_stability.rs`) *does* check weights. This asymmetry is acceptable if lifecycle tests are intentionally concerned only with structural presence/absence, but it should be a conscious decision, not an oversight.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `concept_removed_cleans_stale_edge_contributions` | ✅ Yes | ✅ Yes | ⚠️ Partial — misses `is_none()` for sole-contributor edges | ✅ Yes | ⚠️ With effort | **REWRITE** (assertion strengthening) |
| 2 | `new_concept_added_creates_new_edge_rows` | ✅ Yes | ✅ Yes | ⚠️ Partial — no weight assertions on new edges | ✅ Yes | ⚠️ With effort | **KEEP** (minor gap, low risk) |
| 3 | `last_contributor_removed_deletes_edge_row` | ✅ Yes | ✅ Yes | ✅ Yes — `is_none()` is correct and sufficient | ✅ Yes | ✅ Yes | **KEEP** |

---

## Detailed Findings

### Test 1 — `concept_removed_cleans_stale_edge_contributions`

**What it tests**: Sync a file with `[[alpha]] [[beta]] [[gamma]]`, then re-sync with `[[alpha]] [[beta]]` only. Verifies that:
- `alpha–beta` retains the file's contribution.
- `alpha–gamma` and `beta–gamma` lose the file's contribution.

**What's correct**:
The assertion pattern `!query_edge(...).map(|e| e.1.contains_key(&uri)).unwrap_or(false)` is logically sound — it returns `true` whether the edge row was deleted entirely or just lost this file's entry. This correctly expresses "this file must not contribute to this edge."

The surviving-edge assertion (`alpha-beta` still `contains_key`) is correctly placed as a positive guard to ensure the partial-removal logic does not over-delete.

**What's missing**:
This is an isolated config with exactly one file. After the re-sync, `alpha–gamma` and `beta–gamma` have no contributors. The edge rows should be gone. The assertions do not verify this. A bug where stale zero-contributor rows linger would pass test 1 unchallenged.

Recommended addition for each removed edge:
```rust
assert!(
    query_edge(&cfg, "alpha", "gamma").is_none(),
    "alpha-gamma row should be fully deleted — no contributors remain"
);
```

**Additional gap**: No assertion on the weight/count of `alpha-beta` after gamma is removed. If the re-sync accidentally inflates or deflates the weight, this test misses it.

**Verdict: REWRITE** — add `is_none()` assertions for the two removed edges; optionally add weight assertion for `alpha-beta`. The test is structurally sound; it just stops short of the assertion it should make.

---

### Test 2 — `new_concept_added_creates_new_edge_rows`

**What it tests**: Sync with `[[alpha]] [[beta]]`, then re-sync with `[[alpha]] [[beta]] [[gamma]]`. Verifies that:
- `alpha–gamma` and `beta–gamma` are created and reference the file.
- `alpha–beta` is preserved.

**What's correct**:
The three assertions cover the three structural outcomes exactly. `contains_key` on the new edges, `contains_key` on the preserved edge. The `.expect(...)` messages are informative.

**What's missing**:
No weight assertions for the newly created edges. `contains_key(&uri)` only proves the row references this file; it doesn't verify the actual weight value stored. If the section-proximity weight is miscalculated on creation, this test passes. The stability file checks weights explicitly — this test should follow suit for at least one new edge.

**Risk level**: Low. Weight bugs on creation would likely surface in other tests (stability tests, section tests). The structural check is the primary concern for a *lifecycle* test. The gap is real but not alarming.

**Verdict: KEEP** — consider adding a weight assertion for `alpha_gamma.1.get(&uri)` to close the gap, but current coverage is acceptable.

---

### Test 3 — `last_contributor_removed_deletes_edge_row`

**What it tests**: Sync with `[[alpha]] [[beta]]`, then re-sync with `[[alpha]]` only. Verifies the edge row is fully deleted when no contributors remain.

**What's correct**:
`query_edge(&cfg, "alpha", "beta").is_none()` is the right assertion — it tests full row deletion, not just contribution removal. This is the cleanest test in the file.

**What's correct and notable**: This test is the tightest in the file. Single concept change, single assertion, clear expectation. It could serve as a template for the others.

**What's missing**: Nothing critical. The test could also verify that the `alpha` concept node itself is not erroneously deleted (it still appears in the file), but that's a different concern (concept lifecycle, not edge lifecycle).

**Verdict: KEEP** — no changes needed.

---

## Recommendations

### P1 — Strengthen test 1 assertions (correctness risk)

**Action**: Add `is_none()` assertions for `alpha–gamma` and `beta–gamma` in `concept_removed_cleans_stale_edge_contributions`. In an isolated single-file config these rows must be fully deleted. The current assertions are weaker than they should be and would silently pass on a zombie-row bug.

```rust
// After the existing !contains_key assertions, add:
assert!(
    query_edge(&cfg, "alpha", "gamma").is_none(),
    "alpha-gamma row must be deleted when its only contributor removes the concept"
);
assert!(
    query_edge(&cfg, "beta", "gamma").is_none(),
    "beta-gamma row must be deleted when its only contributor removes the concept"
);
```

### P2 — Document the `set_last_indexed` coupling (fragility risk)

**Action**: Add a comment above each `set_last_indexed` call explaining *why* it is needed. If the cache strategy ever changes to content-hash, these backdating calls become no-ops that silently stop exercising re-sync. A comment creates a forcing function for future maintainers:

```rust
// Force re-sync: the mtime cache skips unchanged files. Backdating last_indexed
// makes the engine treat this file as stale on the next sync_one() call.
// If the cache strategy changes to content-hash, update this call site.
set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
```

### P3 — Evaluate global-mutex necessity (design smell)

**Action**: Determine whether `TEST_MUTEX` can be eliminated. `isolated_config()` creates a TempDir and sets global config overrides; `ConfigOverrideReset` cleans up on drop. If `Config::override_root` / `Config::set_database_path` were thread-local rather than process-global, the mutex would be unnecessary and tests could run in parallel. This is a harness-level improvement but affects the entire concept-sync test suite.

### P4 — Consider parameterizing the three lifecycle operations (DRY, optional)

The three tests share an almost identical scaffold:
1. Acquire mutex + reset guard + isolated config
2. Write initial fixture + sync
3. Backdate + write updated fixture + re-sync
4. Assert structural outcomes

They differ only in: fixture file name, initial concepts, final concepts, and assertions. A table-driven or macro-driven approach would remove ~60% of duplicated setup code. This is a readability improvement, not a correctness one. Given the project's existing test style, this may not be worth the complexity — flag as optional.

---

## Load-Bearing Score: 3/3

All three tests are load-bearing regression guards. Each covers a distinct branch of the edge lifecycle:

- **Test 1**: Partial concept removal — contributions pruned, surviving edges preserved
- **Test 2**: Concept addition — new edges created without disturbing existing ones
- **Test 3**: Complete removal — edge row deleted when sole contributor drops the pair

No test duplicates another. Removing any one of them would leave a real regression path uncovered. The suite is lean and justified.

The file's primary deficit is assertion depth in test 1 (P1), not test coverage breadth. Fix P1, document P2, and this file is solid.
