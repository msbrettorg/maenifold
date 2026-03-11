mod concept_analysis_support;

use concept_analysis_support::{
    CONCEPT_ANALYSIS_FIXTURE, ConfigOverrideReset, TEST_MUTEX, analyze_concept_corruption,
    isolated_config, report_body_before_repairs, write_memory_fixture,
};
use serde_json::json;

#[test]
fn analyze_concept_corruption_scans_memory_files_and_reports_variant_counts() {
    // T-1.4: RTM FR-2 (TC-2.1 / RTM-008)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/test-note.md",
        CONCEPT_ANALYSIS_FIXTURE,
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "tool"
    }));

    assert!(
        output.contains("Concept Family Analysis"),
        "missing report header: {output}"
    );
    assert!(output.contains("tool (2x)"), "missing tool count: {output}");
    assert!(
        output.contains("tools (2x)"),
        "missing tools count: {output}"
    );
}

#[test]
fn analyze_concept_corruption_classifies_variants_into_expected_categories() {
    // T-1.4: RTM FR-2 (TC-2.2 / RTM-009)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/test-note.md",
        CONCEPT_ANALYSIS_FIXTURE,
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "tool"
    }));

    assert!(
        output.contains("Singular/Plural:") && output.contains("tools"),
        "plural bucket missing: {output}"
    );
    assert!(
        output.contains("Compound (with -):") && output.contains("coding-tool"),
        "compound bucket missing: {output}"
    );
    assert!(
        output.contains("With Suffix:") && output.contains("tool system"),
        "suffix bucket missing: {output}"
    );
    assert!(
        output.contains("File Paths:") && output.contains("GraphTools.cs"),
        "file-path bucket missing: {output}"
    );
}

#[test]
fn analyze_concept_corruption_respects_max_results_limit() {
    // T-1.4: RTM FR-2 (TC-2.3 / RTM-010)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/max-results.md",
        r#"---
title: Max Results
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
Variants: [[alpha-tool]], [[beta-tool]], [[gamma-tool]], [[delta-tool]].
"#,
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "tool",
        "maxResults": 2
    }));

    let report_body = report_body_before_repairs(&output);
    let visible_variants = ["alpha-tool", "beta-tool", "gamma-tool", "delta-tool"]
        .into_iter()
        .filter(|variant| report_body.contains(variant))
        .count();

    assert!(
        visible_variants <= 2,
        "expected at most 2 visible variants before repairs, saw {visible_variants}: {output}"
    );
}

#[test]
fn analyze_concept_corruption_returns_empty_report_for_unknown_family() {
    // T-1.4: RTM FR-2 (TC-2.4 / RTM-011)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/test-note.md",
        CONCEPT_ANALYSIS_FIXTURE,
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "zzz_nonexistent"
    }));

    assert!(
        output.contains("Found 0 unique variants"),
        "expected empty-family summary: {output}"
    );
}

#[test]
fn analyze_concept_corruption_learn_true_returns_help_doc_instead_of_analysis() {
    // T-1.4: RTM FR-2 (TC-2.5 / RTM-012)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "tool",
        "learn": true
    }));

    assert!(
        output.contains("AnalyzeConceptCorruption"),
        "expected help content to mention tool name: {output}"
    );
    assert!(
        !output
            .trim_start()
            .starts_with("=== Concept Family Analysis"),
        "learn=true should return help documentation, not start with live analysis output: {output}"
    );
}

#[test]
fn analyze_concept_corruption_suggests_repair_commands_for_safe_merges() {
    // T-1.4: RTM FR-2 (TC-2.6 / RTM-013)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/test-note.md",
        CONCEPT_ANALYSIS_FIXTURE,
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "tool"
    }));

    assert!(
        output.contains("SUGGESTED REPAIRS"),
        "repair suggestion section missing: {output}"
    );
    assert!(
        output.contains("RepairConcepts"),
        "expected RepairConcepts command text: {output}"
    );
    assert!(
        output.contains("tools") && output.contains("tool"),
        "expected plural-to-singular repair suggestion: {output}"
    );
}
