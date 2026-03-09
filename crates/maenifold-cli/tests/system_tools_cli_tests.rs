use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::{Connection, params};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

fn write_markdown_file(tmp: &TempDir, relative: &str, content: &str) {
    let path = tmp.path().join("memory").join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write markdown");
}

fn seed_last_accessed(tmp: &TempDir, uri: &str, last_accessed: &str) {
    let db_path = tmp.path().join("memory.db");
    let connection = Connection::open(db_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO file_content (file_path, last_indexed, last_accessed) VALUES (?1, ?2, ?3)",
            params![uri, "2026-03-08T00:00:00Z", last_accessed],
        )
        .expect("seed last_accessed");
}

fn write_json_asset(tmp: &TempDir, relative: &str, content: &str) {
    let path = tmp.path().join("assets").join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write asset");
}

fn write_concept_analysis_fixture(tmp: &TempDir) {
    write_markdown_file(
        tmp,
        "concept-analysis/test-note.md",
        r#"---
title: Test Note
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
This note discusses [[tool]], [[tool]], [[tools]], [[tools]], [[coding-tool]], [[tool system]], and [[GraphTools.cs]].
"#,
    );
}

fn render_json_or_text(output: Vec<u8>) -> String {
    let text = String::from_utf8(output).expect("utf8");
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::String(rendered)) => rendered,
        Ok(value) => value.to_string(),
        Err(_) => text,
    }
}

fn render_json_data_or_text(output: Vec<u8>) -> String {
    let text = String::from_utf8(output).expect("utf8");
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::String(rendered)) => rendered,
        Ok(value) => value
            .get("data")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.to_string()),
        Err(_) => text,
    }
}

fn test_mutex() -> &'static Mutex<()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX.get_or_init(|| Mutex::new(()))
}

fn workspace_assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("assets")
}

fn unique_asset_relative(dir: &str, prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{dir}/{prefix}-{nanos}.json")
}

fn write_file(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write file");
}

fn remove_empty_ancestors(mut current: Option<&Path>, stop_at: &Path) {
    while let Some(path) = current {
        if path == stop_at {
            break;
        }
        match fs::remove_dir(path) {
            Ok(()) => current = path.parent(),
            Err(_) => break,
        }
    }
}

struct PackagedAssetGuard {
    path: PathBuf,
    assets_root: PathBuf,
}

impl PackagedAssetGuard {
    fn create(relative: &str, content: &str) -> Self {
        let assets_root = workspace_assets_root();
        let path = assets_root.join(relative);
        write_file(&path, content);
        Self { path, assets_root }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PackagedAssetGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        remove_empty_ancestors(self.path.parent(), &self.assets_root);
    }
}

#[test]
fn list_memories_json_mode_still_returns_markdown_directory_listing() {
    let tmp = isolated_root();

    maenifold_cmd(&tmp)
        .args(["--tool", "ListMemories", "--payload", "{}", "--json"])
        .assert()
        .code(0)
        .stdout(predicate::str::starts_with("# Directory: /"));
}

#[test]
fn list_memories_nonexistent_path_returns_directory_not_found() {
    let tmp = isolated_root();

    maenifold_cmd(&tmp)
        .args([
            "--tool",
            "ListMemories",
            "--payload",
            r#"{"path":"non-existent-directory-xyz"}"#,
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Directory not found: non-existent-directory-xyz",
        ));
}

