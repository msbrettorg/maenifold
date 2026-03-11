// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::community::detect_and_persist;
use support::*;

// ─────────────────────────────────────────────────────────────────────────────
// §3  CommunityDetectionRunner tests — mirrors CommunityDetectionRunnerTests.cs
//
// The Rust implementation exposes detect_and_persist rather than a Runner class
// with explicit locks. These tests validate the observable contract:
// detect_and_persist behaves correctly in isolation, on re-run, across connections,
// and under concurrent calls — mapping to the 7 acceptance criteria in the C# suite.
// ─────────────────────────────────────────────────────────────────────────────

// T-W1-8: RTM NFR-2
// AC-4: Community data is immediately available in the DB after detect_and_persist returns.
#[test]
fn runner_community_data_immediately_available_after_run() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
    // Immediately after return, data must be queryable.
    let conn = open_conn(&env.db_path);
    let rows = community_row_count(&conn);
    assert_eq!(
        rows, 5,
        "5 rows must be immediately visible after detect_and_persist returns"
    );
    assert_eq!(count, 2, "community count must be 2");
}

// T-W1-8: RTM NFR-2
// AC-5: Community data persists across new DB connections (durable write).
#[test]
fn runner_community_data_persists_across_connections() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    // Open a brand-new connection — data must still be there.
    let conn2 = open_conn(&env.db_path);
    let rows = community_row_count(&conn2);
    assert_eq!(
        rows, 5,
        "community data must survive across separate DB connections"
    );
}

// T-W1-8: RTM NFR-2
// AC-6: Concurrent calls to detect_and_persist must not panic or corrupt data.
// (Mirrors RunBestEffort_SkipsWhenLockHeld — in Rust, SQLite busy_timeout handles contention.)
#[test]
fn runner_concurrent_calls_do_not_panic_or_corrupt() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    let db_path1 = env.db_path.clone();
    let db_path2 = env.db_path.clone();

    // Launch two threads. One or both may see a busy timeout, but neither must panic.
    let t1 = std::thread::spawn(move || detect_and_persist(&db_path1, 1.0, None));
    let t2 = std::thread::spawn(move || detect_and_persist(&db_path2, 1.0, None));

    // At least one must succeed; neither must panic.
    let r1 = t1.join().expect("thread 1 must not panic");
    let r2 = t2.join().expect("thread 2 must not panic");
    assert!(
        r1.is_ok() || r2.is_ok(),
        "at least one concurrent detect_and_persist call must succeed"
    );

    // Data integrity: count must still be correct.
    let conn = open_conn(&env.db_path);
    let rows = community_row_count(&conn);
    assert_eq!(
        rows, 5,
        "5 concept_communities rows must be present after concurrent runs"
    );
}

// T-W1-8: RTM NFR-2
// AC-7: detect_and_persist returns a valid community count (>= 0) on any initialized DB.
// Mirrors: RunDetection_ReturnsValidTuple
#[test]
fn runner_detect_and_persist_returns_valid_community_count() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    // Empty DB — valid baseline.
    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect on empty DB");
    assert_eq!(count, 0, "empty graph must return count 0");

    // Now add data.
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect with data");
    assert_eq!(count, 2, "triangle + isolated pair must give count 2");
}
