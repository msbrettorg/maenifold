# Test Evaluation: sync_leader_gating

**File:** `crates/maenifold-core/tests/sync_leader_gating.rs`
**Test count:** 6
**Evaluated:** 2026-07-06
**Traceability:** T-12.7 (NFR-1 parity split), T-4.5.1 (FR-30.1, FR-30.2, FR-30.8)
**Companion evaluations:** `sync_leader_lifecycle.md`, `leader_election_boundary.md`

---

## Summary

This file has a structural identity problem: it was split from `sync_leader_lifecycle.rs` to hold "gating and status coverage" (per the T-12.7 comment), but four of its six tests do not test gating behavior at all. They test the `is_leader()` primitive — and two of those four are byte-for-byte identical to the other two.

The duplicate pairs are confirmed by direct body extraction. `fr30_1_ac1` and `fr30_2_ac1` share identical bodies down to whitespace. `fr30_1_ac2` and `fr30_2_ac2` are the same. The only distinction between each pair is the function name and the FR tag in the comment. Since the test bodies call `is_leader()` and assert a boolean result, they verify the same code path under the same conditions. Running both adds zero coverage, zero fault-isolation value, and two extra test executions that serialize behind `TEST_MUTEX`.

Beyond the duplication, the names of all four tests claim application-layer behavior that the assertions cannot verify. A test named `fr30_1_ac1_non_leader_skips_community_recomputation` asserts `!election.is_leader("holder-b")`. It would pass even if the community runner completely ignored the leader gate and always ran on every process. The test is measuring a necessary condition (non-holder gets `false` from `is_leader`) but claiming to measure a sufficient condition (community recomputation is actually skipped). This is test theater: the names signal coverage of FR-30.1 and FR-30.2's behavioral guarantee, but the assertions only cover the `is_leader()` API contract, which is already the primary subject of `sync_leader_lifecycle.rs`.

The two remaining tests — `fr30_3_is_leader_returns_false_when_no_lease_exists` and `fr30_3_is_leader_returns_false_for_expired_lease` — are genuinely load-bearing and correctly named. Test 5 exercises the cold-start path (no row in the `sync_leader_lease` table), which is not covered in isolation anywhere in the lifecycle file. Test 6 exercises the time-based expiry check in `is_leader()`'s SQL; the lifecycle file covers this behavior only as a side assertion inside `fr30_3_try_renew_fails_for_expired_lease`, not as a focused test. Both survive scrutiny.

The institutional context matters here: `leader_election_boundary.md` previously recommended that `is_leader_returns_false_when_no_lease_exists` be added to the lifecycle file. It was added to this file instead — which is acceptable, but means this file now owns the authoritative `is_leader` status tests, while the lifecycle file owns the state-machine transition tests. That split is defensible if the four duplicates are removed and the two remaining are renamed consistently.

---

## Verdicts

| # | Test Function | Verdict | Load-Bearing? | Unique? |
|---|---|---|---|---|
| 1 | `fr30_1_ac1_non_leader_skips_community_recomputation` | **REMOVE** | No — body is identical to test 3 | No |
| 2 | `fr30_1_ac2_leader_runs_community_recomputation` | **REMOVE** | No — body is identical to test 4 | No |
| 3 | `fr30_2_ac1_non_leader_skips_maintenance_work` | **REWRITE** | Partially — the scenario is unique, the name is false | Partially |
| 4 | `fr30_2_ac2_leader_runs_maintenance_normally` | **REWRITE** | Partially — the scenario is unique, the name is false | Partially |
| 5 | `fr30_3_is_leader_returns_false_when_no_lease_exists` | **KEEP** | Yes | Yes |
| 6 | `fr30_3_is_leader_returns_false_for_expired_lease` | **KEEP** | Yes | Yes (focused; lifecycle covers only as side assertion) |

---

### Test 1: `fr30_1_ac1_non_leader_skips_community_recomputation`

**Verdict: REMOVE**

Body is byte-for-byte identical to test 3 (`fr30_2_ac1_non_leader_skips_maintenance_work`). Both acquire a lease for `"holder-a"` with a 10 ms duration and assert `!election.is_leader("holder-b")`. There is no setup difference, no assertion difference, no scenario difference. The only distinguishing text is in the function name and the `// T-4.5.1: RTM FR-30.1` comment.

The name claims to verify that a non-leader skips community recomputation. The assertion is `!election.is_leader("holder-b")`. These are not the same claim. The community runner is not invoked, its skip path is not exercised, and no observable side effect of the skip is checked. The test would pass if the community runner always ran on every process and ignored the lease entirely.

Removing this test loses zero scenario coverage. Test 3 covers the same code path.

