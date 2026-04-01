---
id: T-80.48
title: "Test Evaluation: db_maintenance_tests"
status: done
pipeline: tdd
---

# Test Evaluation: db_maintenance_tests
**File:** `crates/maenifold-core/tests/db_maintenance_tests.rs`
**Test count:** 7
**Evaluated:** 2026-01-10
**Evaluator color:** 🔘 Gray — Skeptical Inquiry
**Traceability:** T-QUAL-SYNC-001 | FR-25.6 ⚠️ (see §Traceability Deficit below)

---

## Summary

Seven tests covering `run_full_maintenance()` and `run_light_maintenance()` in `crates/maenifold-core/src/db/connection.rs`. The test suite is **structurally well-organized** — tests follow a consistent naming convention, use a solid `isolated_env()` harness, and are properly guarded by a `TEST_MUTEX`. However, **skeptical inquiry surfaces three material problems**:

1. **Two smoke tests are subsumed** — `ac1` and `ac4` are strictly implied by `ac2` and `ac5` respectively; they add test count without adding signal.
2. **Two "idempotency" tests claim more than they prove** — `ac6a` and `ac6b` verify "runs twice without panic," not the formal idempotency property their names assert (`f(f(x)) = f(x)`). The name is a lie.
3. **A latent CI failure exists** in `ac2` — the implementation's `journal_mode()` helper returns `"DELETE"` in CI environments, but the test unconditionally asserts `"wal"`. This test will silently never run in CI, or if it does, it will fail.

The **traceability tags are phantom** — `T-QUAL-SYNC-001` and `RTM FR-25.6` appear nowhere in `PRD.md`, `RTM.md`, or `TODO.md`. Additionally, the tests tag themselves `T-QUAL-SYNC-001.1` while the implementation they test is tagged `T-QUAL-SYNC-001.3`. These disconnects mean the requirements matrix cannot trace these tests to any open requirement.

---

## Traceability Deficit ⚠️

All 7 tests are annotated `// T-QUAL-SYNC-001.1: RTM FR-25.6`. The implementation is annotated `// T-QUAL-SYNC-001.3: RTM FR-25.6`.

**Search result:** Neither `T-QUAL-SYNC-001` nor `FR-25.6` appear in `PRD.md`, `RTM.md`, or `TODO.md`.

In the current RTM, `FR-25` is "Azure Deployment Manifests" (Dockerfile, Bicep, SRE Agent YAML, CI workflow) — it has no sub-item 6 related to database maintenance. The closest live requirement is `NFR-2` (each tool SHALL have boundary/CLI/parity test coverage) or potentially a future `T-16.6` (storage abstraction tests). Database maintenance functions exist in `db/connection.rs` and are used by the `RunFullMaintenance`/`RunLightMaintenance` tools, suggesting these tests belong under the tool's own RTM entry, not a phantom `FR-25.6`.

**Action required:** Reclassify these tests to a valid T-* and RTM reference. Sub-item numbers in tests (`001.1`) must match implementation (`001.3`), or explain the split.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `fr25_6_ac1_database_maintenance_run_full_executes_without_error` | MERGE | Subsumed by `ac2`; `ac2` already verifies successful execution plus a behavioral postcondition. Zero independent signal. |
| `fr25_6_ac2_database_maintenance_run_full_restores_wal_journal_mode` | KEEP ⚠️ | Load-bearing critical invariant, but has a latent CI failure: `journal_mode()` returns `"DELETE"` in CI, so the `"wal"` assertion will fail there. Needs CI guard. |
| `fr25_6_ac3_database_maintenance_run_full_reclaims_space` | KEEP | Core behavioral contract. Assertion logic is directionally correct but the second assertion (`size_after_full < size_with_data`) should compare against `size_after_delete`, not `size_with_data`, to verify actual reclamation of freed pages rather than just "smaller than full DB." |
| `fr25_6_ac4_database_maintenance_run_light_executes_without_error` | MERGE | Subsumed by `ac5`; `ac5` calls `.expect()` which would panic on error. No independent signal. |
| `fr25_6_ac5_database_maintenance_run_light_does_not_vacuum` | KEEP | Important negative contract: proves light ≠ full (no VACUUM). 90% threshold is inconsistent with `ac3`'s 80%; standardize. |
| `fr25_6_ac6a_database_maintenance_run_full_is_idempotent` | REWRITE | Name claims `f(f(x)) = f(x)`; test only verifies `f(f(x))` doesn't panic. No state comparison exists. Either add DB-state assertions after each call to prove equivalence, or rename to `_can_be_called_twice_without_error`. |
| `fr25_6_ac6b_database_maintenance_run_light_is_idempotent` | REWRITE | Same failure as `ac6a`. Light maintenance is a WAL checkpoint + FTS optimize — truly idempotent by construction, but the test doesn't demonstrate it. Add assertions. |

