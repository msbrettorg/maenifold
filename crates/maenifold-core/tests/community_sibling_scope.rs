// T-15.7: RTM FR-13.7 — community sibling candidates must be restricted to same community.
// Validates that the all-concepts fallback has been removed and only same-community concepts
// are returned as sibling candidates.

#[path = "community/support.rs"]
mod support;

use maenifold_core::ToolRegistry;
use maenifold_core::community::detect_and_persist;
use maenifold_core::json::OutputFormat;
use rusqlite::params;
use serde_json::json;
use support::*;

// T-15.7: RTM FR-13.7
// Test 1: Concept with community_id=X → only returns concepts also in community X.
#[test]
fn sibling_candidates_restricted_to_same_community() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // Build two tight clusters in separate communities.
    // Cluster A: query-a, peer-a, shared neighbors na1..na4
    // Cluster B: alien-b, shared neighbors nb1..nb4
    for name in [
        "query-a", "peer-a", "na1", "na2", "na3", "na4", "alien-b", "nb1", "nb2", "nb3", "nb4",
    ] {
        insert_concept(&conn, name);
    }

    // Cluster A — tight subgraph: query-a and peer-a share 4 strong neighbors, no direct edge.
    insert_edge(&conn, "query-a", "na1", 10);
    insert_edge(&conn, "query-a", "na2", 10);
    insert_edge(&conn, "query-a", "na3", 10);
    insert_edge(&conn, "query-a", "na4", 10);
    insert_edge(&conn, "peer-a", "na1", 10);
    insert_edge(&conn, "peer-a", "na2", 10);
    insert_edge(&conn, "peer-a", "na3", 10);
    insert_edge(&conn, "peer-a", "na4", 10);
    // Internal cluster A edges for Louvain community detection.
    insert_edge(&conn, "na1", "na2", 10);
    insert_edge(&conn, "na1", "na3", 10);
    insert_edge(&conn, "na1", "na4", 10);
    insert_edge(&conn, "na2", "na3", 10);
    insert_edge(&conn, "na2", "na4", 10);
    insert_edge(&conn, "na3", "na4", 10);

    // Cluster B — tight subgraph: alien-b connected to nb1..nb4.
    insert_edge(&conn, "alien-b", "nb1", 10);
    insert_edge(&conn, "alien-b", "nb2", 10);
    insert_edge(&conn, "alien-b", "nb3", 10);
    insert_edge(&conn, "alien-b", "nb4", 10);
    insert_edge(&conn, "nb1", "nb2", 10);
    insert_edge(&conn, "nb1", "nb3", 10);
    insert_edge(&conn, "nb1", "nb4", 10);
    insert_edge(&conn, "nb2", "nb3", 10);
    insert_edge(&conn, "nb2", "nb4", 10);
    insert_edge(&conn, "nb3", "nb4", 10);

    drop(conn);

    // Run community detection — two distinct clusters should get different community IDs.
    let comm_count = detect_and_persist(&env.db_path, 1.0, None).expect("detect communities");
    assert!(
        comm_count >= 2,
        "expected ≥ 2 communities, got {comm_count}"
    );

    let conn = open_conn(&env.db_path);
    let query_comm = get_community_id(&conn, "query-a").expect("query-a community");
    let alien_comm = get_community_id(&conn, "alien-b").expect("alien-b community");
    assert_ne!(
        query_comm, alien_comm,
        "query-a and alien-b must be in different communities"
    );

    let peer_comm = get_community_id(&conn, "peer-a").expect("peer-a community");
    assert_eq!(
        query_comm, peer_comm,
        "query-a and peer-a must be in the same community"
    );
    drop(conn);

    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "query-a", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("build context");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let siblings = parsed["data"]["CommunitySiblings"]
        .as_array()
        .expect("CommunitySiblings array");
    let sibling_names: Vec<&str> = siblings.iter().filter_map(|s| s["Name"].as_str()).collect();

    // peer-a is in the same community and shares 4 neighbors → must appear.
    assert!(
        sibling_names.contains(&"peer-a"),
        "T-15.7: peer-a (same community, shared neighbors) must appear in CommunitySiblings, got: {sibling_names:?}"
    );

    // alien-b is in a different community → must NOT appear.
    assert!(
        !sibling_names.contains(&"alien-b"),
        "T-15.7: alien-b (different community) must NOT appear in CommunitySiblings, got: {sibling_names:?}"
    );
}

