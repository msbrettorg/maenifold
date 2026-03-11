// T-12.0: RTM NFR-1
#[path = "proximity_weight/support.rs"]
mod support;

use maenifold_core::community::{compute_nmi, detect_and_persist};
use serde_json::json;
use std::collections::BTreeMap;
use support::{
    acquire_test_lock, assert_approx_eq, community_id, insert_concept,
    insert_edge_raw_source_files, insert_edge_with_proximity, open_db, EnvVarGuard, TestEnv,
};

// T-4.3: RTM FR-23.6
#[test]
fn detect_and_persist_clusters_hubs_together_when_proximity_sum_is_strongest() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    let strong_edge = json!({ "memory://test/prox-h-heavy.md": 100.0 }).to_string();
    let satellite_edge = json!({ "memory://test/prox-h-sat.md": 1.0 }).to_string();

    insert_concept(&conn, "prox-h-a");
    insert_concept(&conn, "prox-h-b");
    insert_edge_raw_source_files(&conn, "prox-h-a", "prox-h-b", 1.0, &strong_edge);

    for name in ["prox-h-s1", "prox-h-s2", "prox-h-s3"] {
        insert_concept(&conn, name);
        insert_edge_raw_source_files(&conn, "prox-h-a", name, 1.0, &satellite_edge);
    }
    for name in ["prox-h-s4", "prox-h-s5", "prox-h-s6"] {
        insert_concept(&conn, name);
        insert_edge_raw_source_files(&conn, "prox-h-b", name, 1.0, &satellite_edge);
    }
    drop(conn);

    detect_and_persist(&env.cfg.database_path, 0.5, None).expect("detect communities");
    let conn = open_db(&env.cfg);
    assert_eq!(
        community_id(&conn, "prox-h-a"),
        community_id(&conn, "prox-h-b")
    );
}

// T-4.3: RTM FR-23.6
#[test]
fn detect_and_persist_treats_empty_source_files_as_isolated_edges() {
    let _guard = acquire_test_lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "prox-anchor-a");
    insert_concept(&conn, "prox-anchor-b");
    insert_edge_with_proximity(
        &conn,
        "prox-anchor-a",
        "prox-anchor-b",
        "memory://test/anchor.md",
        1.0,
    );

    insert_concept(&conn, "prox-isolated-x");
    insert_concept(&conn, "prox-isolated-y");
    insert_edge_raw_source_files(&conn, "prox-isolated-x", "prox-isolated-y", 5.0, "{}");
    drop(conn);

    detect_and_persist(&env.cfg.database_path, 1.0, None).expect("detect communities");
    let conn = open_db(&env.cfg);
    assert_ne!(
        community_id(&conn, "prox-isolated-x"),
        community_id(&conn, "prox-isolated-y"),
        "empty source_files must produce an isolated pair"
    );
}

// T-4.3: RTM FR-23.7
#[test]
fn compute_nmi_returns_one_for_identical_clusterings() {
    let _guard = acquire_test_lock();

    let clustering = BTreeMap::from([
        ("alpha".to_string(), 0_usize),
        ("beta".to_string(), 0_usize),
        ("gamma".to_string(), 1_usize),
        ("delta".to_string(), 1_usize),
    ]);

    assert_approx_eq(compute_nmi(&clustering, &clustering), 1.0);
}

// T-4.3: RTM FR-23.7
#[test]
fn compute_nmi_returns_value_below_one_for_different_clusterings() {
    let _guard = acquire_test_lock();

    let clustering_a = BTreeMap::from([
        ("alpha".to_string(), 0_usize),
        ("beta".to_string(), 0_usize),
        ("gamma".to_string(), 1_usize),
        ("delta".to_string(), 1_usize),
    ]);
    let clustering_b = BTreeMap::from([
        ("alpha".to_string(), 0_usize),
        ("beta".to_string(), 1_usize),
        ("gamma".to_string(), 0_usize),
        ("delta".to_string(), 1_usize),
    ]);

    let nmi = compute_nmi(&clustering_a, &clustering_b);
    assert!(
        nmi < 1.0,
        "different partitions must produce NMI < 1.0, got {nmi}"
    );
    assert!(
        (0.0..=1.0).contains(&nmi),
        "NMI must remain normalized, got {nmi}"
    );
}

// T-14.6: RTM FR-15
#[test]
#[cfg(unix)]
fn detect_and_persist_logs_warning_when_community_drift_exceeds_threshold() {
    let _guard = acquire_test_lock();
    let _threshold = EnvVarGuard::set("MAENIFOLD_COMMUNITY_NMI_THRESHOLD", "0.95");
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    for name in ["drift-a0", "drift-a1", "drift-a2"] {
        insert_concept(&conn, name);
    }
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-a1",
        "memory://test/drift-a.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a1",
        "drift-a2",
        "memory://test/drift-a.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-a2",
        "memory://test/drift-a.md",
        1.0,
    );

    for name in ["drift-b0", "drift-b1", "drift-b2"] {
        insert_concept(&conn, name);
    }
    insert_edge_with_proximity(
        &conn,
        "drift-b0",
        "drift-b1",
        "memory://test/drift-b.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-b1",
        "drift-b2",
        "memory://test/drift-b.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-b0",
        "drift-b2",
        "memory://test/drift-b.md",
        1.0,
    );
    drop(conn);

    detect_and_persist(&env.cfg.database_path, 1.0, None).expect("first detect");

    let conn = open_db(&env.cfg);
    conn.execute("DELETE FROM concept_graph", [])
        .expect("clear graph");
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-b0",
        "memory://test/drift-cross.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a1",
        "drift-b1",
        "memory://test/drift-cross.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a2",
        "drift-b2",
        "memory://test/drift-cross.md",
        1.0,
    );
    drop(conn);

    let (_, stderr_output) = support::capture_stderr(|| {
        detect_and_persist(&env.cfg.database_path, 1.0, None).expect("second detect");
    });

    assert!(
        stderr_output.contains("drift")
            || stderr_output.contains("DRIFT")
            || stderr_output.contains("NMI"),
        "expected community drift warning in stderr, got: {stderr_output}"
    );
}
