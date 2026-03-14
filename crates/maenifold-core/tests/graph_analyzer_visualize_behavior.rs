#[path = "graph_analyzer_support/mod.rs"]
mod graph_analyzer_support;

use graph_analyzer_support::*;
use serde_json::json;

// T-1.6: RTM NFR-1 — happy-path: valid concept produces correct Mermaid diagram with
// both depth-1 neighbors and seeded co-occurrence weights.
// Merged: former `visualize_includes_co_occurrence_counts` folded in.
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

    assert!(
        result.starts_with("graph TD"),
        "unexpected output: {result}"
    );
    assert!(
        result.contains("-->"),
        "expected Mermaid edge syntax: {result}"
    );
    // P1: both depth-1 neighbors are guaranteed by seed; AND not OR.
    assert!(
        result.contains("concept_b") && result.contains("concept_c"),
        "expected both depth-1 neighbors in diagram: {result}"
    );
    // Merged from visualize_includes_co_occurrence_counts: verify seeded weight value.
    assert!(
        result.contains("-->|2|"),
        "expected co-occurrence edge labels in output: {result}"
    );
}

// T-1.6: RTM NFR-1 — depth 2 must produce strictly more output than depth 1 given
// the deterministic seed graph (a→b, a→c at depth 1; b→d, c→e at depth 2).
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

    // P0: seed guarantees strictly more edges at depth 2; >= masks regressions.
    assert!(
        depth_two_lines > depth_one_lines,
        "depth 2 must produce strictly more lines than depth 1:\n{depth_one}\n---\n{depth_two}"
    );
    // P2: assert on concepts that only appear at depth 2, not depth 1.
    assert!(
        depth_two.contains("concept_d") && depth_two.contains("concept_e"),
        "depth 2 should include depth-2 concepts d and e: {depth_two}"
    );
    assert!(
        !depth_one.contains("concept_d") && !depth_one.contains("concept_e"),
        "depth 1 should not include depth-2 concepts: {depth_one}"
    );
}

// T-1.6: RTM NFR-1 — case variants of the same concept must produce identical output.
#[test]
fn visualize_normalizes_concept_names() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_graph(&cfg);

    let reference = visualize(
        &cfg,
        json!({
            "conceptName": "concept-a",
            "depth": 1,
            "maxNodes": 10
        }),
    );
    assert!(
        reference.starts_with("graph TD"),
        "reference (lowercase) should produce valid diagram: {reference}"
    );

    // P0 + P3: loop over variants, assert identical output to reference.
    for variant in ["CONCEPT-A", "Concept-A"] {
        let result = visualize(
            &cfg,
            json!({
                "conceptName": variant,
                "depth": 1,
                "maxNodes": 10
            }),
        );
        assert_eq!(
            result, reference,
            "variant '{variant}' should normalize to identical output as lowercase"
        );
    }
}
