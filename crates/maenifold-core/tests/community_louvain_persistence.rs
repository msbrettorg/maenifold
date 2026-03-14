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
// Rewrite of: RunLouvain_HigherGamma_TendsToMoreCommunities
// At extreme gamma, Louvain leaves all nodes as singletons (no modularity gain).
// The fallback then assigns communities by connected components.
// For two disconnected 4-cliques, this must produce exactly 2 communities.
#[test]
fn high_gamma_fallback_produces_connected_components() {
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

    // At extreme gamma (100.0), Louvain produces all singletons.
    // Fallback detects two connected components → exactly 2 communities.
    let count =
        detect_and_persist(&env.db_path, 100.0, Some(42)).expect("extreme gamma");
    assert_eq!(
        count, 2,
        "at extreme gamma, fallback must produce one community per connected component"
    );

    // Verify each clique's nodes co-cluster after fallback.
    let conn = open_conn(&env.db_path);
    let id_1a = get_community_id(&conn, "cluster1-a").expect("cluster1-a");
    let id_1b = get_community_id(&conn, "cluster1-b").expect("cluster1-b");
    let id_1c = get_community_id(&conn, "cluster1-c").expect("cluster1-c");
    let id_1d = get_community_id(&conn, "cluster1-d").expect("cluster1-d");
    let id_2a = get_community_id(&conn, "cluster2-a").expect("cluster2-a");
    let id_2b = get_community_id(&conn, "cluster2-b").expect("cluster2-b");
    let id_2c = get_community_id(&conn, "cluster2-c").expect("cluster2-c");
    let id_2d = get_community_id(&conn, "cluster2-d").expect("cluster2-d");

    assert_eq!(id_1a, id_1b, "clique-1 members must co-cluster after fallback");
    assert_eq!(id_1b, id_1c, "clique-1 members must co-cluster after fallback");
    assert_eq!(id_1c, id_1d, "clique-1 members must co-cluster after fallback");
    assert_eq!(id_2a, id_2b, "clique-2 members must co-cluster after fallback");
    assert_eq!(id_2b, id_2c, "clique-2 members must co-cluster after fallback");
    assert_eq!(id_2c, id_2d, "clique-2 members must co-cluster after fallback");
    assert_ne!(
        id_1a, id_2a,
        "different connected components must get distinct communities"
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

    // R-2: Verify structural membership — which nodes land in which community.
    let id_alpha = get_community_id(&conn, "alpha").expect("alpha");
    let id_beta = get_community_id(&conn, "beta").expect("beta");
    let id_gamma_node = get_community_id(&conn, "gamma").expect("gamma");
    let id_delta = get_community_id(&conn, "delta").expect("delta");
    let id_eps = get_community_id(&conn, "epsilon").expect("epsilon");
    assert_eq!(id_alpha, id_beta, "triangle members must co-cluster");
    assert_eq!(
        id_beta, id_gamma_node,
        "triangle members must co-cluster"
    );
    assert_ne!(
        id_alpha, id_delta,
        "triangle and pair must be distinct communities"
    );
    assert_eq!(id_delta, id_eps, "pair members must co-cluster");
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
    // R-3: Guard against silent first-run persistence failure (0 == 0 false green).
    assert!(
        count_before > 0,
        "first run must have written rows before idempotency check"
    );
    drop(conn);

    detect_and_persist(&env.db_path, 1.0, None).expect("second run");
    let conn = open_conn(&env.db_path);
    let count_after = community_row_count(&conn);

    assert_eq!(
        count_after, count_before,
        "second run must replace, not append, community rows"
    );
}
