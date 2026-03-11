#[path = "graph_analyzer_support/mod.rs"]
mod graph_analyzer_support;

use graph_analyzer_support::*;
use regex::Regex;
use serde_json::json;

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep validation coverage isolated.
#[test]
fn visualize_with_empty_concept_name_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.starts_with("ERROR"),
        "expected error output: {result}"
    );
    assert!(
        result.contains("required"),
        "missing required detail: {result}"
    );
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep validation coverage isolated.
#[test]
fn visualize_with_null_concept_name_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": serde_json::Value::Null,
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.starts_with("ERROR"),
        "expected error output: {result}"
    );
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep validation coverage isolated.
#[test]
fn visualize_with_non_existent_concept_returns_not_found_message() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "non-existent-concept-xyz",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.contains("not found"),
        "missing not-found detail: {result}"
    );
    assert!(
        result.to_ascii_lowercase().contains("sync"),
        "missing sync guidance: {result}"
    );
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep validation coverage isolated.
#[test]
fn visualize_with_invalid_depth_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    for invalid_depth in [0, -1, 6, 10] {
        let result = visualize(
            &cfg,
            json!({
                "conceptName": "concept-a",
                "depth": invalid_depth,
                "maxNodes": 10
            }),
        );

        assert!(
            result.starts_with("ERROR"),
            "depth {invalid_depth} should fail: {result}"
        );
        assert!(
            result.contains("depth") && result.contains("between"),
            "depth range detail missing for {invalid_depth}: {result}"
        );
    }
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep validation coverage isolated.
#[test]
fn visualize_with_invalid_max_nodes_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    for invalid_max_nodes in [0, 4, 101, 1000] {
        let result = visualize(
            &cfg,
            json!({
                "conceptName": "concept-a",
                "depth": 1,
                "maxNodes": invalid_max_nodes
            }),
        );

        assert!(
            result.starts_with("ERROR"),
            "maxNodes {invalid_max_nodes} should fail: {result}"
        );
        assert!(
            result.contains("maxNodes") && result.contains("between"),
            "maxNodes range detail missing for {invalid_max_nodes}: {result}"
        );
    }
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep validation coverage isolated.
#[test]
fn visualize_with_isolated_concept_returns_empty_graph() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let result = visualize(
        &cfg,
        json!({
            "conceptName": "isolated-concept",
            "depth": 1,
            "maxNodes": 10
        }),
    );

    assert!(
        result.starts_with("graph TD"),
        "expected Mermaid output: {result}"
    );
    assert!(
        result.contains("No connections found") || result.contains("isolated_concept"),
        "expected isolated-node indicator: {result}"
    );
}

// T-1.6: RTM NFR-1 — split from graph_analyzer_tests.rs to keep validation coverage isolated.
#[test]
fn visualize_generates_valid_mermaid_node_ids() {
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
        result.starts_with("graph TD"),
        "unexpected output: {result}"
    );

    let edge_regex = Regex::new(r"\w+ -->\|\d+\| \w+").expect("edge regex");
    assert!(
        edge_regex.is_match(&result),
        "expected Mermaid edge line with node identifiers: {result}"
    );

    for line in result
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
    {
        assert!(
            line.contains("-->") || line.contains('['),
            "expected Mermaid syntax on line: {line}"
        );
    }
}
