# Test Evaluation: sync_leader_lifecycle

**File:** `crates/maenifold-core/tests/sync_leader_lifecycle.rs`
**Test count:** 11
**Evaluated:** 2026-03-14
**Traceability:** T-8.1 (FR-30.3 try_acquire, FR-30.4 expired takeover, FR-30.6 renew, FR-30.7 release, FR-30.8 epoch guard), T-12.7 (NFR-1 parity lifecycle split)

## Summary

This is the strongest of the three files under review. Ten of eleven tests are genuinely load-bearing and test distinct, non-overlapping state machine transitions with correct behavioural assertions. The one exception is `fr30_3_ac8_try_acquire_returns_reason_codes`, which is an omnibus test that bundles four independent scenarios into a single body — worth splitting for isolation, but the assertions themselves are correct and parity-justified (the reason strings are part of the C# oracle protocol). The naming convention is inconsistent across the file: tests 1–8 use `ac{N}` acceptance-criterion numbering while the last three abandon that scheme entirely.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `fr30_3_ac1_try_acquire_succeeds_on_empty_table` | **KEEP** | Entry point of the lease lifecycle. Verifies `acquired=true`, reason `"acquired"`, epoch=1, and all four database fields (`holder_id`, `epoch`, `lease_expires_at_ms > 0`, `lease_expires_at_ms > updated_at_ms`). Every assertion is behavioural, not cosmetic. Load-bearing. |
| `fr30_3_ac2_try_renew_fails_for_non_holder` | **KEEP** | Directly tests the authorization boundary: a non-holder MUST NOT be able to extend a lease. Correctly asserts both the return value (`!renewed`) and the side-effect absence (lease row unchanged via `assert_eq!(current_lease, Some(before))`). The "lease is unchanged" assertion is the critical load-bearing claim. |
| `fr30_3_ac3_try_acquire_fails_when_lease_active` | **KEEP** | Tests the contention path. Correctly asserts the full tuple `(false, "held_by_other", 1)` and verifies the original holder remains in the DB. The reason code `"held_by_other"` is protocol-level (oracle parity), so asserting the string is correct. Load-bearing. |
| `fr30_3_ac4_try_acquire_succeeds_on_expired_lease` | **KEEP** | Tests lease takeover after expiry — the most complex valid transition. Asserts epoch increments (`epoch = before.epoch + 1`), new holder persisted, and reason `"acquired"` not `"renewed"`. If this test were removed, a bug that fails to increment the epoch on takeover would ship undetected. Load-bearing. |
| `fr30_3_ac5_try_renew_succeeds_for_current_holder` | **KEEP** | Tests that the holder can extend its own lease without incrementing the epoch. Asserts `after.epoch == before.epoch` (epoch stability) and `after.lease_expires_at_ms >= before.lease_expires_at_ms` (expiry pushes forward). The `short_sleep()` before renewal makes the timestamp comparison meaningful — without it, a no-op renewal would still pass. Load-bearing. |
| `fr30_3_ac6_release_succeeds_for_current_holder` | **KEEP** | The most comprehensive test in the file: release → verify zeroed fields → verify `is_leader` returns false → reacquire → verify epoch increments. It is doing the work of potentially three smaller tests, but the multi-step chain is testing a critical real-world sequence (release-then-reacquire). The epoch increment on re-acquire after release is a distinct and important invariant. Load-bearing. Consider splitting into `release_zeroes_lease_row` and `reacquire_after_release_increments_epoch` for faster fault isolation. |
| `fr30_3_ac7_release_noop_for_wrong_holder` | **KEEP** | Tests the authorization boundary for release, symmetric to ac2. Asserts `!released` and `current_lease == Some(before)`. The "lease unchanged" assertion is the load-bearing claim — without it a buggy implementation that returns false but still clears the lease would pass. Load-bearing. |
| `fr30_3_ac8_try_acquire_returns_reason_codes` | **REWRITE** | This test exercises four sequential state transitions in one body: fresh acquire, same-holder re-acquire (renewal), contended acquire, and force acquire. When this test fails, there is no signal which of the four scenarios regressed. Each scenario is a distinct, named acceptance criterion. The `force_acquire` scenario is especially important — it is not tested in isolation anywhere else in this file. **Recommended split:** `try_acquire_same_holder_returns_renewed`, `try_acquire_contended_returns_held_by_other`, and `force_acquire_preempts_active_lease` as three focused tests. Keep the existing reason-code string assertions in each (they are correct for NFR-1 parity). |
| `fr30_3_try_acquire_idempotent_for_same_holder` | **KEEP** | Tests the idempotency invariant: same holder calling `try_acquire` again gets `"renewed"` and the epoch does NOT increment. This is distinct from ac5 (which tests `try_renew`, not `try_acquire`). Correctly uses `short_sleep()` to make timestamp assertions meaningful. The dual assertion (`first = (true, "acquired", 1)` and `second = (true, "renewed", 1)`) pins both the initial state and the idempotent result. Load-bearing. |
| `fr30_3_try_renew_fails_for_expired_lease` | **KEEP** | Tests that an expired lease cannot be renewed — the clock-based expiry boundary. Uses `expire_lease()` to deterministically set the lease into the past. Asserts `!renewed`, `current_lease` unchanged, and `is_leader` returns false. All three assertions together are necessary: return value alone wouldn't catch a bug that returns false but still updates the lease row. Load-bearing. |
| `fr30_3_try_renew_fails_for_stale_epoch` | **KEEP** | Tests that a stale epoch (epoch + 1 against current epoch) prevents renewal. This is the compare-and-swap guard that prevents split-brain. Without this test, a bug that ignores the epoch in the UPDATE WHERE clause would ship undetected. Correctly asserts `!renewed` and `current_lease == Some(before)`. Load-bearing. |

## Recommendations

1. **Split `fr30_3_ac8_try_acquire_returns_reason_codes` into three tests.** An omnibus that chains four state transitions provides no fault isolation. When it fails, you cannot tell which of the four scenarios regressed without reading the assertion output carefully. The `force_acquire` path in particular deserves its own test because it is not exercised anywhere else in this file.

2. **Adopt a consistent naming convention.** Tests 1–8 use the `ac{N}` scheme derived from acceptance criteria numbers; tests 9–11 drop it. Either apply the scheme uniformly or switch entirely to descriptive names. The current hybrid is confusing: `fr30_3_ac8_try_acquire_returns_reason_codes` and `fr30_3_try_acquire_idempotent_for_same_holder` look like they belong to different files.

3. **Consider splitting `fr30_3_ac6_release_succeeds_for_current_holder`.** It tests release, `is_leader` after release, re-acquire, and epoch increment on re-acquire. These are four distinct assertions about two separate operations. Splitting into `release_zeroes_lease_fields` and `reacquire_after_release_increments_epoch` would give cleaner fault signals.

4. **Add a test for `force_acquire` on an active (non-expired) lease in isolation.** Currently `force_acquire` is only covered inside the omnibus `ac8`. An isolated `force_acquire_succeeds_on_active_lease` test with assertions on `holder_id`, epoch increment, and reason code `"force_acquired"` would close this gap.

5. **The `short_sleep()` calls in ac5 and `idempotent` are load-bearing setup — document why.** Without the sleep, `updated_at_ms` timestamps may be equal before and after renewal (same millisecond), making `>=` comparisons trivially true. A comment explaining this would prevent a future refactor from removing the sleep and silently weakening the assertions.

## Load-Bearing Score: 10/11

Ten tests would catch real bugs if removed. `fr30_3_ac8_try_acquire_returns_reason_codes` is load-bearing in content but architecturally unsound — it would be more accurately counted as four tests worth of coverage crammed into one, making fault isolation impossible on failure.
