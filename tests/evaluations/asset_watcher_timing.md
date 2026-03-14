# Test Evaluation: asset_watcher_timing.rs

**File**: `crates/maenifold-core/tests/asset_watcher_timing.rs`
**Evaluator**: Red Team — Gray (Skeptical Inquiry)
**Traceability**: T-12.7 | RTM NFR-3 | NFR-1 | FR-26
**Date**: 2025-07-10

---

## Summary

This file contains **2 timing-sensitive integration tests** that were split from a broader asset
watcher test suite (T-12.7) into a dedicated file focused on debounce coalescing and latency
bounds. Both tests are tagged `T-0.1: RTM NFR-3` internally — a stale sprint reference that
does not appear in `TODO.md`; the current active traceability anchor is T-12.7 / NFR-3.

### Architecture Concern (Cross-Cutting)

**Both tests exercise the test harness's own reimplemented debounce logic, not the production
code path.**

The `TestAssetWatcher` (in `support.rs`) instantiates a `PollWatcher` and calls `spawn_worker`
from `runtime.rs` — a separate reimplementation of the debounce algorithm. Production code in
`mcp/resources.rs` uses `notify::recommended_watcher` + `spawn_asset_watcher_worker`, which is
**structurally identical** but **not the same code**. If the production debounce implementation
silently diverges, neither of these tests will catch it.

This is the most consequential finding in the file. It is not a reason to delete the tests, but
it is a reason to be clear about what they actually cover.

**Timing constants** (from `support.rs`):

| Constant               | Value  |
|------------------------|--------|
| `DEFAULT_DEBOUNCE_MS`  | 150 ms |
| Poll interval          | 50 ms  |
| Grace window           | 50 ms  |
| `start_and_settle` sleep | 200 ms |

**Expected notification latency for a single write**: ~250 ms  
(`poll_interval` + `debounce_window` + `grace_window` = 50 + 150 + 50)

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Good? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `rapid_file_changes_are_debounced_into_single_notification` | ✅ | ✅ | ⚠️ Partial | ✅ | ✅ | **REWRITE** |
| 2 | `debounce_latency_is_under_500ms` | ✅ | ✅ | ⚠️ Partial | ✅ | ✅ | **REWRITE** |

---

## Test-by-Test Analysis

---

### Test 1 — `rapid_file_changes_are_debounced_into_single_notification`

**Traceability**: T-0.1 (stale) → T-12.7 / RTM NFR-3 / C# oracle port

**What it does**: Writes 5 versions of a JSON file at 30 ms intervals (total spread: ~120 ms),
then asserts that exactly 1 notification is emitted — all writes collapse into a single debounce
flush.

#### Load-Bearing?
**Yes.** This is the primary behavioral contract for debounce coalescing. If this test is
removed, there is no test asserting that rapid writes within a debounce window produce a single
notification rather than N notifications. It is a direct C# oracle port.

#### Unique?
**Yes.** No other test in this file covers burst-coalescing semantics. The event-level test file
(`asset_watcher_events.rs`) tests event classification, not notification count.

#### Assertions — Are They Correct?

**Assertion 1** — `assert!(content.contains('4'))`:  
❌ **Irrelevant.** This checks the filesystem state of a file that was just written by this
test. Nothing in the watcher infrastructure can modify file content. This assertion can never
meaningfully fail in this context and teaches the reader a false lesson — that the watcher might
corrupt the file's content. It should be removed.

**Assertion 2** — `assert_eq!(harness.watcher.notification_count(), 1, ...)`:  
✅ **Correct and load-bearing.** This is the real assertion. However, the check occurs after a
**hard-coded `thread::sleep(Duration::from_millis(300))`** whose sole purpose is to let any
"extra" notifications accumulate before the count is read. The 300 ms window is larger than one
debounce cycle (~250 ms) but not by a meaningful margin on a loaded CI runner. On a machine
where the debounce flush is itself delayed, the sleep could expire while the count is still 0,
and the test would trivially pass for the wrong reason (early pass). The sleep should be replaced
with a structured wait.

#### Timing Assumptions (Questioned)

The test implicitly assumes:
1. All 5 poll events arrive within a single 150 ms debounce window after the first event.
2. The watcher is fully settled after a 200 ms sleep (reasonable for PollWatcher at 50 ms poll,
   but not guaranteed on a loaded system).
3. No spurious notifications arrive during the 300 ms post-wait sleep.

Assumption 1 holds in practice because the total write spread (~120 ms) + poll latency (~50 ms)
= ~170 ms, and the debounce window resets with each incoming event. But this isn't documented.

#### Parameterizable?
**Yes.** `write_count` (5), `write_interval_ms` (30), `expected_notifications` (1),
`post_wait_ms` (300) are all magic numbers that could be named constants or test parameters.
A second scenario (writes spread beyond the debounce window → 2 notifications) would increase
confidence, but is out of scope for this file alone.

#### Verdict: **REWRITE**

Remove the dead `content.contains('4')` assertion. Replace the post-wait `thread::sleep` with
a bounded poll that waits until either a second notification arrives or a full debounce cycle
elapses, then asserts the count. Name the timing constants. Add a comment explaining the
expected flush timeline so the test is self-documenting.

---

### Test 2 — `debounce_latency_is_under_500ms`

**Traceability**: T-0.1 (stale) → T-12.7 / RTM NFR-3 / C# oracle port

**What it does**: Writes a single file, records wall-clock time, waits for a notification, and
asserts that the elapsed time from write to notification is under 500 ms.

#### Load-Bearing?
**Yes.** This is the only test asserting a latency SLA for the watcher. If removed, there is no
upper-bound guarantee on how long it takes for a file write to produce a notification. This
directly gates the user-observable responsiveness of asset hot-loading.

