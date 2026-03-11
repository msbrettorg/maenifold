// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::community::detect_and_persist;
use support::*;

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_TwoCliquesWeakBridge_ImprovedFlagSet
#[test]
fn run_louvain_two_cliques_weak_bridge_improved_flag_set() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    for name in ["clq1-x", "clq1-y", "clq1-z"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "clq1-x", "clq1-y", 10);
    insert_edge(&conn, "clq1-y", "clq1-z", 10);
    insert_edge(&conn, "clq1-x", "clq1-z", 10);

    for name in ["clq2-x", "clq2-y", "clq2-z"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "clq2-x", "clq2-y", 10);
    insert_edge(&conn, "clq2-y", "clq2-z", 10);
    insert_edge(&conn, "clq2-x", "clq2-z", 10);

    insert_edge(&conn, "clq1-x", "clq2-x", 1); // weak bridge
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(
        community_row_count(&conn),
        6,
        "all 6 nodes must be persisted"
    );

    let clq1x = get_community_id(&conn, "clq1-x").expect("clq1-x");
    let clq1y = get_community_id(&conn, "clq1-y").expect("clq1-y");
    let clq1z = get_community_id(&conn, "clq1-z").expect("clq1-z");
    let clq2x = get_community_id(&conn, "clq2-x").expect("clq2-x");
    let clq2y = get_community_id(&conn, "clq2-y").expect("clq2-y");
    let clq2z = get_community_id(&conn, "clq2-z").expect("clq2-z");

    assert_eq!(clq1x, clq1y, "clique 1 nodes must share community");
    assert_eq!(clq1y, clq1z, "clique 1 nodes must share community");
    assert_eq!(clq2x, clq2y, "clique 2 nodes must share community");
    assert_eq!(clq2y, clq2z, "clique 2 nodes must share community");
    assert_ne!(
        clq1x, clq2x,
        "two dense cliques must be in separate communities"
    );

    let modularity = get_modularity(&conn);
    // T-15.9: Gain formula fixed; verify the partition produces finite modularity.
    assert!(
        modularity.is_finite(),
        "two-clique graph must produce finite modularity, got {modularity}"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_IsolatedConcepts_NormalizedContiguousIds
#[test]
fn run_louvain_isolated_concepts_normalized_contiguous_ids() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    for name in ["norm-a", "norm-b", "norm-c", "norm-d"] {
        insert_concept(&conn, name);
    }
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    let mut ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT community_id FROM concept_communities ORDER BY community_id")
            .expect("stmt");
        stmt.query_map([], |r| r.get(0))
            .expect("map")
            .collect::<Result<_, _>>()
            .expect("collect")
    };
    ids.sort_unstable();
    assert_eq!(
        ids,
        vec![0, 1, 2, 3],
        "isolated concepts must receive contiguous normalized IDs 0..N-1, got {:?}",
        ids
    );
}
