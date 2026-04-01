---
id: T-80.88
title: "Test Evaluation: leader_election_boundary"
status: done
pipeline: tdd
---

# Test Evaluation: leader_election_boundary

**File:** `crates/maenifold-core/tests/leader_election_boundary.rs`
**Test count:** 7
**Evaluated:** 2026-03-14
**Traceability:** T-4.5a (FR-30.1 initial acquire, FR-30.2 same-holder renew, FR-30.3 contention, FR-30.4 expiry takeover, FR-30.5/FR-30.6/FR-30.8 is_leader states, FR-30.7 release)
**Companion file:** `crates/maenifold-core/tests/sync_leader_lifecycle.rs` (evaluated separately in `sync_leader_lifecycle.md`)

---

## Summary

This file is a **predecessor artifact**. It predates the shared support module at `tests/support/sync_leader.rs` and the purpose-built `sync_leader_lifecycle.rs`, and has not been kept in sync with either. Six of the seven tests are fully superseded by the lifecycle file with identical or stronger assertion coverage. The one test containing a partially unique assertion (`is_leader_returns_correct_status`) buries that assertion inside a six-state omnibus that also sleeps for 1 100 ms — making it the most expensive test in the suite for the least unique return.

Three structural problems compound the coverage overlap:

1. **Duplicate fixtures.** `ConfigOverrideReset`, `TEST_MUTEX`, `isolated_env()`, `leader_election()`, `open_db()`, and `current_lease()` are all copy-pasted from the shared support module rather than imported. Any change to the canonical implementations in `support/sync_leader.rs` must also be replicated here manually or the two will drift.

2. **Wall-clock sleeps.** Tests #4, #6, and #7 each call `sleep_for_expiry()` (1 100 ms). The lifecycle file eliminates every wall-clock sleep by calling `expire_lease()` — a direct SQL UPDATE. Since `TEST_MUTEX` serialises the entire suite, these three sleeps contribute a guaranteed minimum of 3.3 seconds of dead time per test run, with no compensating coverage that the lifecycle file does not already provide.

3. **Stale task reference.** Every test is tagged `// T-4.5a: RTM FR-30.x`. The implementation comments (`leader.rs`) use `T-8.1` and `T-8.5a` for the same requirements. The tag `T-4.5a` points to Sprint 4, but Sprints 4–8 are not in `CHANGELOG.md`. The reference is technically traceable to completed work, but the inconsistency with the implementation's own task tags creates audit friction. Additionally, `FR-30` in the **current** PRD/RTM refers to the ACP Response Capture Pipeline — a completely different feature — creating a terminology collision for anyone reading tests against the live RTM.

