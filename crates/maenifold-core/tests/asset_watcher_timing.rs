// T-12.7: RTM NFR-1 - split asset watcher debounce/timing coverage into a focused test file.
#[path = "asset_watcher/support.rs"]
mod asset_watcher_support;

use asset_watcher_support::{AssetWatcherHarness, TEST_MUTEX};
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs RapidFileChanges_AreDebouncedIntoSingleNotification.
#[test]
fn rapid_file_changes_are_debounced_into_single_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-rapid.json");
    for version in 0..5 {
        fs::write(&test_file, format!(r#"{{"version":"{version}"}}"#)).expect("write rapid asset");
        if version < 4 {
            thread::sleep(Duration::from_millis(30));
        }
    }

    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));
    thread::sleep(Duration::from_millis(300));

    let content = fs::read_to_string(&test_file).expect("read final asset");
    assert!(content.contains('4'), "file should contain final version");
    assert_eq!(
        harness.watcher.notification_count(),
        1,
        "rapid changes inside one debounce window should collapse into a single notification"
    );
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs DebounceLatency_IsUnder500ms.
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
        elapsed < Duration::from_millis(500),
        "debounce latency was {}ms, expected < 500ms",
        elapsed.as_millis()
    );
    assert!(test_file.exists(), "file should exist after latency test");
}
