#[path = "incremental_sync_tools/support.rs"]
mod support;

use serde_json::json;
use support::{
    ConfigOverrideReset, TEST_MUTEX, context, format_watcher_error_message, isolated_config,
    process_path_for_test, start_watcher, stop_watcher, watcher_is_running,
};

// T-12.7: RTM NFR-1 - Split incremental sync watcher lifecycle assertions into a focused file.

#[test]
fn watcher_errors_are_formatted_with_expected_prefix() {
    let _guard = TEST_MUTEX.lock();

    let message = format_watcher_error_message("Test watcher error");

    assert_eq!(message, "[FileWatcher] Error: Test watcher error");
}

#[test]
fn process_file_created_with_path_outside_memory_path_does_not_throw() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let outside_dir = tempfile::TempDir::new().expect("outside tmp");
    let outside_path = outside_dir.path().join("outside-memory.md");
    std::fs::write(&outside_path, "# Outside\n\n[[some-concept]]\n").expect("write outside file");

    let result = process_path_for_test(&ctx, &outside_path);

    assert!(
        result.is_ok(),
        "processing a path outside the memory root should be ignored without error: {result:?}"
    );
}

#[test]
fn start_watcher_then_stop_watcher_lifecycle_returns_expected_messages() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);
    let _ = stop_watcher(&ctx, &json!({}));

    let start_result = start_watcher(&ctx, &json!({"debounceMs": 250})).expect("start watcher");
    assert!(start_result.contains("Started watching"));
    assert!(start_result.contains(&cfg.memory_path.display().to_string()));

    let second_start_result =
        start_watcher(&ctx, &json!({"debounceMs": 250})).expect("second start watcher");
    assert_eq!(second_start_result, "Watcher already running");
    assert!(watcher_is_running());

    let stop_result = stop_watcher(&ctx, &json!({})).expect("stop watcher");
    assert_eq!(stop_result, "Stopped incremental sync watcher");

    let second_stop_result = stop_watcher(&ctx, &json!({})).expect("second stop watcher");
    assert_eq!(second_stop_result, "Watcher not running");
    assert!(!watcher_is_running());
}
