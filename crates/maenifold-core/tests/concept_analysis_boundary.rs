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
