// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::community::detect_and_persist;
use support::*;

// ─────────────────────────────────────────────────────────────────────────────
// §2  CommunityDetectionCoverage tests — mirrors CommunityDetectionCoverageTests.cs
// ─────────────────────────────────────────────────────────────────────────────

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_MultipleIsolatedConcepts_NoEdges_EachGetOwnCommunity
#[test]
fn run_louvain_multiple_isolated_concepts_no_edges_each_get_own_community() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    for name in ["iso-a", "iso-b", "iso-c"] {
        insert_concept(&conn, name);
    }
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(
        count, 3,
        "3 isolated concepts must each receive own community"
    );
    assert_eq!(community_row_count(&conn), 3, "all 3 must be persisted");
    // All community IDs must be in 0..2.
    let max_id: i64 = conn
        .query_row(
            "SELECT MAX(community_id) FROM concept_communities",
            [],
            |r| r.get(0),
        )
        .expect("max id");
    assert!(
        max_id < 3,
        "community IDs must be in range 0..N-1, got max {max_id}"
    );
    assert_eq!(get_modularity(&conn), 0.0, "no edges → 0 modularity");
}

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_FiveIsolatedConcepts_NoEdges_FiveDistinctCommunities
#[test]
fn run_louvain_five_isolated_concepts_no_edges_five_distinct_communities() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    for name in ["node-1", "node-2", "node-3", "node-4", "node-5"] {
        insert_concept(&conn, name);
    }
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(
        count, 5,
        "5 isolated concepts must each have distinct community"
    );
    assert_eq!(community_row_count(&conn), 5);
    assert_eq!(get_modularity(&conn), 0.0, "no edges → 0 modularity");
}

// T-W1-8: RTM NFR-2
// Mirrors: RunAndPersist_EmptyGraph_ReturnsZeroWithoutDbWrites
#[test]
fn run_and_persist_empty_graph_returns_zero_without_db_writes() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();

    // Verify precondition: no concepts.
    let conn = open_conn(&env.db_path);
    let concept_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM concepts", [], |r| r.get(0))
        .expect("count");
    assert_eq!(concept_count, 0, "concepts table must be empty");
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(count, 0, "empty graph must return 0 communities");
    assert_eq!(
        community_row_count(&conn),
        0,
        "empty graph must write no concept_communities rows"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: RunAndPersist_IsolatedConcepts_PersistsEachWithOwnCommunity
#[test]
fn run_and_persist_isolated_concepts_persists_each_with_own_community() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_concept(&conn, "persist-iso-a");
    insert_concept(&conn, "persist-iso-b");
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(
        count, 2,
        "2 isolated concepts must each be in own community"
    );
    assert_eq!(get_modularity(&conn), 0.0, "no edges → 0 modularity");
    assert_eq!(
        community_row_count(&conn),
        2,
        "both isolated concepts must be persisted"
    );

    let id_a = get_community_id(&conn, "persist-iso-a").expect("persist-iso-a");
    let id_b = get_community_id(&conn, "persist-iso-b").expect("persist-iso-b");
    assert_ne!(
        id_a, id_b,
        "isolated concepts must have distinct community IDs"
    );
}
