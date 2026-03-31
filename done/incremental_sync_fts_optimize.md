---
id: incremental_sync_fts_optimize
status: done
---

# Test Evaluation: incremental_sync_fts_optimize

**Source file**: `crates/maenifold-core/tests/incremental_sync_fts_optimize.rs`
**Traceability**: T-15.21 | FR-15 | NFR-2
**Evaluator**: Red-team / Gray (Skeptical Inquiry)
**Date**: 2025-07-25

---

## Summary

This file tests the FTS optimize scheduling logic in `process_incremental_batch` — specifically
the `schedule_maintenance_if_needed` function that mirrors C#'s
`IncrementalSyncTools.ScheduleMaintenanceIfNeeded()`. The mechanism maintains a global
`AtomicU32` (`CHANGES_SINCE_OPTIMIZE`) across calls and fires `run_light_maintenance` (FTS
`INSERT INTO file_search VALUES('optimize')` + `PRAGMA wal_checkpoint(PASSIVE)`) when the
accumulated counter reaches `MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY`.

The suite has **4 tests** covering: at-threshold single batch, below-threshold single file,
cross-batch accumulation, and empty batch. Three are genuinely load-bearing. One
(`empty_batch_does_not_increment_optimize_counter`) contains a **tautological assertion** that
cannot catch any real bug in the guard it purports to test.

**Critical finding**: No test verifies that `run_light_maintenance` actually *executed* — only
that the counter *reset*. Because the production code swallows maintenance errors silently
(`if let Err(e) = ... { eprintln!(...) }`), a maintenance failure leaves the counter at 0
(triggering condition) and the FTS index unmaintained, while every test here still passes.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `batch_sync_triggers_fts_optimize_after_reaching_threshold` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes | **KEEP** |
| 2 | `single_file_below_threshold_does_not_trigger_optimize` | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes | ✅ Yes | **KEEP** |
| 3 | `accumulated_small_batches_trigger_optimize_at_threshold` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | **KEEP** |
| 4 | `empty_batch_does_not_increment_optimize_counter` | ❌ No | ❌ No | ❌ Tautology | ✅ Yes | N/A | **REWRITE** |

---

## Per-Test Analysis

### Test 1 — `batch_sync_triggers_fts_optimize_after_reaching_threshold`

**What it does**: Sets threshold=3, creates exactly 3 files, processes as a single batch, asserts
`changes_since_last_optimize() == 0`.

**Load-bearing**: Yes. This is the core happy path: counter resets after a single batch that
reaches the threshold. Without this test, the primary scheduler contract is unverified.

**Unique**: Yes. Tests the at-threshold, single-batch case. Test 3 covers multi-batch; test 2
covers below-threshold. No overlap.

**Assertion correctness**: Partial. The assertion `counter == 0` proves the counter *reset* but
does not prove `run_light_maintenance` ran successfully. The production code at line 334 swallows
maintenance errors:

```rust
if let Err(error) = db::run_light_maintenance(&conn) {
    eprintln!("[maenifold] incremental maintenance failed: {error}");
}
```

A broken `run_light_maintenance` (e.g., `file_search` virtual table missing) would still reset
the counter. The test passes; FTS is never optimized. This is a coverage gap, not a test defect
per se, but the test's confidence claim exceeds what its assertions can support.

**Secondary gap**: The test only validates at-threshold (N==threshold). It does not validate that
threshold-1 files leave the counter at threshold-1. Test 3's intermediate assertion partially
covers this, but test 1 in isolation doesn't protect against an off-by-one where `>= threshold`
was changed to `> threshold`.

**Name**: Good. Accurately describes behavior.

**Parameterizable**: Yes. This test and test 2 could be parameterized as
`(threshold, batch_size, expected_counter)` pairs, capturing at-threshold, below-threshold, and
above-threshold in one table-driven structure.

---

### Test 2 — `single_file_below_threshold_does_not_trigger_optimize`

