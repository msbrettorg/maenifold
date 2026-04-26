// T-01.16: AssetManager tests extracted verbatim from manager.rs.

use super::super::types::AssetKind;
use super::AssetManager;
use crate::config::Config;
use crate::test_util::ConfigResetGuard;
use serde_json::Value;
use serial_test::serial;
use std::fs;
use std::sync::Arc;
use std::thread;

/// T-205.716: Regression test for ListAssets deadlock under concurrent access.
///
/// This test verifies that multiple threads can safely call list_assets()
/// concurrently without triggering "Resource deadlock avoided (os error 11)".
/// The bug was originally observed when multiple agents accessed the asset
/// system simultaneously.
///
/// EXPECTED BEHAVIOR:
/// - Before fix: This test should demonstrate the concurrency hazard
/// - After fix: All threads complete successfully without deadlock errors
#[test]
#[serial]
fn concurrent_list_assets_does_not_deadlock() {
    let dir = tempfile::tempdir().expect("tempdir");
    let assets_path = dir.path().join("assets");

    // Set up asset directories with some sample files
    for kind in AssetKind::all() {
        let kind_dir = assets_path.join(kind.dir_name());
        fs::create_dir_all(&kind_dir).expect("create asset dir");

        // Create a few asset files per kind
        for i in 0..5 {
            let file_path = kind_dir.join(format!("test-asset-{i}.json"));
            let content = format!(
                r#"{{"id":"test-asset-{i}","name":"Test Asset {i}","description":"Test"}}"#
            );
            fs::write(&file_path, content).expect("write asset file");
        }
    }

    Config::override_root(dir.path());
    let _guard = ConfigResetGuard;

    let config = Arc::new(Config::current());
    let thread_count = 8;
    let iterations_per_thread = 50;

    let mut handles = Vec::with_capacity(thread_count);

    for thread_id in 0..thread_count {
        let config = Arc::clone(&config);
        let handle = thread::spawn(move || {
            for i in 0..iterations_per_thread {
                let manager = AssetManager::new(&config);

                // Alternate between typed and untyped calls
                let result = if i % 2 == 0 {
                    manager.list_assets(None)
                } else {
                    let kind = AssetKind::all()[i % 4];
                    manager.list_assets(Some(kind))
                };

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = e.to_string();
                        // Fail explicitly if we see the deadlock error
                        if err_str.contains("Resource deadlock avoided")
                            || err_str.contains("os error 11")
                        {
                            panic!(
                                "Thread {thread_id} iteration {i}: Deadlock detected: {err_str}"
                            );
                        }
                        // Other errors may be acceptable during concurrent access
                        // but log them for debugging
                        eprintln!("Thread {thread_id} iteration {i}: Error: {err_str}");
                    }
                }
            }
        });
        handles.push(handle);
    }

    // Join all threads and propagate any panics
    for (id, handle) in handles.into_iter().enumerate() {
        handle
            .join()
            .unwrap_or_else(|e| panic!("Thread {id} panicked: {e:?}"));
    }
}

/// T-205.716: Verify untyped list_assets returns exactly the four asset kinds.
#[test]
#[serial]
fn untyped_list_assets_returns_four_kinds() {
    let dir = tempfile::tempdir().expect("tempdir");
    Config::override_root(dir.path());
    let _guard = ConfigResetGuard;

    let config = Config::current();
    let manager = AssetManager::new(&config);

    let result = manager
        .list_assets(None)
        .expect("list_assets should succeed");
    let Value::Array(kinds) = result else {
        panic!("Expected array, got: {result:?}");
    };

    let kind_names: Vec<&str> = kinds.iter().filter_map(|v| v.as_str()).collect();

    assert_eq!(kind_names.len(), 4, "Should return exactly 4 asset kinds");
    assert!(kind_names.contains(&"workflow"), "Missing workflow");
    assert!(kind_names.contains(&"role"), "Missing role");
    assert!(kind_names.contains(&"color"), "Missing color");
    assert!(kind_names.contains(&"perspective"), "Missing perspective");
}

/// T-205.716: Verify typed list_assets returns metadata array for each kind.
#[test]
#[serial]
fn typed_list_assets_returns_metadata_array() {
    let dir = tempfile::tempdir().expect("tempdir");
    let assets_path = dir.path().join("assets");

    // Create a test asset
    let workflows_dir = assets_path.join("workflows");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    fs::write(
        workflows_dir.join("test-workflow.json"),
        r#"{"id":"test-workflow","name":"Test Workflow","description":"A test"}"#,
    )
    .expect("write workflow");

    Config::override_root(dir.path());
    let _guard = ConfigResetGuard;

    let config = Config::current();
    let manager = AssetManager::new(&config);

    let result = manager
        .list_assets(Some(AssetKind::Workflow))
        .expect("list_assets should succeed");

    let Value::Array(items) = result else {
        panic!("Expected array, got: {result:?}");
    };

    assert_eq!(items.len(), 1, "Should return exactly 1 workflow");
    let item = &items[0];
    assert_eq!(item["id"], "test-workflow");
    assert_eq!(item["name"], "Test Workflow");
}