**Recommendation: retire this file.** The one genuinely unique behavioural claim it contains (`is_leader` returns `false` when the lease table has no row) can be extracted into a single three-line test added to the lifecycle file.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `first_acquire_succeeds_when_no_existing_leader` | **REMOVE** | Fully superseded by `fr30_3_ac1_try_acquire_succeeds_on_empty_table` in the lifecycle file. The lifecycle version uses the `LeaseRow` struct for named-field assertions (`holder_id`, `epoch`, `lease_expires_at_ms > 0`, `lease_expires_at_ms > updated_at_ms`) rather than a positional tuple, making it both easier to read and less fragile. Every assertion here is reproduced there. No unique behavioural coverage. |
| `same_holder_reacquire_renews_existing_lease` | **REMOVE** | Mixed-concern test that invokes both `try_acquire` (same-holder path) AND `try_renew` in one body with a 25 ms sleep in between. The `try_acquire` idempotency path is superseded by `fr30_3_try_acquire_idempotent_for_same_holder`; the `try_renew` path is superseded by `fr30_3_ac5_try_renew_succeeds_for_current_holder`. The lifecycle tests cover each in isolation with cleaner assertions and `short_sleep()` instead of a hardcoded 25 ms. The combined test provides no coverage that the two focused lifecycle tests do not already cover, and makes fault isolation impossible when either path regresses. |
| `different_holder_is_blocked_while_lease_is_active` | **REMOVE** | Superseded by `fr30_3_ac3_try_acquire_fails_when_lease_active`. Both verify the full return tuple `(false, "held_by_other", 1)`, the `is_leader` state for both holders, and the unchanged DB row. The lifecycle version also asserts `current_lease.holder_id` via a named field rather than a positional tuple, reducing assertion brittleness. No unique coverage. |
| `expired_lease_allows_new_holder_to_acquire` | **REMOVE** | Superseded by `fr30_3_ac4_try_acquire_succeeds_on_expired_lease`. The lifecycle version uses `expire_lease()` (direct SQL UPDATE to `lease_expires_at_ms = 1`) instead of `sleep_for_expiry()` (1 100 ms wall-clock wait), making the coverage deterministic and 100× faster. The lifecycle version asserts `after.holder_id`, `after.epoch == before.epoch + 1`, and the returned tuple — all claims that this test also makes. There is no assertion here that the lifecycle test does not already make in a more reliable form. |
| `release_clears_leadership` | **REMOVE** | Superseded by `fr30_3_ac6_release_succeeds_for_current_holder`, which is strictly more thorough: it verifies the same zeroed-field state (empty `holder_id`, `epoch` preserved, `lease_expires_at_ms == 0`, `is_leader` returns false), and additionally verifies that a subsequent `try_acquire` by a new holder succeeds and correctly increments the epoch — a critical reuse-after-release invariant that this test omits entirely. Furthermore, the assertion here uses a complex `matches!` pattern with positional destructuring (`ref holder_id, released_epoch, 0, _`) that is significantly harder to read than the lifecycle's direct struct-field checks. Nothing unique remains once ac6 is present. |
| `is_leader_returns_correct_status` | **REWRITE** | Contains **one genuinely unique assertion**: `!election.is_leader("holder-a")` is called before any `try_acquire`, verifying that `is_leader` correctly returns `false` when the `sync_leader_lease` table has no row for the lock name. None of the lifecycle tests call `is_leader` in the empty-table state; they all acquire first. This is a real, if narrow, edge case — a future refactor that changes `is_leader` to return an error or `true` on missing rows would not be caught by the lifecycle file alone. The remaining five state transitions in this test (post-acquire is_leader, non-holder is_leader, try_renew success, is_leader after expiry, try_renew on expired) are all covered by the lifecycle file. The entire test sleeps for 1 100 ms. **Recommended replacement**: a single four-assertion focused test — `is_leader_returns_false_when_no_lease_exists` — added to the lifecycle file, containing only the `isolated_env()` setup, the `is_leader` call, and the `!result` assertion. |
| `epoch_increments_when_new_holder_acquires_after_expiry` | **REMOVE** | Fully redundant on two levels. First, within this same file, `expired_lease_allows_new_holder_to_acquire` already asserts `assert_eq!(outcome, (true, "acquired", 2))`, which is mathematically identical to `next_epoch == initial_epoch + 1` when starting from epoch 1. Second, the lifecycle's `fr30_3_ac4_try_acquire_succeeds_on_expired_lease` makes the same assertion in a deterministic form: `assert_eq!(after.epoch, before.epoch + 1)`. This test duplicates existing coverage, adds nothing new, costs another 1 100 ms sleep, and is the weakest test in the file by every criterion. |

---

## Recommendations

1. **Retire the file entirely.** All seven tests are either fully superseded or have their unique assertions extractable into a single focused test. Remove `mod leader_election_boundary;` from `tests/main.rs` (line 76) as part of the same commit.

2. **Extract the one unique assertion.** Add `is_leader_returns_false_when_no_lease_exists` to `sync_leader_lifecycle.rs`. This is a three-line test — setup, call, assert — and closes the only gap this file currently fills:
   ```rust
   // T-4.5a: RTM FR-30.8
   #[test]
   fn is_leader_returns_false_when_no_lease_exists() {
       let _guard = TEST_MUTEX.lock();
       let _reset = ConfigOverrideReset;
       let (_tmp, cfg) = isolated_env();
       let election = leader_election(&cfg);
       assert!(!election.is_leader("holder-a").expect("no lease row"));
   }
   ```

3. **Correct the stale task tag.** All tests here use `// T-4.5a`. The implementation uses `T-8.1` and `T-8.5a` for the same requirements. When the unique assertion above is ported to the lifecycle file, it should carry `// T-8.1: RTM FR-30.8` for consistency with the rest of that file.

4. **Note the FR-30 terminology collision.** In the current PRD/RTM, `FR-30` refers to the ACP Response Capture Pipeline. The sync leader lease protocol was numbered `FR-30.x` in an older PRD version (Sprints 4–8). This collision will cause confusion for anyone cross-referencing test tags against the live RTM. The implementation itself uses the same stale references. A follow-up task should either: (a) annotate the `leader.rs` implementation with a note that `FR-30.x` here refers to the completed sync leader feature from Sprint 4/8, not the current ACP FR-30; or (b) update the comments to reference the correct completed RTM numbers from `CHANGELOG.md` once those entries are added for Sprints 4–8.

5. **Remove wall-clock sleeps from the test corpus.** The `sleep_for_expiry()` pattern should not be introduced in any new tests. `expire_lease()` from the shared support module is the canonical approach. If any future tests are written against `LeaderElection`, they must use `expire_lease()`.

---

## Load-Bearing Score: 1/7

**When evaluated in isolation (no lifecycle file):** 5/7. Tests #4 and #7 are redundant with each other even without the lifecycle file. Tests #1, #2, #3, #5, #6 each cover distinct scenarios.

**When evaluated alongside `sync_leader_lifecycle.rs` (correct framing):** 1/7. Only `is_leader_returns_correct_status` contains a single unique behavioural claim (is_leader on empty table), and that claim is buried inside an omnibus test alongside five superseded state transitions and a 1 100 ms sleep. No other test in this file adds coverage not already present in the lifecycle file.