**What it does**: Sets threshold=10, processes 1 file, asserts counter == 1.

**Load-bearing**: Yes. This is the critical anti-false-positive test. It verifies two things:
(a) the counter increments when below threshold, (b) optimize does NOT fire prematurely. Both
are necessary for correct batching behavior.

**Unique**: Partially. Test 3's first-batch assertion (`counter == 2` with threshold=4) also
validates "below threshold → counter increments", but with different values. Test 2's single-file
isolation makes the intent clearer and catches bugs that appear only with batch_size=1.

**Assertion correctness**: Yes. `counter == 1` after processing 1 file with threshold=10 is
both correct and non-trivial.

**Threshold choice skepticism**: Threshold=10 is needlessly large. It makes the "below threshold"
condition clear but contributes nothing beyond threshold=2 would. The specific value has no
additional test-discriminating power. Not a defect, but threshold=2 would be more minimal.

**Name**: Good.

**Parameterizable**: Yes — merge with test 1 into a parameterized suite.

---

### Test 3 — `accumulated_small_batches_trigger_optimize_at_threshold`

**What it does**: Sets threshold=4, processes two separate batches of 2 files each. After batch 1:
asserts `counter == 2`. After batch 2: asserts `counter == 0`.

**Load-bearing**: Yes — and this is the **most important test in the file**. It validates the
cross-batch accumulation semantics of `CHANGES_SINCE_OPTIMIZE`: that the global atomic persists
across multiple calls to `process_incremental_batch`. If the counter were local or reset after
every batch, this test would catch it.

**Unique**: Yes. Neither test 1 nor test 2 exercise cross-call accumulation. This is a qualitatively
different behavioral contract.

**Assertion correctness**: Yes. Two sequential assertions (counter=2, then counter=0) correctly
trace the state machine.

**Name**: Good.

**Parameterizable**: Yes, but the two-phase structure (assert mid-way, then assert after) makes
it less trivially parameterizable than tests 1–2. Keep as a standalone scenario.

**Gap**: No assertion that a *third* batch starts accumulating fresh from 0 (i.e., counter was
truly reset, not just coincidentally 0). With threshold=4 and batch=2, a third batch of 2 should
bring counter back to 2. This would conclusively prove reset-then-accumulate, not just reset.

---

### Test 4 — `empty_batch_does_not_increment_optimize_counter`

**What it does**: Sets threshold=5, resets counter to 0, processes empty slice `&[]`, asserts
`counter == 0`.

**Load-bearing**: No.

**Tautology proof**: Trace the execution:

1. `reset_optimize_counter()` → `CHANGES_SINCE_OPTIMIZE = 0`
2. `process_pending_paths_for_test(&ctx, &[])` → no valid paths → `unique_paths` empty →
   `process_incremental_batch(config, &[])` called with empty vec
3. `sorted_paths.len() as u32 == 0` → `schedule_maintenance_if_needed(config, 0)`
4. **Path A** (guard present): `if batch_size == 0 { return; }` → counter stays 0 ✓
5. **Path B** (guard removed): `fetch_add(0, Relaxed)` → `accumulated = 0 + 0 = 0` →
   `0 >= 5` is false → no reset → counter stays 0 ✓

**Both paths yield `counter == 0`.** The assertion `assert_eq!(counter, 0)` passes regardless
of whether the `if batch_size == 0 { return; }` guard exists. This test **cannot detect the
removal of the guard it purports to test**.

The only scenario this test would catch is one where the empty-batch call somehow increments
the counter to a non-zero value — but `fetch_add(0, ...)` on a u32 atomic cannot do that.
The test provides zero mutation-killing power against the guard condition.

**Name**: Good — but the test does not deliver what the name promises.

**Correct rewrite**: Start with a non-zero counter to make the "no increment" assertion non-trivial:

