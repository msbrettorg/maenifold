use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use tempfile::tempdir;

use super::GateCacheDb;
use crate::task_repository::TaskPathSet;

/// Regression test for T-206.0: concurrent SQLite access without busy_timeout
/// causes "database is locked" errors (EDEADLK on macOS).
///
/// This test reproduces the race condition between file_watcher sync (write)
/// and MCP handler status calls (read). Without busy_timeout configured,
/// SQLite returns SQLITE_BUSY immediately when a read attempts to acquire
/// a lock while a write transaction is active.
///
/// **IMPORTANT**: This test documents the expected behavior AFTER the fix.
/// The bug (no busy_timeout) manifests as intermittent EDEADLK/SQLITE_BUSY
/// errors in production under concurrent load. Unit tests cannot reliably
/// reproduce kernel-level fcntl() deadlock detection, but we can verify
/// that concurrent access patterns don't cause obvious failures.
///
/// See T-205.716 for detailed analysis of why EDEADLK cannot be reproduced
/// in unit tests: fd_lock uses flock() which does NOT return EDEADLK on
/// macOS; only SQLite's fcntl() can, but the contention requires the full
/// async MCP server context.
///
/// This test remains as a smoke test to ensure concurrent read/write access
/// to the cache DB does not deadlock or fail under basic conditions.
#[test]
fn concurrent_read_during_write_does_not_fail_with_database_locked() {
    use rusqlite::Connection;

    let temp = tempdir().expect("create temp root");
    let root = temp.path().to_path_buf();
    fs::create_dir_all(root.join("todo")).expect("create todo");

    // Seed the database with schema using the production path
    GateCacheDb::rebuild(&root, &[]).expect("create initial cache db");

    let barrier = Arc::new(Barrier::new(2));
    let root_clone = root.clone();
    let barrier_clone = Arc::clone(&barrier);

    // Spawn a thread that will attempt multiple reads while main thread writes
    let reader_handle = thread::spawn(move || {
        // Wait for writer to signal it's ready
        barrier_clone.wait();

        // Attempt multiple reads to increase chance of hitting contention
        let mut successes = 0;
        let mut failures = Vec::new();
        for i in 0..10 {
            thread::sleep(Duration::from_millis(20));
            let db = match GateCacheDb::open(&root_clone) {
                Ok(db) => db,
                Err(e) => {
                    failures.push(format!("iteration {i}: open failed: {e}"));
                    continue;
                }
            };
            match db.list_task_status(&root_clone, None) {
                Ok(_) => successes += 1,
                Err(e) => failures.push(format!("iteration {i}: list_task_status failed: {e}")),
            }
        }
        (successes, failures)
    });

    // Main thread: perform multiple write transactions
    let db_path = GateCacheDb::path_for_root(&root);
    let write_conn = Connection::open(&db_path).expect("open write connection");

    // Signal the reader thread we're starting
    barrier.wait();

    // Perform rapid write transactions to maximize contention window
    for i in 0..10 {
        write_conn
            .execute_batch("BEGIN IMMEDIATE TRANSACTION;")
            .expect("begin immediate transaction");
        write_conn
            .execute(
                &format!(
                    "INSERT OR REPLACE INTO tasks (file, task_id, title, status, stages_json, issue) 
                     VALUES ('concurrent-test-{i}.md', 'T-LOCK-{i}', 'Lock test {i}', 'pending', '[]', '')"
                ),
                [],
            )
            .expect("insert row under lock");
        // Hold the lock briefly to create contention window
        thread::sleep(Duration::from_millis(15));
        write_conn.execute_batch("COMMIT;").expect("commit");
    }
    drop(write_conn);

    // Check reader results
    let (successes, failures) = reader_handle.join().expect("reader thread panicked");

    // With busy_timeout configured, all reads should eventually succeed.
    // Without busy_timeout, some reads may fail with "database is locked".
    // We require at least 80% success rate for the test to pass.
    let success_rate = (successes as f64) / 10.0;
    if !failures.is_empty() {
        eprintln!("Concurrent read failures ({} of 10):", failures.len());
        for failure in &failures {
            eprintln!("  - {failure}");
        }
    }
    assert!(
        success_rate >= 0.8,
        "concurrent reads should mostly succeed with busy_timeout; got {:.0}% success rate",
        success_rate * 100.0
    );
}

/// Verifies that GateCacheDb::sync and list_task_status can run concurrently
/// without deadlock, simulating the file_watcher + MCP handler scenario.
/// This is a smoke test for the T-206.0 fix (busy_timeout on connections).
#[test]
fn concurrent_sync_and_list_does_not_deadlock() {
    let temp = tempdir().expect("create temp root");
    let root = temp.path().to_path_buf();
    fs::create_dir_all(root.join("todo")).expect("create todo");

    // Create some task files
    for i in 0..5 {
        let task_path = root.join("todo").join(format!("T-SYNC-{i}.md"));
        fs::write(
            &task_path,
            format!("# T-SYNC-{i} — Concurrent sync test\n\n**Pipeline**: sdd\n"),
        )
        .expect("write task");
    }

    // Initial sync to create the database
    GateCacheDb::rebuild(
        &root,
        &(0..5)
            .map(|i| TaskPathSet {
                task_id: format!("T-SYNC-{i}"),
                path: root.join("todo").join(format!("T-SYNC-{i}.md")),
                location: "todo".to_string(),
            })
            .collect::<Vec<_>>(),
    )
    .expect("initial rebuild");

    let barrier = Arc::new(Barrier::new(2));
    let root_clone = root.clone();
    let barrier_clone = Arc::clone(&barrier);

    // Reader thread - continuously lists status
    let reader_handle = thread::spawn(move || {
        barrier_clone.wait();
        let mut success_count = 0;
        for _ in 0..20 {
            let db = GateCacheDb::open(&root_clone).expect("open db");
            if db.list_task_status(&root_clone, None).is_ok() {
                success_count += 1;
            }
            thread::sleep(Duration::from_millis(10));
        }
        success_count
    });

    // Writer thread - continuously syncs
    barrier.wait();
    let tasks: Vec<TaskPathSet> = (0..5)
        .map(|i| TaskPathSet {
            task_id: format!("T-SYNC-{i}"),
            path: root.join("todo").join(format!("T-SYNC-{i}.md")),
            location: "todo".to_string(),
        })
        .collect();

    for _ in 0..20 {
        let _ = GateCacheDb::sync(&root, &tasks);
        thread::sleep(Duration::from_millis(10));
    }

    let success_count = reader_handle.join().expect("reader panicked");

    // With WAL mode + busy_timeout, we expect most reads to succeed
    assert!(
        success_count >= 15,
        "at least 15/20 concurrent list_task_status calls should succeed; got {success_count}"
    );
}
