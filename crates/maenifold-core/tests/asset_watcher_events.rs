// T-12.7: RTM NFR-1 - split asset watcher filesystem event coverage into a focused test file.
#[path = "asset_watcher/support.rs"]
mod asset_watcher_support;

use asset_watcher_support::{AssetWatcherHarness, TEST_MUTEX};
use maenifold_core::{AssetKind, MaenifoldError};
use std::fs;
use std::time::Duration;

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs FileCreated_TriggersDebounceAndNotification.
#[test]
fn file_created_triggers_debounce_and_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-create.json");
    let test_content = r#"{"name":"test-workflow","version":"1.0.0"}"#;
    fs::write(&test_file, test_content).expect("create workflow asset");

    let batch = harness
        .watcher
        .wait_for_batch_with_path(&test_file, Duration::from_secs(5));

    assert!(test_file.exists(), "file should exist after creation");
    assert!(
        batch.iter().any(|path| path == &test_file),
        "debounced batch should include the created file: {batch:?}"
    );
    let asset = harness
        .asset_manager()
        .read_asset(AssetKind::Workflow, "test-create")
        .expect("created asset should be readable");
    assert_eq!(asset["name"], "test-workflow");
    assert_eq!(asset["version"], "1.0.0");
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs FileChanged_TriggersDebounceAndNotification.
#[test]
fn file_changed_triggers_debounce_and_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-modify.json");
    fs::write(&test_file, r#"{"version":"1.0.0"}"#).expect("write initial asset");
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));

    fs::write(&test_file, r#"{"version":"2.0.0"}"#).expect("write updated asset");
    let batch = harness
        .watcher
        .wait_for_batch_with_path(&test_file, Duration::from_secs(5));

    let content = fs::read_to_string(&test_file).expect("read updated asset");
    assert!(
        content.contains("2.0.0"),
        "file should contain updated content"
    );
    assert!(
        batch.iter().any(|path| path == &test_file),
        "debounced batch should include the changed file: {batch:?}"
    );
    let asset = harness
        .asset_manager()
        .read_asset(AssetKind::Workflow, "test-modify")
        .expect("updated asset should be readable");
    assert_eq!(asset["version"], "2.0.0");
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs FileDeleted_TriggersDebounceAndNotification.
#[test]
fn file_deleted_triggers_debounce_and_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-delete.json");
    fs::write(&test_file, r#"{"name":"delete-me"}"#).expect("write asset");
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));

    fs::remove_file(&test_file).expect("delete asset");
    let batch = harness
        .watcher
        .wait_for_batch_with_path(&test_file, Duration::from_secs(5));

    assert!(!test_file.exists(), "file should not exist after deletion");
    assert!(
        batch.iter().any(|path| path == &test_file),
        "debounced batch should include the deleted file path: {batch:?}"
    );
    let error = harness
        .asset_manager()
        .read_asset(AssetKind::Workflow, "test-delete")
        .expect_err("deleted asset should no longer be readable");
    assert!(
        matches!(&error, MaenifoldError::AssetNotFound(kind, id) if kind == "workflow" && id == "test-delete"),
        "expected missing asset error after deletion, got: {error:?}"
    );
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs FileRenamed_TriggersDebounceAndNotification.
#[test]
fn file_renamed_triggers_debounce_and_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let old_file = harness.workflows_path.join("test-old-name.json");
    let new_file = harness.workflows_path.join("test-new-name.json");
    fs::write(&old_file, r#"{"name":"rename-test"}"#).expect("write old asset");
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));

    fs::rename(&old_file, &new_file).expect("rename asset");
    harness
        .watcher
        .wait_for_notifications(2, Duration::from_secs(5));
    let batches = harness.watcher.drain_batches();

    assert!(new_file.exists(), "renamed file should exist at new path");
    assert!(!old_file.exists(), "old path should not exist after rename");
    assert!(
        batches
            .iter()
            .flatten()
            .any(|path| path == &old_file || path == &new_file),
        "rename should surface old or new file path in a debounced batch: {batches:?}"
    );
    assert!(
        harness
            .asset_manager()
            .read_asset(AssetKind::Workflow, "test-new-name")
            .is_ok(),
        "renamed asset should be readable by new filename"
    );
}