#### Unique?
**Yes.** Test 1 validates coalescing count; Test 2 validates time-to-notification. These are
orthogonal properties.

#### Assertions — Are They Correct?

**Assertion 1** — `assert!(elapsed < Duration::from_millis(500))`:  
✅ **Correct** in intent, but the margin deserves scrutiny.

Expected latency breakdown:
```
poll_interval (50ms) + debounce_window (150ms) + grace_window (50ms) = 250ms
```
The 500 ms budget gives exactly **100% headroom** above the theoretical minimum (~250 ms). On a
heavily loaded CI machine (e.g., shared GitHub Actions runner), OS scheduling jitter, thread
contention, and `mpsc::recv_timeout` granularity can add 50–100 ms. The margin is adequate but
not generous. A flaky failure here would not indicate a real regression — it would indicate CI
load.

**Missing**: There is no lower-bound assertion. If `wait_for_notifications` returns
**immediately** because the watcher misfires a count increment at startup, `elapsed` could be
~0 ms and the test passes vacuously. A lower bound of, say, `elapsed > 50ms` (at least one poll
cycle must have elapsed) would catch this degenerate case.

**Assertion 2** — `assert!(test_file.exists())`:  
❌ **Dead code.** Nothing in this test or the watcher infrastructure deletes the file. This
assertion cannot fail under any realistic condition. It adds noise, wastes reader attention, and
may mislead maintainers into thinking file deletion is a plausible side-effect of watcher
operation. It should be removed unconditionally.

#### Parameterizable?
**Yes.** `500ms` (budget) should be a named constant. The wall-clock timer start (`started`) is
placed correctly — immediately before the write, not before `start_and_settle()`. This is good.

#### Verdict: **REWRITE**

Remove the dead `test_file.exists()` assertion. Add a lower-bound assertion (`elapsed > poll_interval`). Promote `500ms` to a named constant. Add an explanatory comment with the expected timing breakdown so reviewers understand what "under 500ms" actually claims, and why.

---

## Recommendations

### P1 — Fix Dead Assertions (Both Tests)
Remove `assert!(content.contains('4'))` from Test 1 and `assert!(test_file.exists())` from Test 2. Neither can fail under normal conditions. They mislead readers about what the watcher can do to file contents/existence, and they mask the real intent of each test.

### P2 — Replace Magic Post-Notification Sleep with Bounded Poll (Test 1)
The `thread::sleep(Duration::from_millis(300))` in Test 1 is an anti-pattern. Replace with a
poll loop that checks for a second notification during a full debounce cycle:

```rust
// Wait one full debounce cycle to catch any spurious extra notifications.
let extra_wait = Duration::from_millis(DEFAULT_DEBOUNCE_MS * 2);
let deadline = Instant::now() + extra_wait;
while Instant::now() < deadline {
    thread::sleep(Duration::from_millis(25));
}
assert_eq!(harness.watcher.notification_count(), 1, "...");
```

This is both more explicit about its intent and more robust on slow machines.

### P3 — Add Lower-Bound Latency Assertion (Test 2)
```rust
assert!(
    elapsed >= Duration::from_millis(POLL_INTERVAL_MS),
    "notification arrived in {}ms — suspiciously fast, watcher may not have fired correctly",
    elapsed.as_millis()
);
```

### P4 — Fix Traceability (Both Tests)
Replace `// T-0.1: RTM NFR-3` (stale sprint tag) with `// T-12.7: RTM NFR-3` to align with the
active backlog entry. The stale reference creates noise when searching the codebase for T-0.1.

### P5 — Document the Architectural Gap (Informational)
Add a module-level comment to `asset_watcher_timing.rs` noting that:
1. These tests exercise the **test harness's own debounce implementation** (`runtime.rs`), not
   the production `spawn_asset_watcher_worker` in `mcp/resources.rs`.
2. The two implementations are structurally equivalent but **not shared code**.
3. Production-path coverage requires the integration tests in `asset_watcher_events.rs` or
   a dedicated end-to-end test.

This gap is not a defect in these tests — they correctly validate the debounce contract — but
the scope boundary should be explicit for future maintainers.

### P6 — Add FR-26 Polling Fallback Coverage (Future)
Neither test covers FR-26 (polling-based fallback watcher). T-17.4 is pending. When
`MAENIFOLD_WATCHER_POLL_SECONDS` is introduced, the coalescing and latency tests should be
extended (or a new `asset_watcher_polling_fallback.rs` file created) to validate that polling
mode satisfies the same behavioral contracts.

### P7 — Name Magic Constants
Promote the inline durations to named constants:

```rust
const DEBOUNCE_WINDOW_MS: u64 = DEFAULT_DEBOUNCE_MS;       // 150ms
const LATENCY_BUDGET_MS: u64 = 500;
const BURST_WRITE_INTERVAL_MS: u64 = 30;
const BURST_WRITE_COUNT: usize = 5;
```

---

## Load-Bearing Score: 2/2

Both tests carry real behavioral contracts (debounce coalescing and latency SLA) that are not
covered elsewhere in the test suite. Neither should be deleted. Both need targeted rewrites to
remove dead assertions and sharpen the timing safety of their pass conditions.

| Test | Load-Bearing | Notes |
|------|:---:|-------|
| `rapid_file_changes_are_debounced_into_single_notification` | ✅ | Only test verifying N writes → 1 notification |
| `debounce_latency_is_under_500ms` | ✅ | Only test enforcing a latency SLA on notification delivery |

**Net quality deficit**: The tests verify correct contracts but carry dead assertions that erode
reader trust and partially obscure the real intent. The architectural gap (test harness code
vs. production code) is the highest-priority informational finding — not a blocker, but worth
explicitly documenting before FR-26 lands and multiplies the watcher surface area.
