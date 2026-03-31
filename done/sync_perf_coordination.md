---
id: sync_perf_coordination
status: done
---

# Test Evaluation: sync_perf_coordination

**File**: `crates/maenifold-core/tests/sync_perf_coordination.rs`
**Evaluated by**: Red Team (Gray — Skeptical Inquiry)
**Traceability**: T-0.1 (as annotated in file), NFR-3 (No Regressions)
**Total tests**: 2

---

## Summary

`sync_perf_coordination.rs` contains exactly two `#[test]` functions. Both carry the annotation `// T-0.1: RTM NFR-3` — the weakest possible traceability tag ("no regressions"), which says nothing about *what* the tests verify. The file is named `sync_perf_coordination`, but neither test has any meaningful relationship to sync *performance* or concurrent *coordination*. They appear to be stragglers: tests for session-abandonment mtime guards and leader-election lifecycle that were written against the `sync_perf_support.rs` harness when those features were first being built, and were never migrated to the proper purpose-built test suites.

### What the tests cover

| Test | Domain | Real Coverage |
|---|---|---|
| `sweep_abandoned_thinking_sessions_mtime_guard_protects_recently_edited_sessions` | Session abandonment cleanup | The `mtime` guard in `sessions::cleanup_abandoned_sessions` — if filesystem mtime is recent, session is shielded even if `last_indexed` is stale |
| `try_acquire_lock_atomic_behavior_held_returns_false_released_returns_true` | Leader election | acquire → block → release → reacquire lifecycle with epoch increment |

### Infrastructure concerns

Both tests use `sync_perf_support.rs` — a large general-purpose harness. The dedicated `session/session_cleanup_support.rs` and `support/sync_leader.rs` harnesses exist precisely for these domains and include richer assertion utilities (`read_db_status`, `current_lease`, `short_sleep`, etc.) that are absent here. Using the wrong support file is a maintenance liability: future changes to session cleanup or leader election semantics will be found in one harness but not the other.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Good? | Parameterizable? | **Verdict** |
|---|---|---|---|---|---|---|---|
| 1 | `sweep_abandoned_thinking_sessions_mtime_guard_protects_recently_edited_sessions` | ✅ YES | ✅ YES | ⚠️ PARTIAL | ✅ YES | ✅ YES | **REWRITE** |
| 2 | `try_acquire_lock_atomic_behavior_held_returns_false_released_returns_true` | ❌ NO | ❌ NO | ⚠️ MISLEADING | ❌ NO | N/A | **REMOVE** |

---

## Detailed Findings

### Test 1 — `sweep_abandoned_thinking_sessions_mtime_guard_protects_recently_edited_sessions`

#### Load-Bearing?
**YES.** The mtime guard for session abandonment is not exercised in `session_cleanup_sweep.rs` or `session_cleanup_ttl.rs`. A grep across the full test suite confirms: `set_file_mtime` / `FileTime` are used in incremental-sync mtime-skip tests (`sync_mtime_opt_guards.rs`, `sync_mtime_skip.rs`) and workflow supervisor cleanup tests — but NOT in any dedicated session cleanup path. Remove this test and the mtime guard in `cleanup_abandoned_sessions` becomes untested.

#### Unique?
**YES.** The scenario (stale `last_indexed` + recent filesystem mtime → session survives; stale `last_indexed` + stale filesystem mtime → session abandoned) is covered nowhere else. It validates a subtle two-factor abandonment condition: *both* the indexed timestamp AND the filesystem mtime must be stale for abandonment to proceed.

#### Assertions Correct?
**Partially.** The test correctly checks:
- `abandoned_a == 0` after recent mtime
- `frontmatter_status == "active"` after recent mtime
- `abandoned_b == 1` after stale mtime
- `frontmatter_status == "abandoned"` after stale mtime

**Missing:** The `session_cleanup_ttl.rs` tests always verify `read_db_status` in addition to `read_frontmatter_status`, because `cleanup_abandoned_sessions` must update *both* the markdown frontmatter and the `file_content.status` DB column. This test does NOT assert the DB status after abandonment, creating a gap where a regression that abandons the markdown but not the DB would go undetected.

#### Name Good?
**YES** — verbose, but accurately describes the guard being tested.

#### Parameterizable?
**YES.** The test is a compound test verifying two scenarios sequentially. This violates single-responsibility and makes failure attribution harder: if the stale-mtime path fails, the test name says "mtime_guard_protects" which implies the *protection* path failed when it was actually the *abandonment* path. The two magic constants (`Duration::hours(2)`) should be symbolic.

#### Compound test problem
The test mutates the same session through two state transitions within one function. This is not inherently wrong — it does create a coherent story — but it means a failure in phase 2 may produce a confusing diagnostic because the phase 1 assertion is entangled.

---

