#[path = "graph_analyzer_support/mod.rs"]
mod graph_analyzer_support;

use graph_analyzer_support::*;
use maenifold_core::search;
use rusqlite::params;

// T-1.6: RTM NFR-1 — REWRITE per R-1, R-2: replace dead assertion with minimum-length
// check and add edge-validity assertion for consecutive walk steps.
#[test]
fn random_walk_with_valid_concept_returns_walk_sequence() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = open_connection(&cfg);
    let walk = search::random_walk(&connection, "concept-a", Some(5)).expect("random walk");

    // R-1: concept-a has 2 neighbors (concept-b, concept-c) so the walk must advance
    // at least once beyond the start node. Replaces the dead `!walk.is_empty()`.
    assert!(
        walk.len() >= 2,
        "walk from a connected concept should have at least 2 steps, got {walk:?}"
    );
    assert_eq!(
        walk[0], "concept-a",
        "walk should start at requested concept"
    );
    assert!(
        walk.len() <= 5,
        "walk length should not exceed request: {walk:?}"
    );

    // R-2: every consecutive pair must follow a real edge in concept_graph.
    for pair in walk.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let edge_exists: bool = connection
            .query_row(
                "SELECT COUNT(*) > 0 FROM concept_graph \
                 WHERE (concept_a = ?1 AND concept_b = ?2) \
                    OR (concept_a = ?2 AND concept_b = ?1)",
                params![a, b],
                |row| row.get(0),
            )
            .expect("edge check");
        assert!(
            edge_exists,
            "walk step {a} → {b} does not follow a real edge"
        );
    }
}

// T-1.6: RTM NFR-1 — KEEP: tests the specific early-exit branch for isolated concepts.
#[test]
fn random_walk_terminates_at_dead_end() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = open_connection(&cfg);
    let walk = search::random_walk(&connection, "isolated-concept", Some(10)).expect("random walk");

    assert_eq!(
        walk.len(),
        1,
        "isolated concept walk should stop immediately"
    );
    assert_eq!(walk[0], "isolated-concept");
}

// T-1.6: RTM NFR-1 — REWRITE per R-4 (Option B): increase sample count from 10 to 50
// to reduce false-failure rate from ~0.2% to astronomically low (~10⁻¹⁵).
#[test]
fn random_walk_produces_different_results() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = open_connection(&cfg);
    // R-4: 50 walks. P(all identical) = (1/2)^49 ≈ 10⁻¹⁵ — effectively zero.
    let unique_walks = (0..50)
        .map(|_| {
            search::random_walk(&connection, "concept-a", Some(5))
                .expect("random walk")
                .join(",")
        })
        .collect::<std::collections::BTreeSet<_>>();

    assert!(
        unique_walks.len() >= 3,
        "expected at least 3 distinct walks from 50 samples, got {}: {unique_walks:?}",
        unique_walks.len()
    );
}

// T-1.6: RTM NFR-1 — R-3: explicitly test that the walk may revisit nodes.
// The implementation has no visited-set guard; a walk on a two-node graph must oscillate.
#[test]
fn random_walk_may_revisit_nodes_on_two_node_graph() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    // Seed a minimal two-node graph: node-x ↔ node-y only.
    let connection = open_connection(&cfg);
    connection
        .execute(
            "INSERT OR IGNORE INTO concepts (concept_name, first_seen) \
             VALUES ('node-x', '2026-01-01')",
            [],
        )
        .expect("insert node-x");
    connection
        .execute(
            "INSERT OR IGNORE INTO concepts (concept_name, first_seen) \
             VALUES ('node-y', '2026-01-01')",
            [],
        )
        .expect("insert node-y");
    connection
        .execute(
            "INSERT OR REPLACE INTO concept_graph \
             (concept_a, concept_b, co_occurrence_count, source_files) \
             VALUES ('node-x', 'node-y', 1, '{\"test://f0\":1.0}')",
            [],
        )
        .expect("insert edge");

    let walk = search::random_walk(&connection, "node-x", Some(5)).expect("random walk");

    // On a 2-node graph, both nodes always have exactly 1 neighbor (each other).
    // The walk must reach the full requested length since there is never a dead end.
    assert_eq!(
        walk.len(),
        5,
        "two-node walk should reach full length via revisits: {walk:?}"
    );

    // Every element must be one of the two concepts.
    for (i, concept) in walk.iter().enumerate() {
        assert!(
            concept == "node-x" || concept == "node-y",
            "walk[{i}] = {concept}, expected node-x or node-y"
        );
    }

    // The walk must oscillate: consecutive steps must differ
    // (each node's only neighbor is the other).
    for pair in walk.windows(2) {
        assert_ne!(
            pair[0], pair[1],
            "consecutive steps should differ on a two-node graph: {walk:?}"
        );
    }
}

// T-1.6: RTM NFR-1 — R-5: test the default walk_length path (None → 10).
#[test]
fn random_walk_uses_default_length_when_none_given() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = open_connection(&cfg);
    let walk = search::random_walk(&connection, "concept-a", None).expect("random walk");

    // Default walk_length is 10. All nodes in the connected component containing
    // concept-a have at least one neighbor (due to revisits), so the walk reaches full length.
    assert_eq!(
        walk.len(),
        10,
        "walk with None length should default to 10: {walk:?}"
    );
    assert_eq!(
        walk[0], "concept-a",
        "walk should start at requested concept"
    );
}

// T-1.6: RTM NFR-1 — R-6: test behavior with a non-existent start concept.
// The implementation pushes start unconditionally, then breaks on empty neighbors.
#[test]
fn random_walk_with_nonexistent_concept_returns_single_element() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = open_connection(&cfg);
    let walk = search::random_walk(&connection, "does-not-exist", Some(10)).expect("random walk");

    assert_eq!(
        walk.len(),
        1,
        "non-existent concept should produce a single-element walk"
    );
    assert_eq!(walk[0], "does-not-exist");
}