---

## Detailed Findings

### Finding 1 — `ac1` is subsumed by `ac2` (MERGE)

`ac1` asserts only `run_full_maintenance(&mut conn).is_ok()`. `ac2` calls the same function, also asserts success via `.expect()`, *and* additionally checks the WAL postcondition. If `ac2` passes, `ac1` trivially passes. Merging removes dead weight without sacrificing any signal.

**Merge target:** absorb `ac1` into `ac2` — `ac2` already proves execution succeeds.

### Finding 2 — `ac2` has a latent CI failure (KEEP with fix)

`connection.rs` contains:
```rust
fn journal_mode() -> &'static str {
    if std::env::var("CI").ok().as_deref() == Some("true")
        || std::env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true")
    {
        "DELETE"  // WAL disabled in CI
    } else {
        "WAL"
    }
}
```

`ac2` unconditionally asserts:
```rust
assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
```

In any CI environment where `CI=true` or `GITHUB_ACTIONS=true`, the connection opens in `DELETE` mode. The assertion will **fail** or the test will never actually validate WAL restoration in CI. Either the test must be `#[cfg_attr(ci, ignore)]` annotated, or the assertion must be conditional:
```rust
let expected = if std::env::var("CI").ok().as_deref() == Some("true") { "delete" } else { "wal" };
assert_eq!(journal_mode.to_ascii_lowercase(), expected);
```

### Finding 3 — `ac3` second assertion is weak (KEEP with note)

The space-reclamation test verifies:
```rust
// After VACUUM:
assert!(size_after_full < size_with_data, ...);
```

This asserts the post-VACUUM file is smaller than the file *at peak data load*. That's trivially true — any reclamation passes this bar, including minimal reclamation. The stronger and more meaningful assertion would be:

```rust
// VACUUM should return the file close to its pre-bulk-insert baseline
assert!(
    size_after_full < size_after_delete,
    "RunFull should reclaim freed pages; VACUUM should shrink below post-delete size"
);
```

`size_after_delete` captures the state where rows are logically gone but pages are still occupied. If `size_after_full < size_after_delete` doesn't hold, VACUUM didn't actually reclaim anything.

### Finding 4 — `ac4` is subsumed by `ac5` (MERGE)

Identical argument to Finding 1. `ac5` already calls `run_light_maintenance(&conn).expect("run light maintenance")` which panics on error, covering the smoke-test signal of `ac4`.

### Finding 5 — Threshold inconsistency between `ac3` and `ac5`

`ac3` uses 80% as the lower bound for "file hasn't shrunk yet":
```rust
size_after_delete >= ((size_with_data as f64) * 0.8) as u64
```

`ac5` uses 90%:
```rust
size_after_light >= ((size_with_data as f64) * 0.9) as u64
```

There is no documented rationale for the difference. If 80% is the right tolerance for SQLite free-page behavior, use it consistently. If 90% is tighter and more correct, use that. Pick one threshold with a comment explaining it.

### Finding 6 — `ac6a`/`ac6b` name-assertion mismatch (REWRITE)

Idempotency is formally: `f(f(x)) = f(x)`. Both tests prove only that `f(f(x))` terminates without error. The database state after two runs is never inspected. For `run_full_maintenance` specifically, the function closes and reopens the connection internally — calling it twice on an already-VACUUM'd database should be safe, and this is worth *proving* by capturing the page count or file size:

```rust
// After first run:
let page_count_1: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap();
run_full_maintenance(&mut conn).expect("second run full");
let page_count_2: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap();
assert_eq!(page_count_1, page_count_2, "second VACUUM should not change page count");
```

Until this is added, the tests should be renamed to `_can_be_called_twice_without_error`.