#[test]
fn list_memories_learn_true_returns_help_documentation() {
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args(["--tool", "ListMemories", "--payload", r#"{"learn":true}"#])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("utf8");
    assert!(text.contains("ListMemories") || text.contains("directory") || text.contains("folder"));
}

#[test]
fn list_memories_includes_decay_metadata_and_top_level_folders() {
    let tmp = isolated_root();
    let root_payload = r#"{"title":"Seed Root","content":"Testing [[system-tools]]."}"#;
    maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", root_payload])
        .assert()
        .code(0);

    write_markdown_file(
        &tmp,
        "decay-metadata-tests/standard/created-test.md",
        "---\ntitle: Created Test\ntype: memory\nstatus: saved\ncreated: 2026-01-15T10:30:00Z\nmodified: 2026-01-15T10:30:00Z\n---\n\n# Created Test\n\nTest content for [[decay-metadata]].\n",
    );
    write_markdown_file(
        &tmp,
        "decay-metadata-tests/hidden/.ignored.md",
        "# Hidden\n\n[[hidden]].\n",
    );
    seed_last_accessed(
        &tmp,
        "memory://decay-metadata-tests/standard/created-test",
        "2026-01-20T14:45:00Z",
    );

    let parent_output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "ListMemories",
            "--payload",
            r#"{"path":"decay-metadata-tests"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let parent_text = String::from_utf8(parent_output).expect("utf8");
    assert!(parent_text.contains("# Directory: decay-metadata-tests"));
    assert!(parent_text.contains("### Folders"));
    assert!(parent_text.contains("📁 standard/ (1 files)"));
    assert!(!parent_text.contains(".ignored.md"));

    let child_output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "ListMemories",
            "--payload",
            r#"{"path":"decay-metadata-tests/standard"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let child_text = String::from_utf8(child_output).expect("utf8");
    assert!(child_text.contains("created: 2026-01-15"));
    assert!(child_text.contains("last_accessed: 2026-01-20"));
    assert!(child_text.contains("decay_weight:"));
}

#[test]
fn get_help_with_valid_tool_name_returns_documentation() {
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "GetHelp",
            "--payload",
            r#"{"toolName":"writememory"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("utf8");
    assert!(text.contains("WriteMemory") || text.contains("memory") || text.contains('#'));
}

#[test]
fn get_help_invalid_tool_lists_available_tools() {
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "GetHelp",
            "--payload",
            r#"{"toolName":"non-existent-tool-xyz"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("utf8");
    assert!(
        text.contains("No help file found for tool: non-existent-tool-xyz")
            || text.contains("not found")
    );
    assert!(text.contains("Available tools with help"));
    assert!(text.contains("writememory"));
}

#[test]
fn get_help_is_case_insensitive() {
    let tmp = isolated_root();

    let lower = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "GetHelp",
            "--payload",
            r#"{"toolName":"writememory"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let upper = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "GetHelp",
            "--payload",
            r#"{"toolName":"WRITEMEMORY"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let mixed = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "GetHelp",
            "--payload",
            r#"{"toolName":"WriteMemory"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let lower = String::from_utf8(lower).expect("utf8");
    let upper = String::from_utf8(upper).expect("utf8");
    let mixed = String::from_utf8(mixed).expect("utf8");

    assert_eq!(lower, upper);
    assert_eq!(upper, mixed);
}

#[test]
fn analyze_concept_corruption_json_flag_learn_true_returns_help_documentation() {
    // T-1.4b: RTM FR-2 (TC-2.5 / RTM-012)
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "AnalyzeConceptCorruption",
            "--payload",
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
            "--tool",
            "AnalyzeConceptCorruption",
            "--payload",
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
            "--tool",
            "Adopt",
            "--payload",
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
            "--tool",
            "Adopt",
            "--payload",
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
            "--tool",
            "UpdateAssets",
            "--payload",
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
        .args(["--tool", "GetConfig", "--payload", "{}", "--json"])
        .assert()
        .code(0);

    write_file(source.path(), r#"{"id":"t14c-cli-updated","version":2}"#);

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "UpdateAssets",
            "--payload",
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

// T-2.5a: RTM FR-4 (TC-4.7 / RTM-028 / CLI learn contract)
#[test]
fn recent_activity_json_flag_learn_true_returns_help_documentation() {
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

// T-2.5a: RTM FR-4 (TC-4.6 / RTM-027 / CLI empty-state contract)
#[test]
fn recent_activity_json_flag_empty_database_reports_no_recent_activity() {
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

// T-2.5b: RTM FR-5 (RTM-036 / TC-5.6 / CLI learn contract)
#[test]
fn run_full_benchmark_json_flag_learn_true_returns_help_documentation() {
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
    // The help doc contains the suite header inside a code block example, so we
    // verify learn=true did NOT produce a live "Started: 20" timestamp line,
    // which only appears in actual benchmark runs (outside code blocks).
    let outside_code_blocks: String = rendered.split("```").step_by(2).collect();
    assert!(
        !outside_code_blocks.contains("Started: 20"),
        "learn=true should not produce live benchmark output: {rendered}"
    );
}

// T-2.5b: RTM FR-5 (RTM-032 / TC-5.2 / CLI iterations=0 error)
#[test]
fn run_full_benchmark_json_flag_iterations_zero_returns_error_envelope() {
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

    let value: Value = serde_json::from_slice(&output).expect("json");
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

// T-3.3: RTM FR-6 (RTM-038 / TC-6.1 / CLI apply contract)
#[test]
fn repair_concepts_json_flag_applies_wikilink_replacements() {
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
            r#"{"conceptsToReplace":"tools","canonicalConcept":"tool","dryRun":false}"#,
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
        rendered.contains("Modified: repair/apply.md") || rendered.contains("repair/apply.md"),
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

// T-3.3: RTM FR-6 (RTM-040 / TC-6.3 / CLI dry-run contract)
#[test]
fn repair_concepts_json_flag_dry_run_reports_summary_and_preserves_file() {
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
            r#"{"conceptsToReplace":"tools","canonicalConcept":"tool","dryRun":true}"#,
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
        rendered.contains("Would modify: repair/dry-run.md"),
        "expected dry-run summary: {rendered}"
    );
    assert!(
        rendered.contains("DRY RUN"),
        "missing dry-run marker: {rendered}"
    );
    assert_eq!(after, original, "dry-run should not change file contents");
}

// T-3.3: RTM FR-6 (RTM-046 / TC-6.9 / CLI learn contract)
#[test]
fn repair_concepts_json_flag_learn_true_returns_help_documentation() {
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

// T-3.3: RTM FR-13.6 / RTM FR-13.7 / RTM FR-13.10
#[test]
fn sync_and_build_context_cli_surface_real_community_ids_and_siblings() {
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();
    for (relative, content) in [
        (
            "clusters/alpha-1.md",
            r#"---
title: Alpha Cluster 1
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Alpha
Links [[alpha]], [[beta]], and [[gamma]].
"#,
        ),
        (
            "clusters/alpha-2.md",
            r#"---
title: Alpha Cluster 2
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Alpha
Links [[beta]] and [[gamma]].
"#,
        ),
        (
            "clusters/omega-rho.md",
            r#"---
title: Omega Rho
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Omega
Links [[omega]] and [[rho]].
"#,
        ),
        (
            "clusters/omega-neighborhood.md",
            r#"---
title: Omega Neighborhood
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Omega
Links [[omega]], [[rho]], [[sigma]], and [[tau]].
"#,
        ),
        (
            "clusters/lambda-neighborhood.md",
            r#"---
title: Lambda Neighborhood
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Lambda
Links [[lambda]], [[rho]], [[sigma]], and [[tau]].
"#,
        ),
    ] {
        write_markdown_file(&tmp, relative, content);
    }

    let sync_output = maenifold_cmd(&tmp)
        .args(["--tool", "Sync", "--payload", "{}", "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let sync_rendered = render_json_data_or_text(sync_output);
    assert!(
        sync_rendered.contains("COMMUNITIES detected"),
        "expected community summary after sync: {sync_rendered}"
    );

    let connection = Connection::open(tmp.path().join("memory.db")).expect("open db");
    let omega_community_id: i64 = connection
        .query_row(
            "SELECT community_id FROM concept_communities WHERE concept_name = 'omega'",
            [],
            |row| row.get(0),
        )
        .expect("omega community id");
    assert!(
        omega_community_id > 0,
        "expected omega to land outside community 0"
    );

    let build_output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "BuildContext",
            "--payload",
            r#"{"conceptName":"omega","depth":1}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let build_value: Value = serde_json::from_slice(&build_output).expect("json");
    let direct_relations = build_value["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations");
    assert!(
        direct_relations
            .iter()
            .any(|relation| relation["CommunityId"] == omega_community_id),
        "expected real community id in BuildContext output: {build_value}"
    );
    assert!(
        build_value["data"]["CommunitySiblings"]
            .as_array()
            .expect("siblings")
            .iter()
            .any(|sibling| sibling["Name"] == "lambda"),
        "expected sibling section in BuildContext output: {build_value}"
    );
}

// T-3.3: RTM FR-13.10
#[test]
fn build_context_cli_gracefully_degrades_when_community_table_is_empty() {
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();
    for (relative, content) in [
        (
            "clusters/omega-rho.md",
            r#"---
title: Omega Rho
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Omega
Links [[omega]] and [[rho]].
"#,
        ),
        (
            "clusters/omega-sigma.md",
            r#"---
title: Omega Sigma
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Omega
Links [[omega]] and [[sigma]].
"#,
        ),
    ] {
        write_markdown_file(&tmp, relative, content);
    }

    maenifold_cmd(&tmp)
        .args(["--tool", "Sync", "--payload", "{}"])
        .assert()
        .code(0);

    let connection = Connection::open(tmp.path().join("memory.db")).expect("open db");
    connection
        .execute("DELETE FROM concept_communities", [])
        .expect("clear concept_communities");

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "BuildContext",
            "--payload",
            r#"{"conceptName":"omega","depth":1}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    let direct_relations = value["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations");
    assert!(
        direct_relations
            .iter()
            .any(|relation| relation["Name"] == "rho"),
        "expected relation output to remain: {value}"
    );
    assert!(
        direct_relations
            .iter()
            .all(|relation| relation["CommunityId"].is_null()),
        "graceful degradation should omit community ids: {value}"
    );
    assert!(
        value["data"]["CommunitySiblings"]
            .as_array()
            .map(|siblings| siblings.is_empty())
            .unwrap_or(false),
        "sibling section should be omitted without persisted communities: {value}"
    );
}
