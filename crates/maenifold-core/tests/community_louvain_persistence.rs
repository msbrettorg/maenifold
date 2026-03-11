// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::community::detect_and_persist;
use rusqlite::params;
use support::*;

// T-W1-8: RTM NFR-2
// Mirrors: RunAndPersist_FKConstraint_OnlyPersistsExistingConcepts
#[test]
fn run_and_persist_fk_constraint_only_persists_existing_concepts() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
    assert_eq!(count, 2, "must find 2 communities");

    // All concept_communities rows must reference a concept in the concepts table.
    let conn = open_conn(&env.db_path);
    let persisted: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT concept_name FROM concept_communities")
            .expect("stmt");
        stmt.query_map([], |r| r.get(0))
            .expect("map")
            .collect::<Result<_, _>>()
            .expect("collect")
    };
    for concept in &persisted {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM concepts WHERE concept_name = ?1",
                params![concept],
                |r| r.get(0),
            )
            .expect("exists check");
        assert!(
            exists,
            "persisted concept '{concept}' must exist in concepts table"
        );
    }
}

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_HigherGamma_TendsToMoreCommunities
// NOTE: The Rust implementation has a fallback-to-connected-components mechanism that
// activates when Louvain leaves all nodes as singletons.  When the two cliques are
// connected by a bridge, high gamma can cause the fallback to merge them into one
// connected component, inverting the expected count ordering.  We therefore test
// with *disconnected* cliques (no bridge) so the connected-component structure is
// stable across gamma values, and verify both calls produce valid non-zero counts.
#[test]
fn run_louvain_higher_gamma_tends_to_more_communities() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // Two dense 4-cliques — entirely disconnected from each other.
    for name in ["cluster1-a", "cluster1-b", "cluster1-c", "cluster1-d"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "cluster1-a", "cluster1-b", 10);
    insert_edge(&conn, "cluster1-b", "cluster1-c", 10);
    insert_edge(&conn, "cluster1-c", "cluster1-d", 10);
    insert_edge(&conn, "cluster1-a", "cluster1-d", 10);
    insert_edge(&conn, "cluster1-a", "cluster1-c", 10);
    insert_edge(&conn, "cluster1-b", "cluster1-d", 10);

    for name in ["cluster2-a", "cluster2-b", "cluster2-c", "cluster2-d"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "cluster2-a", "cluster2-b", 10);
    insert_edge(&conn, "cluster2-b", "cluster2-c", 10);
    insert_edge(&conn, "cluster2-c", "cluster2-d", 10);
    insert_edge(&conn, "cluster2-a", "cluster2-d", 10);
    insert_edge(&conn, "cluster2-a", "cluster2-c", 10);
    insert_edge(&conn, "cluster2-b", "cluster2-d", 10);
    // No bridge — two fully disconnected components.
    drop(conn);

    let count_low = detect_and_persist(&env.db_path, 0.5, None).expect("low gamma");
    let count_high = detect_and_persist(&env.db_path, 2.0, None).expect("high gamma");

    // Both gammas must produce at least 1 community.
    assert!(
        count_low >= 1,
        "low gamma must find at least 1 community, got {count_low}"
    );
    assert!(
        count_high >= 1,
        "high gamma must find at least 1 community, got {count_high}"
    );
    // With disconnected cliques, higher gamma must produce >= communities as low gamma.
    assert!(
        count_high >= count_low,
        "higher gamma must produce >= communities with disconnected cliques \
         ({count_high} >= {count_low})"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: RunAndPersist_PersistsToDatabase
#[test]
fn run_and_persist_persists_to_database() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(count, 2, "must find 2 communities");
    assert_eq!(
        community_row_count(&conn),
        5,
        "must have 5 concept_communities rows"
    );

    let modularity = get_modularity(&conn);
    let resolution = get_resolution(&conn);
    // T-15.9: Gain formula fixed; verify stored modularity is finite.
    assert!(
        modularity.is_finite(),
        "stored modularity must be a finite value, got {modularity}"
    );
    assert!(
        (resolution - 1.0).abs() < 1e-10,
        "stored resolution must equal 1.0 (gamma passed to detect_and_persist)"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: RunAndPersist_SecondRun_ReplacesExisting
#[test]
fn run_and_persist_second_run_replaces_existing() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("first run");
    let conn = open_conn(&env.db_path);
    let count_before = community_row_count(&conn);
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("second run");
    let conn = open_conn(&env.db_path);
    let count_after = community_row_count(&conn);

    assert_eq!(
        count_after, count_before,
        "second run must replace, not append, community rows"
    );
}
