#[path = "incremental_sync_tools/support.rs"]
mod support;

use serde_json::json;
use support::{
    ConfigOverrideReset, TEST_MUTEX, context, db_connection, format_watcher_error_message,
    isolated_config, process_path_for_test, start_watcher, stop_watcher, watcher_is_running,
};

// T-12.7: RTM NFR-1 - Split incremental sync watcher lifecycle assertions into a focused file.

// T-12.7: Format-contract lock test — pure function, no mutex needed.
#[test]
fn watcher_error_message_format_contract() {
    // P2: Removed TEST_MUTEX — format_watcher_error_message is a pure, stateless function
    // with no shared state; holding the mutex adds unnecessary contention against lifecycle tests.
    // P5: Parameterized across nominal, empty, and nested-prefix inputs.
    assert_eq!(
        format_watcher_error_message("disk full"),
        "[FileWatcher] Error: disk full"
    );
    assert_eq!(format_watcher_error_message(""), "[FileWatcher] Error: ");
    assert_eq!(
        format_watcher_error_message("Error: nested"),
        "[FileWatcher] Error: Error: nested"
    );
}

// T-12.7: RTM NFR-1 — Boundary guard: out-of-memory-root paths must be silently ignored.
#[test]
fn process_path_outside_memory_root_is_silently_ignored() {
    // P3: Renamed from `does_not_throw` to Rust-idiomatic `is_silently_ignored`.
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);

    let outside_dir = tempfile::TempDir::new().expect("outside tmp");
    let outside_path = outside_dir.path().join("outside-memory.md");
    std::fs::write(&outside_path, "# Outside\n\n[[some-concept]]\n").expect("write outside file");

    let result = process_path_for_test(&ctx, &outside_path);

    assert!(result.is_ok(), "should return Ok without error: {result:?}");

    // P1: Verify the file was NOT indexed — the critical contract is that out-of-scope
    // files produce no side-effects in the database, not merely that they don't error.
    let conn = db_connection(&cfg);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM file_content", [], |r| r.get(0))
        .expect("count");
    assert_eq!(count, 0, "out-of-memory-root file must not be indexed");
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
    // T-5.7: RTM FR-5.7 — migrated debounce format check from boundary_guards (unique assertion)
    assert!(
        start_result.contains(" for incremental sync (debounce: 250ms)"),
        "start output should include debounce format: {start_result}"
    );

    // P4: Assert watcher_is_running immediately after first start — closes the gap where
    // a start that returns Ok but fails to register in WATCHER would go undetected.
    assert!(
        watcher_is_running(),
        "watcher should be running after first start"
    );

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
