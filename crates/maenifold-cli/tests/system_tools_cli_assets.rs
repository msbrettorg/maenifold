mod system_tools_cli_support;

// T-12.7: RTM NFR-1 - preserve asset-oriented system tool CLI coverage after splitting the monolithic test file.
use serde_json::Value;
use std::path::Path;
use system_tools_cli_support::*;

#[test]
fn analyze_concept_corruption_json_flag_learn_true_returns_help_documentation() {
    // T-1.4b: RTM FR-2 (TC-2.5 / RTM-012)
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "AnalyzeConceptCorruption",
            r#"{"conceptFamily":"tool","learn":true}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_or_text(output);
    assert!(
        rendered.contains("AnalyzeConceptCorruption"),
        "expected help output under --json: {rendered}"
    );
    assert!(
        !rendered
            .trim_start()
            .starts_with("=== Concept Family Analysis"),
        "learn=true should not start with live analysis output: {rendered}"
    );
}

#[test]
fn analyze_concept_corruption_json_flag_reports_zero_variants_for_unknown_family() {
    // T-1.4b: RTM FR-2 (TC-2.4 / RTM-011)
    let tmp = isolated_root();
    write_concept_analysis_fixture(&tmp);

    let output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "AnalyzeConceptCorruption",
            r#"{"conceptFamily":"zzz_nonexistent"}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_or_text(output);
    assert!(
        rendered.contains("Found 0 unique variants"),
        "expected empty-family summary under --json: {rendered}"
    );
}

// T-1.4a: RTM FR-1 (RTM-006 / CLI learn contract)
#[test]
fn adopt_learn_true_json_returns_success_envelope() {
    let tmp = isolated_root();
    write_json_asset(
        &tmp,
        "roles/test.json",
        r#"{"id":"test","name":"Test Role","principles":["accuracy"]}"#,
    );

    let output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "Adopt",
            r#"{"type":"role","identifier":"test","learn":true}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(value["success"], true, "unexpected output: {value}");
    assert!(
        value["data"]
            .as_str()
            .unwrap_or_default()
            .contains("# Adopt"),
        "unexpected output: {value}"
    );
}

// T-1.4a: RTM FR-1 (RTM-004 / CLI invalid type)
#[test]
fn adopt_invalid_type_json_returns_error_envelope() {
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "Adopt",
            r#"{"type":"invalid","identifier":"x"}"#,
            "--json",
        ])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(value["success"], false, "unexpected output: {value}");
    assert_eq!(value["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        value["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("Invalid type"),
        "unexpected error message: {value}"
    );
}

// T-1.4c: RTM FR-3 (RTM-018 / CLI learn contract)
#[test]
fn update_assets_json_flag_learn_true_returns_help_documentation() {
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "UpdateAssets",
            r#"{"learn":true}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let rendered = render_json_or_text(output);
    assert!(
        rendered.contains("UpdateAssets") || rendered.contains("Refreshes"),
        "expected help output under --json: {rendered}"
    );
    assert!(
        !rendered.contains("Asset Update Summary"),
        "learn=true should not render the live update summary: {rendered}"
    );
}

// T-1.4c: RTM FR-3 (RTM-014 / CLI dry-run JSON contract)
#[test]
fn update_assets_json_flag_dry_run_reports_changed_files() {
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();
    let relative = unique_asset_relative("roles", "t14c-cli-updated");
    let file_name = Path::new(&relative)
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let source = PackagedAssetGuard::create(&relative, r#"{"id":"t14c-cli-updated","version":1}"#);

    maenifold_cmd(&tmp)
        .args(["tool", "GetConfig", "{}", "--json"])
        .assert()
        .code(0);

    write_file(source.path(), r#"{"id":"t14c-cli-updated","version":2}"#);

    let output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "UpdateAssets",
            r#"{"dryRun":true}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    if value.get("success").is_some() {
        assert_eq!(value["success"], true, "unexpected output: {value}");
    }

    let rendered = render_json_or_text(output);
    assert!(
        rendered.contains("filesUpdated") || rendered.contains("updatedFiles"),
        "expected structured changed-file fields under --json: {rendered}"
    );
    assert!(
        rendered.contains(&file_name),
        "expected changed file to appear in JSON output: {rendered}"
    );
}
