// T-12.7: RTM NFR-1 — split asset watcher debounce/timing coverage into a focused test file.
//
// ARCHITECTURAL NOTE: These tests exercise the **test harness's own debounce implementation**
// (`asset_watcher/runtime.rs`), not the production `spawn_asset_watcher_worker` in
// `mcp/resources.rs`. The two implementations are structurally equivalent but **not shared
// code**. Production-path coverage requires the integration tests in `asset_watcher_events.rs`
// or a dedicated end-to-end test.
#[path = "asset_watcher/support.rs"]
mod asset_watcher_support;

use asset_watcher_support::{AssetWatcherHarness, DEFAULT_DEBOUNCE_MS, TEST_MUTEX};
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

// T-12.7: RTM NFR-3 — Named timing constants for debounce coalescing test.
const BURST_WRITE_COUNT: usize = 5;
const BURST_WRITE_INTERVAL_MS: u64 = 30;
/// Poll interval used by `PollWatcher` in the test harness (see `support.rs`).
const POLL_INTERVAL_MS: u64 = 50;
/// How long to wait after the first notification for any spurious extra notifications.
/// Two full debounce cycles gives ample room on slow CI runners.
const POST_NOTIFICATION_WATCH_MS: u64 = DEFAULT_DEBOUNCE_MS * 2;
/// Upper-bound latency budget for a single write → notification round-trip.
/// Expected: poll_interval (50ms) + debounce_window (150ms) + grace_window (50ms) = 250ms.
/// The 500ms budget gives ~100% headroom for OS scheduling jitter on loaded machines.
const LATENCY_BUDGET_MS: u64 = 500;

// T-12.7: RTM NFR-3 — ports AssetWatcherToolsTests.cs RapidFileChanges_AreDebouncedIntoSingleNotification.
//
// Writes BURST_WRITE_COUNT versions of a JSON file at BURST_WRITE_INTERVAL_MS intervals
// (total spread: ~120ms). All writes land within a single debounce window (150ms, resetting
// on each incoming event) so they should collapse into exactly 1 notification.
#[test]
fn rapid_file_changes_are_debounced_into_single_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-rapid.json");
    for version in 0..BURST_WRITE_COUNT {
        fs::write(&test_file, format!(r#"{{"version":"{version}"}}"#)).expect("write rapid asset");
        if version < BURST_WRITE_COUNT - 1 {
            thread::sleep(Duration::from_millis(BURST_WRITE_INTERVAL_MS));
        }
    }

    // Wait for the first (expected-only) notification to arrive.
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));

    // Wait two full debounce cycles to catch any spurious extra notifications.
    // This replaces a hard-coded sleep(300ms) with an explicit, documented window.
    let deadline = Instant::now() + Duration::from_millis(POST_NOTIFICATION_WATCH_MS);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(25));
    }

    assert_eq!(
        harness.watcher.notification_count(),
        1,
        "rapid changes inside one debounce window should collapse into a single notification"
    );
}

// T-12.7: RTM NFR-3 — ports AssetWatcherToolsTests.cs DebounceLatency_IsUnder500ms.
//
// Writes a single file, records wall-clock time, waits for a notification, and asserts the
// elapsed time is within [POLL_INTERVAL_MS .. LATENCY_BUDGET_MS]. The lower bound ensures
// the notification wasn't a spurious startup artifact; the upper bound enforces the SLA.
#[test]
fn debounce_latency_is_under_500ms() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-latency.json");
    let started = Instant::now();
    fs::write(&test_file, r#"{"name":"latency-test"}"#).expect("write latency asset");
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_millis(POLL_INTERVAL_MS),
        "notification arrived in {}ms — suspiciously fast (< {}ms poll interval), \
         watcher may not have fired correctly",
        elapsed.as_millis(),
        POLL_INTERVAL_MS,
    );
    assert!(
        elapsed < Duration::from_millis(LATENCY_BUDGET_MS),
        "debounce latency was {}ms, expected < {}ms \
         (poll_interval={}ms + debounce={}ms + grace={}ms = ~250ms + headroom)",
        elapsed.as_millis(),
        LATENCY_BUDGET_MS,
        POLL_INTERVAL_MS,
        DEFAULT_DEBOUNCE_MS,
        POLL_INTERVAL_MS,
    );
}
