#[path = "concept_analysis_support.rs"]
mod concept_analysis_support;

use concept_analysis_support::{
    CONCEPT_ANALYSIS_FIXTURE, ConfigOverrideReset, TEST_MUTEX, analyze_concept_corruption,
    isolated_config, report_body_before_repairs, try_analyze_concept_corruption,
    write_memory_fixture,
};
use serde_json::json;

#[test]
fn analyze_concept_corruption_reports_variant_counts_from_single_file() {
    // T-1.4: RTM FR-2 (TC-2.1 / RTM-008)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/test-note.md",
        CONCEPT_ANALYSIS_FIXTURE,
    );

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "tool"
        }),
    );

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

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "tool"
        }),
    );

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

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "tool",
            "maxResults": 2
        }),
    );

    let report_body = report_body_before_repairs(&output);

    // Exact count: 4 variants in fixture, maxResults=2 → exactly 2 visible.
    let visible_variants = ["alpha-tool", "beta-tool", "gamma-tool", "delta-tool"]
        .into_iter()
        .filter(|variant| report_body.contains(variant))
        .count();
    assert!(
        visible_variants == 2,
        "expected exactly 2 visible variants before repairs, saw {visible_variants}: {output}"
    );

    // Sort order: all variants have count 1, so sorted alphabetically.
    // Expected survivors: alpha-tool, beta-tool. Dropped: delta-tool, gamma-tool.
    assert!(
        report_body.contains("alpha-tool"),
        "alpha-tool should survive truncation (first alphabetically): {output}"
    );
    assert!(
        report_body.contains("beta-tool"),
        "beta-tool should survive truncation (second alphabetically): {output}"
    );
    assert!(
        !report_body.contains("delta-tool"),
        "delta-tool should be truncated (third alphabetically, beyond maxResults=2): {output}"
    );
    assert!(
        !report_body.contains("gamma-tool"),
        "gamma-tool should be truncated (fourth alphabetically, beyond maxResults=2): {output}"
    );
}

#[test]
fn analyze_concept_corruption_returns_zero_variants_for_unknown_family() {
    // T-1.4: RTM FR-2 (TC-2.4 / RTM-011)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/test-note.md",
        CONCEPT_ANALYSIS_FIXTURE,
    );

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "zzz_nonexistent"
        }),
    );

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
    let (_tmp, cfg) = isolated_config();

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "tool",
            "learn": true
        }),
    );

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

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "tool"
        }),
    );

    assert!(
        output.contains("SUGGESTED REPAIRS"),
        "repair suggestion section missing: {output}"
    );
    assert!(
        output.contains("RepairConcepts"),
        "expected RepairConcepts command text: {output}"
    );
    // Section-scoped check: the plural variant "tools" must appear as a
    // conceptsToReplace argument in the repair command, not merely anywhere
    // in the output (the old assertion `output.contains("tools")` trivially
    // passed because "tools" appeared in variant listings and section headers).
    assert!(
        output.contains("conceptsToReplace='tools'"),
        "expected plural-to-singular repair command with conceptsToReplace='tools': {output}"
    );
}

// --- Error-path tests (P1, recommendation 3) ---

#[test]
fn analyze_concept_corruption_rejects_missing_concept_family() {
    // T-1.4: RTM FR-2 (error-path: MissingProperty)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let result = try_analyze_concept_corruption(&cfg, json!({}));

    assert!(
        result.is_err(),
        "missing conceptFamily should produce an error"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("conceptFamily"),
        "error should mention the missing property: {err}"
    );
}

#[test]
fn analyze_concept_corruption_rejects_empty_concept_family() {
    // T-1.4: RTM FR-2 (error-path: InvalidArgument)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let result = try_analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": ""
        }),
    );

    assert!(
        result.is_err(),
        "empty conceptFamily should produce an error"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("conceptFamily") && err.contains("empty"),
        "error should explain conceptFamily cannot be empty: {err}"
    );
}

#[test]
fn analyze_concept_corruption_rejects_negative_max_results() {
    // T-1.4: RTM FR-2 (error-path: InvalidArgument)
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let result = try_analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "tool",
            "maxResults": -1
        }),
    );

    assert!(
        result.is_err(),
        "negative maxResults should produce an error"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("maxResults"),
        "error should mention maxResults: {err}"
    );
}
