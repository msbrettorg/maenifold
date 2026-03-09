use assert_cmd::Command;
use tempfile::TempDir;

use std::fs;

// T-1.5: RTM FR-1 / FR-3 / FR-4 / FR-5 / FR-6 ports the C# SystemToolsTests coverage to the Rust CLI.

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd(tmp: &TempDir) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

fn run_tool_text(tmp: &TempDir, tool: &str, payload: &str) -> String {
    let output = maenifold_cmd(tmp)
        .args(["--tool", tool, "--payload", payload])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).expect("utf8")
}

fn write_markdown_file(tmp: &TempDir, relative: &str, content: &str) {
    let path = tmp.path().join("memory").join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write markdown");
}

#[test]
fn get_config_returns_expected_structure() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "GetConfig", "{}");

    assert!(!result.is_empty(), "config result should not be empty");
    assert!(
        result.contains("Memory") || result.contains("Maenifold"),
        "config should contain memory or maenifold path information: {result}"
    );
    assert!(
        result.contains("Database") || result.contains("memory.db"),
        "config should contain database path information: {result}"
    );
}

#[test]
fn get_config_with_learn_true_returns_help_documentation() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "GetConfig", r#"{"learn":true}"#);

    let is_help_doc = result.contains("GetConfig") || result.contains("configuration");
    let is_error_message = result.contains("ERROR") && result.contains("Help file not found");

    assert!(
        is_help_doc || is_error_message,
        "result should be help documentation or a missing-help error: {result}"
    );
}

#[test]
fn memory_status_returns_valid_statistics() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "MemoryStatus", "{}");

    assert!(!result.is_empty(), "status result should not be empty");
    assert!(
        result.contains("fileCount") || result.contains("Files") || result.contains("files"),
        "status should include file statistics: {result}"
    );
}

#[test]
fn memory_status_includes_database_metrics() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "MemoryStatus", "{}");

    assert!(
        result.contains("databasePath") || result.contains("Database"),
        "status should contain database information: {result}"
    );
    assert!(
        result.contains("memory.db") || result.contains("not initialized"),
        "status should either reference the database path or indicate initialization state: {result}"
    );
}

#[test]
fn memory_status_with_learn_true_returns_help_documentation() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "MemoryStatus", r#"{"learn":true}"#);

    let is_help_doc = result.contains("MemoryStatus")
        || result.contains("statistics")
        || result.contains("status");
    let is_error_message = result.contains("ERROR") && result.contains("Help file not found");

    assert!(
        is_help_doc || is_error_message,
        "result should be help documentation or a missing-help error: {result}"
    );
}

#[test]
fn list_memories_with_no_path_returns_root_structure() {
    let tmp = isolated_root();
    write_markdown_file(
        &tmp,
        "test-list-memories.md",
        "# Test\n\nTest content for listing",
    );

    let result = run_tool_text(&tmp, "ListMemories", "{}");

    assert!(!result.is_empty(), "list result should not be empty");
    assert!(
        result.contains("# Directory: /") || result.contains("Directory"),
        "list should contain a directory header: {result}"
    );
}

#[test]
fn list_memories_with_subdirectory_path_returns_subdirectory_contents() {
    let tmp = isolated_root();
    write_markdown_file(
        &tmp,
        "test-subdir-list/test-file.md",
        "# Test\n\nTest content",
    );

    let result = run_tool_text(&tmp, "ListMemories", r#"{"path":"test-subdir-list"}"#);

    assert!(
        result.contains("test-subdir-list"),
        "list should reference the subdirectory: {result}"
    );
    assert!(
        result.contains("test-file.md"),
        "list should contain the test file: {result}"
    );
}

#[test]
fn list_memories_with_non_existent_path_returns_error_message() {
    let tmp = isolated_root();

    let result = run_tool_text(
        &tmp,
        "ListMemories",
        r#"{"path":"non-existent-directory-xyz"}"#,
    );

    assert!(
        result.to_ascii_lowercase().contains("not found"),
        "result should indicate the directory was not found: {result}"
    );
}

#[test]
fn list_memories_with_learn_true_returns_help_documentation() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "ListMemories", r#"{"learn":true}"#);

    let is_help_doc = result.contains("ListMemories")
        || result.contains("directory")
        || result.contains("folder");
    let is_error_message = result.contains("ERROR") && result.contains("Help file not found");

    assert!(
        is_help_doc || is_error_message,
        "result should be help documentation or a missing-help error: {result}"
    );
}

#[test]
fn get_help_with_valid_tool_name_returns_documentation() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "GetHelp", r#"{"toolName":"writememory"}"#);

    let is_help_content =
        result.contains("WriteMemory") || result.contains("memory") || result.contains('#');
    let is_tool_list = result.contains("Available tools") && result.contains("writememory");

    assert!(
        is_help_content || is_tool_list,
        "result should be either help documentation or the available tool list: {result}"
    );
}

#[test]
fn get_help_with_invalid_tool_name_returns_error_with_available_tools() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "GetHelp", r#"{"toolName":"non-existent-tool-xyz"}"#);

    assert!(
        result.contains("No help file found") || result.contains("not found"),
        "result should indicate the help file was not found: {result}"
    );
    assert!(
        result.contains("Available tools"),
        "result should list available tools: {result}"
    );
}

#[test]
fn get_help_is_case_insensitive() {
    let tmp = isolated_root();

    let lower_result = run_tool_text(&tmp, "GetHelp", r#"{"toolName":"writememory"}"#);
    let upper_result = run_tool_text(&tmp, "GetHelp", r#"{"toolName":"WRITEMEMORY"}"#);
    let mixed_result = run_tool_text(&tmp, "GetHelp", r#"{"toolName":"WriteMemory"}"#);

    let all_have_help = !lower_result.contains("Available tools")
        && !upper_result.contains("Available tools")
        && !mixed_result.contains("Available tools");
    let all_list_tools = lower_result.contains("Available tools")
        && upper_result.contains("Available tools")
        && mixed_result.contains("Available tools");

    assert!(
        all_have_help || all_list_tools,
        "case variations should return consistent results:\nlower={lower_result}\nupper={upper_result}\nmixed={mixed_result}"
    );
}

#[test]
fn update_assets_with_dry_run_true_returns_preview_without_modifying() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "UpdateAssets", r#"{"dryRun":true}"#);

    let is_dry_run_result = result.contains("DRY RUN")
        || result.contains("dry run")
        || result.contains("Preview")
        || result.contains("would")
        || result.contains("Summary");

    assert!(
        is_dry_run_result,
        "result should indicate dry-run mode or show a preview: {result}"
    );
}

#[test]
fn update_assets_with_learn_true_returns_help_documentation() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "UpdateAssets", r#"{"learn":true}"#);

    let is_help_doc =
        result.contains("UpdateAssets") || result.contains("assets") || result.contains("upgrade");
    let is_error_message = result.contains("ERROR") && result.contains("Help file not found");

    assert!(
        is_help_doc || is_error_message,
        "result should be help documentation or a missing-help error: {result}"
    );
}

#[test]
fn update_assets_defaults_to_dry_run() {
    let tmp = isolated_root();

    let result = run_tool_text(&tmp, "UpdateAssets", "{}");

    let is_dry_run_result = result.contains("DRY RUN")
        || result.contains("dry run")
        || result.contains("Preview")
        || result.contains("would");
    let shows_summary_only = result.contains("Summary") && !result.contains("Updated:");

    assert!(
        is_dry_run_result || shows_summary_only,
        "default behavior should stay in safe preview mode: {result}"
    );
}
