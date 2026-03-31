#[path = "concept_analysis_support.rs"]
mod concept_analysis_support;

use concept_analysis_support::{
    ConfigOverrideReset, TEST_MUTEX, analyze_concept_corruption, isolated_config,
    write_memory_fixture,
};
use serde_json::json;

/// Returns the text of a report section from its header to the next section header.
/// Section headers are non-empty lines that start at column 0 (not indented).
/// Bullet items and blank lines are considered part of the current section.
fn section_text(output: &str, header: &str) -> String {
    let Some(idx) = output.find(header) else {
        return String::new();
    };
    let rest = &output[idx..];
    let mut lines = rest.lines();
    let header_line = lines.next().unwrap_or("");
    let mut result = String::from(header_line);
    result.push('\n');
    for line in lines {
        // A non-empty, non-indented line signals the next section header.
        if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

// MERGED tests 1 + 2: parameterized file-path classification (slash-prefix + dot-notation).
#[test]
fn analyze_concept_corruption_classifies_file_path_pattern_variants() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();

    let cases: &[(&str, &str, &str, &str)] = &[
        (
            "concept-analysis/path-slash.md",
            "See [[/src/workflow]] for details.\n",
            "/src/workflow",
            "slash-prefixed",
        ),
        (
            "concept-analysis/path-dot.md",
            "See [[workflow.json]] for the config.\n",
            "workflow.json",
            "dot-notation",
        ),
    ];

    for &(fixture_path, content, expected_concept, label) in cases {
        let _reset = ConfigOverrideReset;
        let (_tmp, cfg) = isolated_config();
        write_memory_fixture(&cfg, fixture_path, content);

        let output = analyze_concept_corruption(
            &cfg,
            json!({
                "conceptFamily": "workflow"
            }),
        );

        assert!(
            output.contains("File Paths:"),
            "{label} concepts should classify as file paths: {output}"
        );
        assert!(
            output.contains(expected_concept),
            "{label}: expected concept '{expected_concept}' in file-path bucket: {output}"
        );
    }
}

// REWRITTEN: renamed from classifies_unmatched_variants_as_other; [[workflow]] is the
// canonical form, not a "variant". Replaced brittle count-format coupling ("workflow (1x)")
// with section-scoped check that decouples from count rendering.
#[test]
fn analyze_concept_corruption_classifies_canonical_form_as_other() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/other-concept.md",
        "The [[workflow]] definition.\n",
    );

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "workflow"
        }),
    );

    assert!(
        output.contains("Other:"),
        "canonical form should fall into Other bucket: {output}"
    );
    let other_section = section_text(&output, "Other:");
    assert!(
        other_section.contains("workflow"),
        "expected canonical form 'workflow' in Other section: {output}"
    );
}

// KEEP: unchanged
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

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "workflow"
        }),
    );

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

// REWRITTEN: added missing RepairConcepts assertion for parity with plural repair test.
// The RepairConcepts invocation snippet is the most actionable part of the suggestion output;
// without this assertion the compound-repair pathway can regress silently.
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

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "workflow"
        }),
    );

    assert!(
        output.contains("Fix compound forms:"),
        "compound variants should produce a compound repair suggestion: {output}"
    );
    assert!(
        output.contains("RepairConcepts"),
        "compound repair suggestion should mention RepairConcepts: {output}"
    );
    assert!(
        output.contains("workflow-dispatch"),
        "repair suggestion should include compound variant: {output}"
    );
}

// REWRITTEN: replaced fragile compound negative assertion
// (`!output.contains("Singular/Plural:\n  • /workflows")`) with section-scoped checks.
// The old assertion embedded a literal newline, Unicode bullet, and section header in one
// string — any format-only change would silently invert it to a false pass.
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

    let output = analyze_concept_corruption(
        &cfg,
        json!({
            "conceptFamily": "workflow"
        }),
    );

    // Positive: File Paths section exists and contains /workflows.
    assert!(
        output.contains("File Paths:"),
        "path pattern should take priority over plural classification: {output}"
    );
    let file_paths = section_text(&output, "File Paths:");
    assert!(
        file_paths.contains("/workflows"),
        "/workflows should appear in File Paths section: {output}"
    );

    // Negative: if Singular/Plural section exists, /workflows must not appear in it.
    if output.contains("Singular/Plural:") {
        let plural_section = section_text(&output, "Singular/Plural:");
        assert!(
            !plural_section.contains("/workflows"),
            "/workflows should not appear in Singular/Plural section: {output}"
        );
    }
}
