// T-12.7: RTM NFR-1
#[path = "community/support.rs"]
mod support;

use maenifold_core::community::detect_and_persist;
use support::*;

// ─────────────────────────────────────────────────────────────────────────────
// §1  CommunityDetection (Louvain) unit tests — mirrors CommunityDetectionTests.cs
// ─────────────────────────────────────────────────────────────────────────────

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_DeterministicSeed_ProducesSameResult
// T-15.10: Rust now shuffles node visit order each iteration (matching C#).
// With seed=None, shuffles use system entropy but well-separated topologies
// converge to the same partition regardless of visit order. This verifies
// re-run stability (idempotency on a populated DB), not algorithmic
// determinism — the true determinism contract (seed=Some(42)) is tested
// by the inline unit tests in louvain.rs.
#[test]
fn run_louvain_rerun_on_populated_db_produces_stable_partition() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    let count1 = detect_and_persist(&env.db_path, 1.0, None).expect("first run");
    let conn = open_conn(&env.db_path);
    let mod1 = get_modularity(&conn);
    let id_alpha1 = get_community_id(&conn, "alpha");
    let id_delta1 = get_community_id(&conn, "delta");
    drop(conn);

    // Run again — should be identical.
    let count2 = detect_and_persist(&env.db_path, 1.0, None).expect("second run");
    let conn = open_conn(&env.db_path);
    let mod2 = get_modularity(&conn);
    let id_alpha2 = get_community_id(&conn, "alpha");
    let id_delta2 = get_community_id(&conn, "delta");

    assert_eq!(count1, count2, "community count must be stable across runs");
    assert!(
        (mod1 - mod2).abs() < 1e-10,
        "modularity must be identical across runs: {mod1} vs {mod2}"
    );
    assert_eq!(
        id_alpha1, id_alpha2,
        "alpha's community must be stable across runs"
    );
    assert_eq!(
        id_delta1, id_delta2,
        "delta's community must be stable across runs"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_TrianglePlusIsolatedPair_TwoCommunities
#[test]
fn run_louvain_triangle_plus_isolated_pair_two_communities() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_triangle_plus_isolated_pair(&conn);
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(
        count, 2,
        "triangle + isolated pair must produce exactly 2 communities"
    );

    let alpha = get_community_id(&conn, "alpha").expect("alpha");
    let beta = get_community_id(&conn, "beta").expect("beta");
    let gamma = get_community_id(&conn, "gamma").expect("gamma");
    let delta = get_community_id(&conn, "delta").expect("delta");
    let epsilon = get_community_id(&conn, "epsilon").expect("epsilon");

    assert_eq!(alpha, beta, "triangle: alpha-beta must share community");
    assert_eq!(beta, gamma, "triangle: beta-gamma must share community");
    assert_eq!(
        delta, epsilon,
        "isolated pair: delta-epsilon must share community"
    );
    assert_ne!(
        alpha, delta,
        "triangle and isolated pair must be in different communities"
    );

    // T-15.9: Gain formula now uses kiIn/m matching C#. Modularity must be finite and
    // within the theoretical range [-0.5, 1.0) for any valid partition. This is
    // meaningfully stronger than the previous `.is_finite()` check which would pass
    // for NaN-coerced or runaway values.
    let modularity = get_modularity(&conn);
    assert!(
        modularity.is_finite(),
        "modularity must be finite for a valid partition, got {modularity}"
    );
    assert!(
        (-0.5..1.0).contains(&modularity),
        "modularity must be in theoretical range [-0.5, 1.0), got {modularity}"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_EmptyGraph_ReturnsEmpty
// T-12.7: Merged precondition guard from community_coverage_basics (R-2).
#[test]
fn run_louvain_empty_graph_returns_empty() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();

    // Precondition: TestEnv must start with an empty concepts table.
    let conn = open_conn(&env.db_path);
    let concept_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM concepts", [], |r| r.get(0))
        .expect("count");
    assert_eq!(
        concept_count, 0,
        "TestEnv must start with empty concepts table"
    );
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(count, 0, "empty graph must return 0 communities");
    assert_eq!(
        community_row_count(&conn),
        0,
        "no rows must be written for empty graph"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_SingleNode_OneCommunity
#[test]
fn run_louvain_single_node_one_community() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);
    insert_concept(&conn, "alone");
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");

    let conn = open_conn(&env.db_path);
    assert_eq!(count, 1, "single node must produce 1 community");
    assert_eq!(
        get_community_id(&conn, "alone"),
        Some(0),
        // Note: Some(0) depends on normalize_communities' first-occurrence ordering
        // via BTreeMap. If normalization strategy changes, this assertion may need updating.
        "single node community must be 0"
    );
    assert_eq!(
        get_modularity(&conn),
        0.0,
        "single node must have 0 modularity"
    );
}

// T-W1-8: RTM NFR-2
// Mirrors: RunLouvain_DisconnectedComponents_SeparateCommunities
#[test]
fn run_louvain_disconnected_components_separate_communities() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // Component 1: 3-clique
    for name in ["c1-a", "c1-b", "c1-c"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "c1-a", "c1-b", 5);
    insert_edge(&conn, "c1-b", "c1-c", 5);
    insert_edge(&conn, "c1-a", "c1-c", 5);

    // Component 2: 3-clique
    for name in ["c2-a", "c2-b", "c2-c"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "c2-a", "c2-b", 5);
    insert_edge(&conn, "c2-b", "c2-c", 5);
    insert_edge(&conn, "c2-a", "c2-c", 5);
    drop(conn);

    let count = detect_and_persist(&env.db_path, 1.0, None).expect("detect");
    assert_eq!(
        count, 2,
        "two disconnected 3-cliques must produce exactly 2 communities"
    );

    let conn = open_conn(&env.db_path);
    let c1a = get_community_id(&conn, "c1-a").expect("c1-a");
    let c1b = get_community_id(&conn, "c1-b").expect("c1-b");
    let c1c = get_community_id(&conn, "c1-c").expect("c1-c");
    let c2a = get_community_id(&conn, "c2-a").expect("c2-a");
    let c2b = get_community_id(&conn, "c2-b").expect("c2-b");
    let c2c = get_community_id(&conn, "c2-c").expect("c2-c");

    assert_eq!(c1a, c1b, "component 1: c1-a and c1-b must share community");
    assert_eq!(c1b, c1c, "component 1: c1-b and c1-c must share community");
    assert_eq!(c2a, c2b, "component 2: c2-a and c2-b must share community");
    assert_eq!(c2b, c2c, "component 2: c2-b and c2-c must share community");
    assert_ne!(
        c1a, c2a,
        "disconnected components must be in different communities"
    );
    assert_ne!(
        c1c, c2c,
        "last node of each component must also be in different communities"
    );

    // T-15.9: Gain formula fixed; modularity must be finite and within theoretical range.
    let modularity = get_modularity(&conn);
    assert!(
        modularity.is_finite(),
        "modularity must be finite for disconnected components, got {modularity}"
    );
    assert!(
        (-0.5..1.0).contains(&modularity),
        "modularity must be in theoretical range [-0.5, 1.0), got {modularity}"
    );
}
