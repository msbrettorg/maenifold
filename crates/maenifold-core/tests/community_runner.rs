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
// - Empty-DB boundary: detect_and_persist on an empty graph returns 0 without
//   panicking or corrupting the schema.
// - Concurrency: concurrent calls must not panic or corrupt data.
// - Seed determinism: a fixed seed produces identical community assignments.
//
// Persistence and data-path assertions for the triangle_plus_isolated_pair
// topology are covered with stronger assertions in:
//   community_louvain_persistence::run_and_persist_persists_to_database
//   community_louvain_basics::run_louvain_triangle_plus_isolated_pair_two_communities
// ─────────────────────────────────────────────────────────────────────────────

// T-W1-8: RTM NFR-2
// AC-7 (boundary): detect_and_persist on an empty, freshly initialized DB must
// return 0 communities and leave the schema intact — not panic or corrupt.
#[test]
fn runner_returns_zero_on_empty_db() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();

    // T-12.7: RTM NFR-1 — empty graph is the boundary condition for detect_and_persist.
    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect on empty DB");
    assert_eq!(count, 0, "empty graph must return community count 0");

    // Verify no spurious rows were written.
    let conn = open_conn(&env.db_path);
    let rows = community_row_count(&conn);
    assert_eq!(rows, 0, "no community rows should exist for an empty graph");
}

// T-W1-8: RTM NFR-2
// AC-6: Concurrent calls to detect_and_persist must not panic or corrupt data.
// (Mirrors RunBestEffort_SkipsWhenLockHeld — in Rust, SQLite busy_timeout handles contention.)
//
// Both threads are expected to succeed: open_conn sets PRAGMA busy_timeout=5000ms
// and the triangle_plus_isolated_pair topology completes in microseconds, so
// SQLite contention is resolved well within the timeout window. The && assertion
// catches regressions where concurrent access starts returning errors.
#[test]
fn runner_concurrent_calls_do_not_panic_or_corrupt() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    let db_path1 = env.db_path.clone();
    let db_path2 = env.db_path.clone();

    let t1 = std::thread::spawn(move || detect_and_persist(&db_path1, 1.0, None));
    let t2 = std::thread::spawn(move || detect_and_persist(&db_path2, 1.0, None));

    // T-12.7: RTM NFR-1 — both must succeed given busy_timeout=5000ms for this dataset size.
    let r1 = t1.join().expect("thread 1 must not panic");
    let r2 = t2.join().expect("thread 2 must not panic");
    assert!(
        r1.is_ok() && r2.is_ok(),
        "both concurrent detect_and_persist calls must succeed (busy_timeout=5000ms); r1={r1:?}, r2={r2:?}"
    );

    // Data integrity: the final state must have exactly 5 concept_communities rows.
    let conn = open_conn(&env.db_path);
    let rows = community_row_count(&conn);
    assert_eq!(
        rows, 5,
        "5 concept_communities rows must be present after concurrent runs"
    );
}

// T-W1-8: RTM NFR-2
// T-15.10: RTM FR-23.7 — seed parameter for deterministic Louvain node ordering.
// A fixed seed must produce identical community assignments across two runs on
// the same topology. This verifies the determinism contract introduced by T-15.10.
#[test]
fn runner_seed_produces_deterministic_assignments() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    // First run with seed=42.
    let count1 = detect_and_persist(&env.db_path, 1.0, Some(42)).expect("seeded run 1");
    let conn = open_conn(&env.db_path);
    let ids1: Vec<Option<i64>> = ["alpha", "beta", "gamma", "delta", "epsilon"]
        .iter()
        .map(|c| get_community_id(&conn, c))
        .collect();
    drop(conn);

    // Second run with the same seed — assignments must be identical.
    let count2 = detect_and_persist(&env.db_path, 1.0, Some(42)).expect("seeded run 2");
    let conn = open_conn(&env.db_path);
    let ids2: Vec<Option<i64>> = ["alpha", "beta", "gamma", "delta", "epsilon"]
        .iter()
        .map(|c| get_community_id(&conn, c))
        .collect();

    assert_eq!(count1, count2, "seeded runs must return the same community count");
    assert_eq!(
        ids1, ids2,
        "seeded runs must produce identical community assignments"
    );
}