/// T-205.716: Regression test simulating MCP server concurrency pattern.
///
/// The actual MCP server uses `tokio::task::spawn_blocking` for list_resources.
/// This test simulates multiple concurrent "agents" making asset listing calls
/// while also performing file-system modifications (simulating asset updates).
///
/// The deadlock in production occurs with "Resource deadlock avoided (os error 11)"
/// which indicates a circular lock dependency at the kernel level.
#[test]
#[serial]
fn concurrent_list_and_modify_assets_does_not_deadlock() {
    use notify::{RecursiveMode, Watcher};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    let dir = tempfile::tempdir().expect("tempdir");
    let assets_path = dir.path().join("assets");

    // Set up asset directories with sample files
    for kind in AssetKind::all() {
        let kind_dir = assets_path.join(kind.dir_name());
        fs::create_dir_all(&kind_dir).expect("create asset dir");
        for i in 0..10 {
            let file_path = kind_dir.join(format!("asset-{i}.json"));
            let content =
                format!(r#"{{"id":"asset-{i}","name":"Asset {i}","description":"Test"}}"#);
            fs::write(&file_path, content).expect("write asset file");
        }
    }

    Config::override_root(dir.path());
    let _guard = ConfigResetGuard;

    let config = Arc::new(Config::current());
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Create a file watcher to simulate the MCP asset watcher
    let stop_flag_watcher = Arc::clone(&stop_flag);
    let _watcher = notify::recommended_watcher(move |_event: notify::Result<notify::Event>| {
        // Just consume events, simulating the MCP watcher
        if stop_flag_watcher.load(Ordering::Relaxed) {
            return;
        }
    })
    .expect("create watcher");

    // Watch the assets directory
    let mut watcher = notify::recommended_watcher(|_| {}).expect("watcher");
    let _ = watcher.watch(&assets_path, RecursiveMode::Recursive);

    let mut handles = Vec::new();

    // Thread 1: Continuous asset modifications (simulating UpdateAssets)
    let stop_flag_mod = Arc::clone(&stop_flag);
    let assets_path_mod = assets_path.clone();
    let modifier_handle = thread::spawn(move || {
        let mut counter = 0_u32;
        while !stop_flag_mod.load(Ordering::Relaxed) {
            for kind in AssetKind::all() {
                let kind_dir = assets_path_mod.join(kind.dir_name());
                let temp_path = kind_dir.join(format!("temp-{counter}.json"));
                let content = format!(r#"{{"id":"temp-{counter}","name":"Temp {counter}"}}"#);
                // Create and immediately remove to trigger fs events
                if fs::write(&temp_path, content).is_ok() {
                    let _ = fs::remove_file(&temp_path);
                }
                counter = counter.wrapping_add(1);
            }
            thread::sleep(Duration::from_micros(100));
        }
    });
    handles.push(modifier_handle);

    // Threads 2-9: Concurrent list_assets calls
    for thread_id in 0..8 {
        let config = Arc::clone(&config);
        let stop_flag_reader = Arc::clone(&stop_flag);
        let handle = thread::spawn(move || {
            let mut iteration = 0_u32;
            while !stop_flag_reader.load(Ordering::Relaxed) && iteration < 100 {
                let manager = AssetManager::new(&config);

                // Alternate between typed and untyped calls
                let result = if iteration % 2 == 0 {
                    manager.list_assets(None)
                } else {
                    let kind = AssetKind::all()[(iteration as usize) % 4];
                    manager.list_assets(Some(kind))
                };

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = e.to_string();
                        if err_str.contains("Resource deadlock avoided")
                            || err_str.contains("os error 11")
                            || err_str.contains("EDEADLK")
                        {
                            panic!("Thread {thread_id} iteration {iteration}: DEADLOCK: {err_str}");
                        }
                    }
                }
                iteration += 1;
                thread::yield_now();
            }
        });
        handles.push(handle);
    }

    // Wait for reader threads to complete (should finish in ~1 second)
    thread::sleep(Duration::from_secs(2));
    stop_flag.store(true, Ordering::Relaxed);

    for (id, handle) in handles.into_iter().enumerate() {
        handle
            .join()
            .unwrap_or_else(|e| panic!("Thread {id} panicked: {e:?}"));
    }

    // Final verification: list_assets should still work after concurrent stress
    let manager = AssetManager::new(&config);
    let result = manager.list_assets(None);
    assert!(
        result.is_ok(),
        "list_assets should succeed after concurrent stress: {:?}",
        result.err()
    );
}
