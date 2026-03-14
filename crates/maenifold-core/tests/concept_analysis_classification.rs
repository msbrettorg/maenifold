#[path = "concept_analysis_support.rs"]
mod concept_analysis_support;

use concept_analysis_support::{
    ConfigOverrideReset, TEST_MUTEX, analyze_concept_corruption, isolated_config,
    write_memory_fixture,
};
use serde_json::json;

#[test]
fn analyze_concept_corruption_classifies_slash_prefixed_concepts_as_file_paths() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/path-concept.md",
        "See [[/src/workflow]] for details.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("File Paths:"),
        "slash-prefixed concepts should classify as file paths: {output}"
    );
    assert!(
        output.contains("/src/workflow"),
        "expected slash-prefixed concept in file-path bucket: {output}"
    );
}

#[test]
fn analyze_concept_corruption_classifies_dot_concepts_as_file_paths() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/dot-concept.md",
        "See [[workflow.json]] for the config.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("File Paths:"),
        "dot-containing concepts should classify as file paths: {output}"
    );
    assert!(
        output.contains("workflow.json"),
        "expected dot concept in file-path bucket: {output}"
    );
}

#[test]
fn analyze_concept_corruption_classifies_unmatched_variants_as_other() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/other-concept.md",
        "The [[workflow]] definition.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Other:"),
        "plain workflow concept should fall into Other bucket: {output}"
    );
    assert!(
        output.contains("workflow (1x)"),
        "expected workflow entry in Other bucket: {output}"
    );
}

#[test]
fn analyze_concept_corruption_produces_plural_repair_suggestions() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/plural-repair.md",
        "Using [[workflows]] in the pipeline.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Fix plural forms:"),
        "plural variants should produce a plural repair suggestion: {output}"
    );
    assert!(
        output.contains("RepairConcepts"),
        "repair suggestion should mention RepairConcepts: {output}"
    );
    assert!(
        output.contains("workflows"),
        "repair suggestion should include plural variant: {output}"
    );
}

#[test]
fn analyze_concept_corruption_produces_compound_repair_suggestions() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/compound-repair.md",
        "The [[workflow-dispatch]] event is fired.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Fix compound forms:"),
        "compound variants should produce a compound repair suggestion: {output}"
    );
    assert!(
        output.contains("workflow-dispatch"),
        "repair suggestion should include compound variant: {output}"
    );
}

#[test]
fn analyze_concept_corruption_prefers_path_classification_over_plural_classification() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/priority-test.md",
        "See [[/workflows]] for the path.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("File Paths:"),
        "path pattern should take priority over plural classification: {output}"
    );
    assert!(
        !output.contains("Singular/Plural:\n  • /workflows"),
        "path-like plural should not be listed under Singular/Plural: {output}"
    );
}
