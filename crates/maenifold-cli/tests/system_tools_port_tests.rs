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