---

## Positive Observations

- **Test isolation is solid.** `isolated_env()` creates a fresh `TempDir`, overrides `Config`, and `TestEnv::Drop` calls `Config::reset_overrides()`. No test bleeds into another.
- **`TEST_MUTEX` is correct.** WAL file-size measurements require exclusive filesystem state; the mutex prevents flaky concurrent ordering.
- **Helper functions are well-factored.** `insert_bulk_data`, `delete_bulk_data`, `checkpoint_and_sync`, `get_db_file_size` are reusable and clearly named. The `open_wal_connection` helper makes WAL preconditions explicit.
- **Naming convention is consistent and informative.** `fr25_6_acN_<operation>_<postcondition>` is readable and grep-able by AC number.
- **The `ac3` setup correctly models the real VACUUM use case.** Inserting 4KB rows × 200, forcing a WAL checkpoint, then deleting gives VACUUM meaningful work to do. This isn't a toy scenario.

---

## Recommendations

**Priority 1 — Fix latent CI failure (ac2)**
Add a conditional expected value or CI guard to `ac2` so it doesn't silently fail or skip the most critical invariant in CI.

**Priority 2 — Fix phantom traceability**
`T-QUAL-SYNC-001` and `RTM FR-25.6` are not in any of `PRD.md`, `RTM.md`, or `TODO.md`. Reclassify to a real T-* item. Sync sub-item numbers between tests (`001.1`) and implementation (`001.3`).

**Priority 3 — Rewrite ac6a / ac6b**
Add state-comparison assertions to substantiate the idempotency claim, or rename to accurately describe what is actually tested.

**Priority 4 — Merge ac1 into ac2, ac4 into ac5**
Remove the subsumed smoke tests. Net result: 5 tests with no signal loss, lower maintenance burden.

**Priority 5 — Standardize threshold and strengthen ac3 assertion**
Align the 80%/90% thresholds with a single documented rationale. Change `ac3`'s second assertion to compare against `size_after_delete`.

---

## Load-Bearing Score: 3/7

| Test | Load-Bearing? | Rationale |
|---|---|---|
| `ac1` | ❌ No | Subsumed by `ac2` |
| `ac2` | ✅ Yes | Only test verifying WAL-restoration postcondition — critical invariant |
| `ac3` | ✅ Yes | Only test proving VACUUM executes and reclaims space |
| `ac4` | ❌ No | Subsumed by `ac5` |
| `ac5` | ✅ Yes | Only test proving light maintenance does NOT VACUUM — negative contract |
| `ac6a` | ❌ Marginal | Proves "no panic on second call" but not idempotency proper |
| `ac6b` | ❌ Marginal | Same as `ac6a` |

3 tests carry genuine behavioral signal. 2 are subsumed. 2 claim more than they prove.

---

## SWE Remediation

- MERGE ac1 → ac2: removed subsumed smoke test; ac2 already proves execution + postcondition
- KEEP ac2: P1 — added `expected_journal_mode()` helper mirroring `connection.rs::journal_mode()` CI logic; assertion now environment-conditional (WAL outside CI, DELETE in CI)
- KEEP ac3: P5 — strengthened second assertion to compare `size_after_full < size_after_delete` (proves VACUUM reclaimed freed pages, not just "smaller than peak"); standardized threshold to shared `NO_VACUUM_RETENTION_RATIO` (0.9)
- MERGE ac4 → ac5: removed subsumed smoke test; ac5 already proves execution + postcondition
- KEEP ac5: P5 — standardized threshold to shared `NO_VACUUM_RETENTION_RATIO` (0.9) with documented rationale
- REWRITE ac6a: added page_count + freelist_count state comparison after each run to prove f(f(x)) = f(x); setup creates bulk data then deletes it so first VACUUM has meaningful work
- REWRITE ac6b: added page_count comparison with checkpoint_and_sync after each call; setup inserts FTS5 content so first optimize has real work
- P2 traceability: reclassified all tests from phantom `T-QUAL-SYNC-001.1: RTM FR-25.6` to `T-DB-MAINT-001: RTM NFR-2`; added merge-tombstone comments for removed tests
- Test count: 7 → 5 (2 subsumed smoke tests removed, 0 signal loss)
- Commit: 592df81