### Test 2 — `try_acquire_lock_atomic_behavior_held_returns_false_released_returns_true`

#### Load-Bearing?
**NO.** `sync_leader_lifecycle.rs::fr30_3_ac6_release_succeeds_for_current_holder` (line 124) covers the exact scenario:
1. Acquires as `holder-a`
2. Releases
3. Asserts `try_acquire("holder-b") == (true, "acquired", epoch + 1)`

`sync_leader_lifecycle.rs::fr30_3_ac3_try_acquire_fails_when_lease_active` covers the blocking assertion (`held_by_other`). `leader_election_boundary.rs` provides a third layer. Removing this coordination test changes nothing about the coverage posture of `LeaderElection`.

#### Unique?
**NO.** Both component behaviors (blocking a second acquirer, release enabling reacquire) are tested more rigorously in the lifecycle suite, with explicit database state assertions via `current_lease()` that this test lacks.

#### Assertions Misleading?
**YES.** The normalization preamble at the top is the chief problem:

```rust
// --- Normalization preamble ---
if !election.try_acquire("holder-a", 10).expect("normalize").0 {
    panic!("lock was unexpectedly held at test start");
}
assert!(election.release("holder-a", 1).expect("normalize release"));
```

In a fresh `TempDir`, the database starts empty. `try_acquire` is guaranteed to succeed with epoch 1 on first call. The `if !...` branch can never fire. The immediate release burns epoch 1, so all subsequent assertions are on epoch 2 and 3 — which is confusing to a reader who expects to start from epoch 1. The normalization *looks* defensive but is not, and it creates an implicit dependency on epoch bookkeeping that obfuscates intent.

A reader must mentally simulate three acquire/release cycles just to understand what epoch numbers to expect in the actual assertions. This is cognitive noise.

#### Name Good?
**NO.** `atomic_behavior` implies concurrent atomicity — two goroutines/threads racing for the lock. The test is entirely single-threaded and sequential. It should be called something like `acquire_block_release_reacquire_increments_epoch` if it were worth keeping.

---

## Recommendations

### Immediate

1. **REMOVE** `try_acquire_lock_atomic_behavior_held_returns_false_released_returns_true`.
   - No unique coverage. Superior tests exist in `sync_leader_lifecycle.rs` with proper traceability (`T-8.1: RTM FR-30.3`, `FR-30.6`, `FR-30.7`), richer DB-state assertions, and accurate names.
   - File a note confirming `fr30_3_ac6_release_succeeds_for_current_holder` covers the release-then-reacquire path before deleting.

2. **REWRITE** `sweep_abandoned_thinking_sessions_mtime_guard_protects_recently_edited_sessions`:
   - Move it into `session_cleanup_sweep.rs` (or create `session_cleanup_mtime.rs`)
   - Use the `session/session_cleanup_support.rs` harness (`isolated_env`, `read_db_status`, etc.)
   - Split into two tests:
     - `mtime_guard_shields_recently_edited_active_session`
     - `stale_mtime_and_stale_indexed_session_is_abandoned`
   - Add `read_db_status` assertion to match the `session_cleanup_ttl.rs` pattern
   - Fix traceability: `T-0.1: RTM NFR-3` → `T-4.5b: RTM FR-4.2` (matches `session_cleanup_ttl.rs` and `session_cleanup_sweep.rs`)
   - Replace `Duration::hours(2)` magic numbers with named constants

3. **Delete `sync_perf_coordination.rs`** once rewrites land. With both tests migrated/removed, the file (and its inclusion of `sync_perf_support.rs`) contributes no unique value.

### Structural / Long-term

4. The `// T-0.1: RTM NFR-3` annotation pattern is a red flag wherever it appears. NFR-3 ("no regressions") is a meta-requirement that applies to *all* tests — tagging a test with it specifically means the author couldn't identify the functional requirement being tested. Audit other files in the `sync_perf_*` cluster for the same pattern.

5. `sync_perf_support.rs` duplicates infrastructure from three specialized support files (`session/session_cleanup_support.rs`, `support/sync_leader.rs`, various concept-graph helpers). Consider whether the `sync_perf_*` cluster should be migrated to their respective purpose-built suites wholesale, leaving `sync_perf_support.rs` to only serve its rightful clients (`sync_perf_baseline.rs`, `sync_perf_graph.rs`, `sync_perf_indexing.rs`).

---

## Load-Bearing Score: 1/2

One of the two tests (`sweep_abandoned_thinking_sessions_mtime_guard_protects_recently_edited_sessions`) provides coverage that is genuinely absent from the dedicated session cleanup suites and cannot be safely deleted without creating a coverage gap in the mtime guard logic. It earns its existence but needs a rewrite. The other test is redundant dead weight.

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- sync_perf_coordination`: 2 passed, 0 failed
- Verdict: PASS
