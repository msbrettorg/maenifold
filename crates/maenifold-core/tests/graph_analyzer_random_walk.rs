#[path = "graph_analyzer_support/mod.rs"]
mod graph_analyzer_support;

use graph_analyzer_support::*;
use maenifold_core::search;

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep random-walk coverage isolated.
#[test]
fn random_walk_with_valid_concept_returns_walk_sequence() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = open_connection(&cfg);
    let walk = search::random_walk(&connection, "concept-a", Some(5)).expect("random walk");

    assert!(!walk.is_empty(), "walk should contain at least one concept");
    assert_eq!(
        walk[0], "concept-a",
        "walk should start at requested concept"
    );
    assert!(
        walk.len() <= 5,
        "walk length should not exceed request: {walk:?}"
    );
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep random-walk coverage isolated.
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

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep random-walk coverage isolated.
#[test]
fn random_walk_produces_different_results() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let connection = open_connection(&cfg);
    let unique_walks = (0..10)
        .map(|_| {
            search::random_walk(&connection, "concept-a", Some(5))
                .expect("random walk")
                .join(",")
        })
        .collect::<std::collections::BTreeSet<_>>();

    assert!(
        unique_walks.len() > 1,
        "expected multiple random walks to diverge, got {unique_walks:?}"
    );
}