// T-15.7: RTM FR-13.7
// Test 2: Concept with no community_id → returns empty list (no fallback).
#[test]
fn no_community_returns_empty_siblings() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // Create concepts with edges but NO community assignments.
    for name in ["orphan-q", "orphan-t", "on1", "on2", "on3", "on4"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "orphan-q", "on1", 10);
    insert_edge(&conn, "orphan-q", "on2", 10);
    insert_edge(&conn, "orphan-q", "on3", 10);
    insert_edge(&conn, "orphan-q", "on4", 10);
    insert_edge(&conn, "orphan-t", "on1", 10);
    insert_edge(&conn, "orphan-t", "on2", 10);
    insert_edge(&conn, "orphan-t", "on3", 10);
    insert_edge(&conn, "orphan-t", "on4", 10);
    // Deliberately do NOT run detect_and_persist → no community assignments.
    drop(conn);

    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "orphan-q", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("build context");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let siblings = parsed["data"]["CommunitySiblings"]
        .as_array()
        .expect("CommunitySiblings array");

    assert!(
        siblings.is_empty(),
        "T-15.7: concept with no community_id must return empty CommunitySiblings, got: {siblings:?}"
    );
}

// T-15.7: RTM FR-13.7
// Test 3: Concepts in different communities → not returned as siblings.
// Uses manual community assignments to ensure precise community placement.
#[test]
fn different_community_concepts_not_siblings() {
    let _g = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let conn = open_conn(&env.db_path);

    // Create two concepts that share 4 strong neighbors (would qualify as siblings
    // by overlap metrics) but place them in different communities manually.
    for name in ["cross-query", "cross-alien", "cn1", "cn2", "cn3", "cn4"] {
        insert_concept(&conn, name);
    }
    insert_edge(&conn, "cross-query", "cn1", 10);
    insert_edge(&conn, "cross-query", "cn2", 10);
    insert_edge(&conn, "cross-query", "cn3", 10);
    insert_edge(&conn, "cross-query", "cn4", 10);
    insert_edge(&conn, "cross-alien", "cn1", 10);
    insert_edge(&conn, "cross-alien", "cn2", 10);
    insert_edge(&conn, "cross-alien", "cn3", 10);
    insert_edge(&conn, "cross-alien", "cn4", 10);

    // Manually assign communities: cross-query in community 100, cross-alien in community 200.
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO concept_communities \
         (concept_name, community_id, modularity, resolution, detected_at) \
         VALUES (?1, 100, 0.5, 1.0, ?2)",
        params!["cross-query", now],
    )
    .expect("insert community for cross-query");
    conn.execute(
        "INSERT OR REPLACE INTO concept_communities \
         (concept_name, community_id, modularity, resolution, detected_at) \
         VALUES (?1, 200, 0.5, 1.0, ?2)",
        params!["cross-alien", now],
    )
    .expect("insert community for cross-alien");
    // Also assign neighbors to query's community so the query concept has a valid community.
    for n in ["cn1", "cn2", "cn3", "cn4"] {
        conn.execute(
            "INSERT OR REPLACE INTO concept_communities \
             (concept_name, community_id, modularity, resolution, detected_at) \
             VALUES (?1, 100, 0.5, 1.0, ?2)",
            params![n, now],
        )
        .expect("insert community for neighbor");
    }
    drop(conn);

    let output = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "cross-query", "depth": 1 }),
            OutputFormat::Json,
        )
        .expect("build context");

    let parsed: serde_json::Value = serde_json::from_str(&output).expect("parse json");
    let siblings = parsed["data"]["CommunitySiblings"]
        .as_array()
        .expect("CommunitySiblings array");
    let sibling_names: Vec<&str> = siblings.iter().filter_map(|s| s["Name"].as_str()).collect();

    // cross-alien is in community 200, cross-query is in community 100 → must NOT appear.
    assert!(
        !sibling_names.contains(&"cross-alien"),
        "T-15.7: cross-alien (community 200) must NOT appear as sibling of cross-query (community 100), got: {sibling_names:?}"
    );
}
