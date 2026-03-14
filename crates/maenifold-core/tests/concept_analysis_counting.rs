#[path = "concept_analysis_support.rs"]
mod concept_analysis_support;

use concept_analysis_support::{
    ConfigOverrideReset, TEST_MUTEX, analyze_concept_corruption, bullet_count, isolated_config,
    report_body_before_repairs, write_memory_fixture,
};
use serde_json::json;

#[test]
fn analyze_concept_corruption_empty_memory_directory_reports_zero_unique_variants() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("=== Concept Family Analysis: 'workflow' ==="),
        "missing exact concept family header: {output}"
    );
    assert!(
        output.contains("Found 0 unique variants"),
        "empty memory directory should report zero variants: {output}"
    );
}

#[test]
fn analyze_concept_corruption_extracts_wikilinks_from_markdown_content() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/wikilink-test.md",
        "---\ntags: []\n---\n# Test\n\nUsing [[workflow]] here.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Found 1 unique variants"),
        "expected one extracted wikilink variant: {output}"
    );
    assert!(
        output.contains("workflow (1x)"),
        "expected extracted workflow count: {output}"
    );
}

#[test]
fn analyze_concept_corruption_counts_multiple_wikilinks_in_the_same_file() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/multi-wikilink.md",
        "# Test\n\n[[workflow]] again [[workflow]] and [[workflow]].\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Found 1 unique variants"),
        "expected a single unique variant for repeated wikilinks: {output}"
    );
    assert!(
        output.contains("workflow (3x)"),
        "expected repeated workflow mentions to aggregate to 3x: {output}"
    );
}

#[test]
fn analyze_concept_corruption_counts_multiple_matching_variants_in_one_file() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/single-file.md",
        "[[workflow]] is used here and [[workflow-dispatch]] is used there.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Found 2 unique variants"),
        "expected two unique variants in a single file: {output}"
    );
    assert!(
        output.contains("workflow (1x)") && output.contains("workflow-dispatch (1x)"),
        "expected both matching concepts to be counted: {output}"
    );
}

#[test]
fn analyze_concept_corruption_aggregates_counts_across_multiple_files() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/file-a.md",
        "# File A\n\n[[workflow]] is mentioned here.\n",
    );
    write_memory_fixture(
        &cfg,
        "concept-analysis/file-b.md",
        "# File B\n\n[[workflow]] appears again.\n",
    );
    write_memory_fixture(
        &cfg,
        "concept-analysis/file-c.md",
        "# File C\n\n[[workflows]] is a plural form.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Found 2 unique variants"),
        "expected workflow/workflows aggregation across files: {output}"
    );
    assert!(
        output.contains("workflow (2x)"),
        "expected workflow aggregate count of 2: {output}"
    );
    assert!(
        output.contains("workflows (1x)"),
        "expected workflows aggregate count of 1: {output}"
    );
}

#[test]
fn analyze_concept_corruption_reports_total_unique_variants_before_max_results_truncation() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/max-results-total.md",
        "[[workflow]] [[workflow-a]] [[workflow-b]] [[workflow-c]] [[workflow-d]]\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow",
        "maxResults": 2
    }));

    assert!(
        output.contains("Found 5 unique variants"),
        "header should report total variants before truncation: {output}"
    );
    assert!(
        bullet_count(report_body_before_repairs(&output)) <= 2,
        "maxResults=2 should limit visible bullet entries: {output}"
    );
}

#[test]
fn analyze_concept_corruption_excludes_files_in_git_directories() {
    // T-1.4: RTM FR-2
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "concept-analysis/.git/git-tracked.md",
        "[[workflow]] should not be counted.\n",
    );
    write_memory_fixture(
        &cfg,
        "concept-analysis/legitimate.md",
        "No workflow wikilinks here.\n",
    );
    write_memory_fixture(
        &cfg,
        "concept-analysis/visible.md",
        "# Visible\n\nUsing [[workflow]] in a non-hidden file.\n",
    );

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Found 1 unique variants"),
        "non-hidden file should be counted, .git file should be excluded: {output}"
    );
    assert!(
        output.contains("workflow (1x)"),
        "only the non-hidden file's wikilink should contribute to the count: {output}"
    );
}