**Correctness of assertions:** The `assert!(!...)` form is correct for the boolean result, but the `.expect("holder b not leader")` message is misleading — `.expect()` on a `Result<bool>` fires when the `Result` is an `Err`, not when the bool is unexpected. The message reads as if it describes the expected value, but it would only appear in output if `is_leader()` returned an `Err`. This is a minor confusion present in all six tests. It should be `.unwrap()` or the message should say `"is_leader returned Err"`.

---

### Test 2: `fr30_1_ac2_leader_runs_community_recomputation`

**Verdict: REMOVE**

Body is byte-for-byte identical to test 4 (`fr30_2_ac2_leader_runs_maintenance_normally`). Both acquire for `"holder-a"` and assert `election.is_leader("holder-a")`. No difference in setup, assertions, or scenario.

The name claims that the leader runs community recomputation. The assertion is `is_leader("holder-a") == true`. The community runner is not invoked. This test would pass even if the community runner was deleted from the codebase. It asserts a necessary condition for FR-30.1 but not the behavioral guarantee FR-30.1 describes.

Removing this test loses zero scenario coverage. Test 4 covers the same code path.

---

### Test 3: `fr30_2_ac1_non_leader_skips_maintenance_work`

**Verdict: REWRITE**

The scenario is real and worth keeping: after an active lease is acquired by `"holder-a"`, a different process querying `is_leader("holder-b")` must receive `false`. This is a load-bearing check on the `is_leader()` SQL query — it verifies that `holder_id != ?2` (mismatched holder) is correctly excluded even when the lease is active and unexpired. The lifecycle file does not have this exact isolated test; its `is_leader` assertions are always in the same-holder context.

The name is the problem. `non_leader_skips_maintenance_work` implies the maintenance code path was exercised. It was not. Rename to `is_leader_returns_false_for_non_holder_with_active_lease`. Update the FR tag: this is testing FR-30.8 (the `is_leader` read path) not FR-30.2 (the maintenance gate). The FR-30.2 behavioral guarantee requires an integration test that actually calls the maintenance runner and verifies the early return fires — this test does not provide that.

Also fix the `.expect()` message: `"holder b not leader"` describes the expected outcome, not the error case. Use `"is_leader returned Err for holder-b"` or `.unwrap()`.

---

### Test 4: `fr30_2_ac2_leader_runs_maintenance_normally`

**Verdict: REWRITE**

Same analysis as test 3, flipped: the scenario (`is_leader` returns `true` for the current lease holder) is real and worth keeping. It is not covered in isolation in the lifecycle file, which tests `is_leader` returning `true` only implicitly (e.g., inside the release test chain). A dedicated test asserting that the active leaseholder gets `true` from `is_leader` is useful.

Rename to `is_leader_returns_true_for_current_holder_with_active_lease`. Update the FR tag to FR-30.8. Apply the same `.expect()` message fix.

---

### Test 5: `fr30_3_is_leader_returns_false_when_no_lease_exists`

**Verdict: KEEP**

This tests the cold-start code path: `is_leader()` called when the `sync_leader_lease` table has no row for `"sync_leader"`. The `is_leader()` implementation uses `SELECT EXISTS(...)`, and this test verifies that an empty result set returns `false`, not an error or an unexpected default. The lifecycle file calls `try_acquire()` before every `is_leader()` assertion; this is the only test in the suite that exercises `is_leader()` against an empty table.

The name is accurate. The assertion is correct. This test was previously recommended by `leader_election_boundary.md` to be migrated to the lifecycle file. That migration did not happen — it was placed here instead. The current placement is acceptable, since this file is now the authoritative home for `is_leader` status tests.

The `lease_duration_ms = 10` used in other tests is not relevant here (no acquire call), which keeps this test maximally minimal. Good.

---

### Test 6: `fr30_3_is_leader_returns_false_for_expired_lease`

**Verdict: KEEP**

This tests the time-based branch of `is_leader()`: the SQL's `AND lease_expires_at_ms > ?3` predicate. After acquiring the lease with a 10 ms duration and then forcing expiry via `expire_lease()` (SQL UPDATE to `lease_expires_at_ms = 1`), the test asserts `!is_leader("holder-a")`. The implementation's `is_leader` path is a read-only `SELECT EXISTS` that does not touch the expiry field beyond comparing it to `now_ms` — if this comparison were ever inverted or removed, only this test would catch it directly.

There is partial overlap: `sync_leader_lifecycle.rs` → `fr30_3_try_renew_fails_for_expired_lease` also calls `assert!(!election.is_leader(...))` after `expire_lease()`. However, that assertion is a secondary claim within a test about `try_renew` behavior. If it were removed or weakened during a refactor of that test, this dedicated test would remain as the safety net. The focused naming and single-purpose body justify its independent existence.