```rust
// Accumulate 2 changes below threshold (threshold=5)
let (path1, _) = create_test_file(&cfg, "tests/fts-empty", "# A\n[[a]]\n");
let (path2, _) = create_test_file(&cfg, "tests/fts-empty", "# B\n[[b]]\n");
process_pending_paths_for_test(&ctx, &[path1, path2]).expect("prime");
assert_eq!(changes_since_last_optimize(), 2, "baseline primed");

// Now process an empty batch — counter must NOT change
process_pending_paths_for_test(&ctx, &[]).expect("empty batch");
assert_eq!(
    changes_since_last_optimize(),
    2,  // unchanged, not 3 or 0
    "empty batch must not disturb accumulated counter"
);
```

This rewrite turns the tautology into a genuine regression guard: it would fail if `batch_size=0`
was mistakenly treated as `batch_size=1`, or if the guard erroneously reset the counter.

---

## Cross-Cutting Concerns

### 1. Maintenance execution not verified (affects tests 1, 3)

The tests verify the *counter* resets, which is a proxy for maintenance being scheduled. They do
not verify that `run_light_maintenance` actually succeeded. Consider adding:

```rust
// After optimize fires, verify FTS integrity is maintained
let conn = db_connection(&cfg);
let result = conn.query_row(
    "SELECT COUNT(*) FROM file_search WHERE file_search MATCH 'fts-optimize-concept-0'",
    [],
    |row| row.get::<_, i64>(0),
);
assert!(result.is_ok(), "FTS should be queryable after optimization");
```

Or at minimum, instrument `run_light_maintenance` to return a status that the test helper can
surface.

### 2. Env var not panic-safe (affects all 4 tests)

`std::env::remove_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY")` is called at the end of each test
body — but NOT in a panic-safe Drop guard. If an assertion fails (panic), the env var leaks to
subsequent tests. Since tests 2–4 call `set_var` at the start, they will override the leaked
value correctly, making this a low-severity risk in practice. However, the hygiene is
inconsistent: `ConfigOverrideReset` is Drop-guarded but env var teardown is not. The fix is to
include env var removal in `ConfigOverrideReset::drop()` or use a scoped env var helper.

### 3. Counter global state not reset by `ConfigOverrideReset`

`ConfigOverrideReset::drop()` calls `Config::reset_overrides()` but does NOT call
`reset_optimize_counter()`. Tests call `reset_optimize_counter()` explicitly in their bodies. If
a test panics before reaching `reset_optimize_counter()` (e.g., an earlier assertion fails), the
counter leaks its value to the next test. The mutex serializes tests, so this is a sequential
risk. Consider adding `reset_optimize_counter()` to `ConfigOverrideReset::drop()`.

### 4. Missing coverage scenarios

| Missing Scenario | Risk | Recommended New Test |
|---|---|---|
| `threshold == 1` (every sync triggers) | Off-by-one in `>= threshold` | `threshold=1, file=1 → counter=0` |
| `batch_size > threshold` (single overshoot) | Possible counter underflow or missed trigger | `threshold=3, batch=5 → counter=0` |
| Post-trigger accumulation restart | Second crossing never tested explicitly | Extend test 3 with a third batch |
| `run_light_maintenance` returns error | Maintenance silent failure | Mock or corrupt FTS table, verify error logged |

---

## Recommendations

1. **REWRITE test 4** (`empty_batch_does_not_increment_optimize_counter`) with a non-zero
   baseline counter so the assertion is non-tautological. See corrected implementation above.

2. **Add `reset_optimize_counter()` to `ConfigOverrideReset::drop()`** to make counter cleanup
   panic-safe and reduce per-test boilerplate.

3. **Consider a Drop-guarded env var helper** (or extend `ConfigOverrideReset`) to ensure
   `MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY` is always removed even on test panic.

4. **Add a `threshold == 1` boundary test** to protect against off-by-one regressions in
   the `>= threshold` comparison.

