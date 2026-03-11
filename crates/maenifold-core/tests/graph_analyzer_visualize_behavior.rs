#[path = "graph_analyzer_support/mod.rs"]
mod graph_analyzer_support;

use graph_analyzer_support::*;
use serde_json::json;

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep focused behavior coverage.
#[test]
fn visualize_with_valid_concept_returns_mermaid_diagram() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(!result.is_empty(), "result should not be empty");
    assert!(
        result.starts_with("graph TD"),
        "unexpected output: {result}"
    );
    assert!(
        result.contains("-->"),
        "expected Mermaid edge syntax: {result}"
    );
    assert!(
        result.contains("concept_b") || result.contains("concept_c"),
        "expected connected concepts in diagram: {result}"
    );
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep focused behavior coverage.
#[test]
fn visualize_includes_co_occurrence_counts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.contains("-->|2|"),
        "expected edge labels in output: {result}"
    );
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep focused behavior coverage.
#[test]
fn visualize_respects_depth_parameter() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let depth_one = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 50
        }),
    );
    let depth_two = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 2,
            "maxNodes": 50
        }),
    );

    let depth_one_lines = depth_one
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let depth_two_lines = depth_two
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();

    assert!(
        depth_two_lines >= depth_one_lines,
        "depth 2 should include at least as many lines as depth 1:\n{depth_one}\n---\n{depth_two}"
    );
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep focused behavior coverage.
#[test]
fn visualize_normalizes_concept_names() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let lower = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 10
        }),
    );
    let upper = visualize(
        &cfg,
        json!({
            "conceptName": "CONCEPT-A",
            "depth": 1,
            "maxNodes": 10
        }),
    );
    let mixed = visualize(
        &cfg,
        json!({
            "conceptName": "Concept-A",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        lower.starts_with("graph TD"),
        "lowercase should work: {lower}"
    );
    assert!(
        upper.starts_with("graph TD"),
        "uppercase should normalize: {upper}"
    );
    assert!(
        mixed.starts_with("graph TD"),
        "mixed case should normalize: {mixed}"
    );
}
