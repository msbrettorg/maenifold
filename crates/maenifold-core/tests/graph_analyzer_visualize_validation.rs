#[path = "graph_analyzer_support/mod.rs"]
mod graph_analyzer_support;

use graph_analyzer_support::*;
use regex::Regex;
use serde_json::json;

// T-1.6: RTM NFR-1 — validation: empty and null conceptName are identical code paths
// (Value::Null.as_str() → None → unwrap_or_default() → ""; same as empty string).
// Merged from separate empty-string and null tests per eval recommendation.
#[test]
fn visualize_with_empty_or_null_concept_name_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    // No seed_graph — validation rejects before any DB access.

    for (label, concept_value) in [("empty string", json!("")), ("null", json!(null))] {
        let result = visualize(
            &cfg,
            json!({
                "conceptName": concept_value,
                "depth": 1,
                "maxNodes": 10
            }),
        );

        assert!(
            result.starts_with("ERROR"),
            "{label}: expected error output: {result}"
        );
        assert!(
            result.contains("required"),
            "{label}: missing required detail: {result}"
        );
    }
}

// T-1.6: RTM NFR-1 — non-existent concept returns guidance to run sync.
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
    // T-1.6: RTM NFR-2 — pinned to exact phrase; previous `.to_ascii_lowercase().contains("sync")`
    // was incidental and would match any output containing "sync" in any context.
    assert!(
        result.contains("Run sync first"),
        "missing sync guidance: {result}"
    );
}

// T-1.6: RTM NFR-1 — depth boundary validation. Collects all failures so every
// boundary value is verified even if an earlier one fails.
#[test]
fn visualize_with_invalid_depth_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let mut failures = Vec::new();
    for invalid_depth in [0, -1, 6, 10] {
        let result = visualize(
            &cfg,
            json!({
                "conceptName": "concept-a",
                "depth": invalid_depth,
                "maxNodes": 10
            }),
        );

        if !result.starts_with("ERROR") {
            failures.push(format!("depth {invalid_depth} should fail: {result}"));
        } else if !(result.contains("depth") && result.contains("between")) {
            failures.push(format!(
                "depth range detail missing for {invalid_depth}: {result}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "depth validation failures:\n{}",
        failures.join("\n")
    );
}

// T-1.6: RTM NFR-1 — maxNodes boundary validation. Collects all failures so every
// boundary value is verified even if an earlier one fails.
#[test]
fn visualize_with_invalid_max_nodes_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let mut failures = Vec::new();
    for invalid_max_nodes in [0, 4, 101, 1000] {
        let result = visualize(
            &cfg,
            json!({
                "conceptName": "concept-a",
                "depth": 1,
                "maxNodes": invalid_max_nodes
            }),
        );

        if !result.starts_with("ERROR") {
            failures.push(format!(
                "maxNodes {invalid_max_nodes} should fail: {result}"
            ));
        } else if !(result.contains("maxNodes") && result.contains("between")) {
            failures.push(format!(
                "maxNodes range detail missing for {invalid_max_nodes}: {result}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "maxNodes validation failures:\n{}",
        failures.join("\n")
    );
}

// T-1.6: RTM NFR-1 — isolated concept with zero edges produces valid Mermaid with
// "No connections found" indicator. Both node ID and message are asserted independently.
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
    // T-1.6: RTM NFR-2 — assert both conditions independently; previous OR allowed
    // silent regressions where one branch could disappear undetected.
    assert!(
        result.contains("isolated_concept"),
        "expected node ID in output: {result}"
    );
    assert!(
        result.contains("No connections found"),
        "expected isolation indicator: {result}"
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
