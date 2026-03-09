use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// T-1.4b: RTM FR-2

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const CONCEPT_ANALYSIS_FIXTURE: &str = r#"---
title: Test Note
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
This note discusses [[tool]], [[tool]], [[tools]], [[tools]], [[coding-tool]], [[tool system]], and [[GraphTools.cs]].
"#;

struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    (tmp, cfg)
}

fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write memory fixture");
}

fn analyze_concept_corruption(payload: serde_json::Value) -> String {
    ToolRegistry::global()
        .try_invoke("AnalyzeConceptCorruption", &payload, OutputFormat::Markdown)
        .expect("registry invoke")
}

fn report_body_before_repairs(output: &str) -> &str {
    output.split("SUGGESTED REPAIRS").next().unwrap_or(output)
}

fn bullet_count(output: &str) -> usize {
    output.lines().filter(|line| line.contains("  • ")).count()
}

#[test]
fn analyze_concept_corruption_scans_memory_files_and_reports_variant_counts() {
    // T-1.4b: RTM FR-2 (TC-2.1 / RTM-008)
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
    // T-1.4b: RTM FR-2 (TC-2.2 / RTM-009)
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
    // T-1.4b: RTM FR-2 (TC-2.3 / RTM-010)
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
    // T-1.4b: RTM FR-2 (TC-2.4 / RTM-011)
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
    // T-1.4b: RTM FR-2 (TC-2.5 / RTM-012)
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
    // T-1.4b: RTM FR-2 (TC-2.6 / RTM-013)
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

#[test]
fn analyze_concept_corruption_empty_memory_directory_reports_zero_unique_variants() {
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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
fn analyze_concept_corruption_classifies_slash_prefixed_concepts_as_file_paths() {
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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
fn analyze_concept_corruption_reports_total_unique_variants_before_max_results_truncation() {
    // T-W1-4: RTM FR-2
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
    // T-W1-4: RTM FR-2
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

    let output = analyze_concept_corruption(json!({
        "conceptFamily": "workflow"
    }));

    assert!(
        output.contains("Found 0 unique variants"),
        "hidden .git directories should be excluded from analysis: {output}"
    );
}

#[test]
fn analyze_concept_corruption_prefers_path_classification_over_plural_classification() {
    // T-W1-4: RTM FR-2
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
