mod system_tools_cli_support;

// T-12.7: RTM NFR-1 - preserve runtime/reporting system tool CLI coverage after splitting the monolithic test file.
use std::fs;
use system_tools_cli_support::*;

#[test]
fn recent_activity_json_flag_learn_true_returns_help_documentation() {
    // T-2.5a: RTM FR-4 (TC-4.7 / RTM-028 / CLI learn contract)
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "RecentActivity",
            "--payload",
            r#"{"learn":true}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_data_or_text(output);
    assert!(
        rendered.contains("RecentActivity"),
        "expected help output under --json: {rendered}"
    );
    assert!(
        rendered.contains("timespan") || rendered.contains("Include full section content"),
        "expected parameter documentation under --json: {rendered}"
    );
    assert!(
        !rendered.starts_with("# Recent Activity"),
        "learn=true should not render live activity output: {rendered}"
    );
}

#[test]
fn recent_activity_json_flag_empty_database_reports_no_recent_activity() {
    // T-2.5a: RTM FR-4 (TC-4.6 / RTM-027 / CLI empty-state contract)
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "RecentActivity",
            "--payload",
            r#"{"limit":10}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_data_or_text(output);
    assert_eq!(rendered.trim(), "No recent activity found");
}

#[test]
fn run_full_benchmark_json_flag_learn_true_returns_help_documentation() {
    // T-2.5b: RTM FR-5 (RTM-036 / TC-5.6 / CLI learn contract)
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "RunFullBenchmark",
            "--payload",
            r#"{"learn":true}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_data_or_text(output);
    assert!(
        rendered.contains("RunFullBenchmark"),
        "expected help output under --json: {rendered}"
    );
    assert!(
        rendered.contains("iterations") || rendered.contains("Iterations"),
        "expected parameter docs in help output: {rendered}"
    );
    let outside_code_blocks: String = rendered.split("```").step_by(2).collect();
    assert!(
        !outside_code_blocks.contains("Started: 20"),
        "learn=true should not produce live benchmark output: {rendered}"
    );
}

#[test]
fn run_full_benchmark_json_flag_iterations_zero_returns_error_envelope() {
    // T-2.5b: RTM FR-5 (RTM-032 / TC-5.2 / CLI iterations=0 error)
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "RunFullBenchmark",
            "--payload",
            r#"{"iterations":0}"#,
            "--json",
        ])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(value["success"], false, "unexpected output: {value}");
    assert_eq!(
        value["error"]["code"], "INVALID_ARGUMENT",
        "expected INVALID_ARGUMENT error code: {value}"
    );
    assert!(
        value["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("iterations must be greater than zero"),
        "unexpected error message: {value}"
    );
}

#[test]
fn repair_concepts_json_flag_applies_wikilink_replacements() {
    // T-3.3: RTM FR-6 (RTM-038 / TC-6.1 / CLI apply contract)
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();
    write_markdown_file(
        &tmp,
        "repair/apply.md",
        r#"---
title: Repair Apply
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
Using [[tools]] and [[coding-tool]].
"#,
    );

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "RepairConcepts",
            "--payload",
            r#"{"conceptsToReplace":"tools","canonicalConcept":"tool","dryRun":false,"minSemanticSimilarity":0.0}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_data_or_text(output);
    let updated = fs::read_to_string(tmp.path().join("memory/repair/apply.md")).expect("read file");
    assert!(
        rendered.contains("Modified: repair/apply") || rendered.contains("repair/apply"),
        "expected applied change summary: {rendered}"
    );
    assert!(
        updated.contains("[[tool]]"),
        "canonical link missing: {updated}"
    );
    assert!(
        !updated.contains("[[tools]]"),
        "variant should be replaced: {updated}"
    );
}

#[test]
fn repair_concepts_json_flag_dry_run_reports_summary_and_preserves_file() {
    // T-3.3: RTM FR-6 (RTM-040 / TC-6.3 / CLI dry-run contract)
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();
    write_markdown_file(
        &tmp,
        "repair/dry-run.md",
        r#"---
title: Repair Dry Run
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
Using [[tools]] for a dry-run.
"#,
    );

    let original = fs::read_to_string(tmp.path().join("memory/repair/dry-run.md")).expect("read");
    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "RepairConcepts",
            "--payload",
            r#"{"conceptsToReplace":"tools","canonicalConcept":"tool","dryRun":true,"minSemanticSimilarity":0.0}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_data_or_text(output);
    let after = fs::read_to_string(tmp.path().join("memory/repair/dry-run.md")).expect("read");
    assert!(
        rendered.contains("Would modify: repair/dry-run"),
        "expected dry-run summary: {rendered}"
    );
    assert!(
        rendered.contains("DRY RUN"),
        "missing dry-run marker: {rendered}"
    );
    assert_eq!(after, original, "dry-run should not change file contents");
}

#[test]
fn repair_concepts_json_flag_learn_true_returns_help_documentation() {
    // T-3.3: RTM FR-6 (RTM-046 / TC-6.9 / CLI learn contract)
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "RepairConcepts",
            "--payload",
            r#"{"conceptsToReplace":"tools","canonicalConcept":"tool","learn":true}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_data_or_text(output);
    assert!(
        rendered.contains("RepairConcepts"),
        "expected help output under --json: {rendered}"
    );
    let outside_code_blocks: String = rendered.split("```").step_by(2).collect();
    assert!(
        !outside_code_blocks.contains("\nScanning "),
        "learn=true should return documentation instead of repair execution output: {rendered}"
    );
}