5. **Add a `batch_size > threshold` test** to verify the scheduler handles single-shot
   overshoots (e.g., a large watcher flush) without counter arithmetic errors.

6. **Consider parameterizing tests 1 and 2** into a single table-driven test covering at-threshold,
   below-threshold, and above-threshold in one structure, reducing duplication.

7. **Consider adding a `run_light_maintenance` execution verification** — either by querying FTS
   after optimize fires, or by making the production error path surface to the caller rather than
   silently `eprintln!`-ing.

---

## Load-Bearing Score: 3/4

| Test | Verdict | Load-Bearing? | Justification |
|------|---------|:---:|---|
| `batch_sync_triggers_fts_optimize_after_reaching_threshold` | KEEP | ✅ | Core at-threshold, single-batch contract |
| `single_file_below_threshold_does_not_trigger_optimize` | KEEP | ✅ | Anti-false-positive; counter increments below threshold |
| `accumulated_small_batches_trigger_optimize_at_threshold` | KEEP | ✅ | Cross-batch accumulation — the most distinctive scenario |
| `empty_batch_does_not_increment_optimize_counter` | REWRITE | ❌ | Tautological: `counter == 0` is true regardless of guard presence |

**3 out of 4 tests earn their place.** The suite provides solid coverage of the core scheduling
contract (FR-15 / T-15.21) but has a dead-weight test that wastes the reader's trust budget,
silently accepts maintenance failures, and carries a latent env-var leak on panic.

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- incremental_sync_fts_optimize`: 6 passed, 0 failed
- SWE remediation confirmed:
  - Test 4 (`empty_batch_does_not_increment_optimize_counter`) **rewritten**: now primes counter to 2 with real files before processing empty batch, asserts counter remains 2 — eliminates the tautology (red-team §Test 4)
  - New test `threshold_one_triggers_optimize_on_every_sync`: boundary test for threshold=1, guards against off-by-one in `>= threshold` (red-team §Missing Coverage, row 1)
  - New test `batch_exceeding_threshold_triggers_optimize`: batch_size=5 with threshold=3, guards against counter-arithmetic bugs on overshoot (red-team §Missing Coverage, row 2)
  - Env var cleanup in rewritten/new tests relies on `ConfigOverrideReset` Drop guard rather than manual `remove_var`, improving panic-safety (red-team §Cross-Cutting Concern 2)
- Verdict: PASS

---
## SWE Remediation

### Verdicts Applied
- **KEEP** test 1 (`batch_sync_triggers_fts_optimize_after_reaching_threshold`): untouched except removing manual `remove_var` (now in Drop guard)
- **KEEP** test 2 (`single_file_below_threshold_does_not_trigger_optimize`): untouched except removing manual `remove_var`
- **KEEP** test 3 (`accumulated_small_batches_trigger_optimize_at_threshold`): untouched except removing manual `remove_var`
- **REWRITE** test 4 (`empty_batch_does_not_increment_optimize_counter`): primes counter to 2 via real file processing before empty batch, asserts counter stays at 2 (not the tautological 0→0)

### New Tests Added
- `threshold_one_triggers_optimize_on_every_sync`: boundary test for threshold=1 (off-by-one guard, Rec #4)
- `batch_exceeding_threshold_triggers_optimize`: batch_size=5 > threshold=3 overshoot guard (Rec #5)

### Infrastructure Improvements
- Extended `ConfigOverrideReset::drop()` with `reset_optimize_counter()` + `remove_var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY")` for panic-safe cleanup (Rec #2, #3)
- Removed 3x redundant manual `remove_var()` calls from KEEP tests (cleanup now panic-safe via Drop)

### Test Results
- 6/6 tests pass: `cargo test --test integration_tests -p maenifold-core -- incremental_sync_fts_optimize`
- 5/5 sibling processing tests pass (no regression from shared support.rs change)
- 3/3 sibling watcher tests pass (no regression from shared support.rs change)

### Commit: 216adad