---
## SWE Remediation
- REMOVE first_acquire_succeeds_when_no_existing_leader: superseded by fr30_3_ac1_try_acquire_succeeds_on_empty_table
- REMOVE same_holder_reacquire_renews_existing_lease: superseded by fr30_3_try_acquire_idempotent_for_same_holder + fr30_3_ac5_try_renew_succeeds_for_current_holder
- REMOVE different_holder_is_blocked_while_lease_is_active: superseded by fr30_3_ac3_try_acquire_fails_when_lease_active
- REMOVE expired_lease_allows_new_holder_to_acquire: superseded by fr30_3_ac4_try_acquire_succeeds_on_expired_lease
- REMOVE release_clears_leadership: superseded by fr30_3_ac6_release_succeeds_for_current_holder
- REWRITE is_leader_returns_correct_status → extracted unique empty-table assertion as `is_leader_returns_false_when_no_lease_exists` into sync_leader_lifecycle.rs with tag T-8.1: RTM FR-30.8
- REMOVE epoch_increments_when_new_holder_acquires_after_expiry: redundant with fr30_3_ac4_try_acquire_succeeds_on_expired_lease
- Retired leader_election_boundary.rs entirely; removed mod declaration from main.rs (line 74)
- All 12 sync_leader_lifecycle tests pass (11 existing + 1 new); 0 failed

---
## Red-team Audit

**Traceability:** T-4.5a (FR-30.1–FR-30.8)

### REMOVE verdicts (6/6 confirmed)
- ✅ REMOVE `first_acquire_succeeds_when_no_existing_leader`: file deleted, superseding test `fr30_3_ac1_try_acquire_succeeds_on_empty_table` confirmed present (lifecycle line 12)
- ✅ REMOVE `same_holder_reacquire_renews_existing_lease`: file deleted, superseding tests `fr30_3_try_acquire_idempotent_for_same_holder` + `fr30_3_ac5_try_renew_succeeds_for_current_holder` confirmed present
- ✅ REMOVE `different_holder_is_blocked_while_lease_is_active`: file deleted, superseding test `fr30_3_ac3_try_acquire_fails_when_lease_active` confirmed present (lifecycle line 50)
- ✅ REMOVE `expired_lease_allows_new_holder_to_acquire`: file deleted, superseding test `fr30_3_ac4_try_acquire_succeeds_on_expired_lease` confirmed present (lifecycle line 74)
- ✅ REMOVE `release_clears_leadership`: file deleted, superseding test `fr30_3_ac6_release_succeeds_for_current_holder` confirmed present (lifecycle line 124)
- ✅ REMOVE `epoch_increments_when_new_holder_acquires_after_expiry`: file deleted, redundancy with `fr30_3_ac4_try_acquire_succeeds_on_expired_lease` confirmed

### REWRITE verdict (1/1 confirmed)
- ✅ REWRITE `is_leader_returns_correct_status` → `is_leader_returns_false_when_no_lease_exists`: added to `sync_leader_lifecycle.rs` at line 244. Test body matches eval-recommended code exactly (setup → call → assert, 3-line focused test). Task tag `T-8.1: RTM FR-30.8` matches eval recommendation #3.

### File retirement
- ✅ `leader_election_boundary.rs` deleted (verified: file does not exist on disk)
- ✅ `mod leader_election_boundary;` removed from `main.rs` (verified: git diff shows removal at former line 74)

### Test execution
- ✅ 12/12 `sync_leader_lifecycle` tests pass (`cargo test -p maenifold-core sync_leader_lifecycle`): 11 existing + 1 new, 0 failures

### No unrelated changes
- ✅ Commit 549f292 bundles this remediation with `markdown_adversarial_structure` (separate eval), but the leader_election_boundary changes are cleanly isolated: 1 file deleted, 1 mod line removed from main.rs, 10 lines added to lifecycle file. No other leader-election-related files modified.

### Observation (non-blocking)
- `sync_leader_gating.rs` (added in Sprint 15, commit 558d5ff) already contains `fr30_3_is_leader_returns_false_when_no_lease_exists` — the identical empty-table assertion. This predates the eval remediation. The lifecycle file now has its own copy as the eval specified. Coverage overlap exists across two files; the gating file's evaluation is tracked separately per the companion-file note in this eval.

### Verdict: PASS

---
## Blue-team Verification
- `leader_election_boundary.rs`: confirmed deleted (file does not exist on disk)
- `mod leader_election_boundary;`: confirmed absent from `main.rs` (no grep match)
- `is_leader_returns_false_when_no_lease_exists`: confirmed present in `sync_leader_lifecycle.rs` at line 244
- `cargo test --test integration_tests -p maenifold-core -- sync_leader_lifecycle`: 12 passed, 0 failed
- Superseding tests confirmed: all 11 original + 1 new extracted assertion pass
- Verdict: PASS