The `lease_duration_ms = 10` in the initial acquire call is suspicious: a 10 ms lease may already be expired by the time `expire_lease()` runs, making the latter call redundant in some environments under load. This is not a correctness risk (the test would still pass — expired by clock or by SQL update), but the intent is for `expire_lease()` to be the deterministic mechanism. Use a large duration (e.g., `60_000`) to make the test's dependency on `expire_lease()` unambiguous.

---

## Recommendations

1. **Remove tests 1 and 2.** They are byte-for-byte duplicates of tests 3 and 4. No scenario coverage is lost. Remove both function bodies and their comments.

2. **Rename tests 3 and 4 to honest names.**
   - `fr30_2_ac1_non_leader_skips_maintenance_work` → `is_leader_returns_false_for_non_holder_with_active_lease`
   - `fr30_2_ac2_leader_runs_maintenance_normally` → `is_leader_returns_true_for_current_holder_with_active_lease`

3. **Correct the FR tags on tests 3 and 4.** These tests exercise the `is_leader()` API contract (FR-30.8 in the sprint 4 PRD). They do not exercise the maintenance gate (FR-30.2). Tagging them `RTM FR-30.2` is incorrect and will mislead RTM auditors into believing FR-30.2 has behavioral test coverage when it does not.

4. **Add real FR-30.1 / FR-30.2 integration tests elsewhere.** The behavioral guarantee — "non-leader skips community recomputation" — is NOT covered anywhere in the test suite. Testing it requires calling the actual community runner or maintenance scheduler with a non-leader context and verifying the early return fires (e.g., via a spy, a counter, or a side-effect observable). A test that only calls `is_leader()` does not cover this guarantee. This gap was flagged in `TSYNCLEADER001.7 RedTeam` and remains open.

5. **Fix the `.expect()` message pattern across all six tests.** `election.is_leader("holder-b").expect("holder b not leader")` is semantically wrong: `.expect()` on a `Result<bool>` fires only when the result is `Err`, not when the bool is `false`. The message "holder b not leader" describes the expected assertion outcome, not the error condition. Use either `.unwrap()` (when the error case needs no message) or `.expect("is_leader returned Err")` (when a message adds diagnostic value). As written, the messages could mislead a developer reading a panic output.

6. **Use a large `lease_duration_ms` in test 6.** Replace `try_acquire("holder-a", 10)` with `try_acquire("holder-a", 60_000)` to make clear that the expiry in this test is 100% controlled by `expire_lease()`, not by wall-clock race.

7. **Consider renaming this file to `sync_leader_is_leader.rs`.** The current name "gating" implies testing the application-level gate on community recomputation and maintenance work. After the recommended rewrites, the file's remaining tests are exclusively about the `is_leader()` API surface. A name that reflects the actual content is less misleading.

---

## Load-Bearing Score: 4/6

Tests 5 and 6 are unambiguously load-bearing: each catches a distinct regression class in `is_leader()` (empty-table path and expiry-predicate path respectively) that no other test in the suite covers in isolation.

Tests 3 and 4 are conditionally load-bearing: the scenarios they cover (non-holder exclusion and holder confirmation against an active lease) are not tested in isolation in the lifecycle file. If the lifecycle file were the only frame of reference, tests 3 and 4 would be KEEP after renaming. They are scored as load-bearing in this count.

Tests 1 and 2 are not load-bearing: removing them loses no coverage. Tests 3 and 4 cover the same code paths with identical assertions.

**4 load-bearing out of 6 tests in the file. 2 are pure duplicates that inflate the count without adding safety.**

---
## SWE Remediation
- REMOVE `fr30_1_ac1_non_leader_skips_community_recomputation`: confirmed byte-for-byte dupe of test 3
- REMOVE `fr30_1_ac2_leader_runs_community_recomputation`: confirmed byte-for-byte dupe of test 4
- REWRITE `fr30_2_ac1_non_leader_skips_maintenance_work` → `is_leader_returns_false_for_non_holder_with_active_lease`: honest name, FR tag corrected to FR-30.8, `.expect()` → `.unwrap()`
- REWRITE `fr30_2_ac2_leader_runs_maintenance_normally` → `is_leader_returns_true_for_current_holder_with_active_lease`: honest name, FR tag corrected to FR-30.8, `.expect()` → `.unwrap()`
- KEEP `fr30_3_is_leader_returns_false_when_no_lease_exists`: fixed `.expect()` → `.unwrap()` per rec 5
- KEEP `fr30_3_is_leader_returns_false_for_expired_lease`: fixed `.expect()` → `.unwrap()` per rec 5, `lease_duration_ms` 10 → 60_000 per rec 6
- Rec 4 (add real FR-30.1/FR-30.2 integration tests): out of scope — requires application-layer test harness
- Rec 7 (rename file): deferred — "Consider" priority, ripple effects on module declaration in main.rs
- Tests: 4 passed, 0 failed (down from 6 → 4 after removing duplicates)
- Commit: f14da4f
