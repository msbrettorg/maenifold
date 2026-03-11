// T-12.7: RTM NFR-1
#![allow(dead_code)]
//! Shared helpers for CLI output integration tests split from `cli_output_tests.rs`.

use assert_cmd::Command;
pub use serde_json::{Value, json};
use tempfile::TempDir;

const TEST_FOLDER: &str = "cli-json-tests";

pub fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

pub fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-CLI-JSON-001: RTM FR-8.1
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

pub fn run_output(
    tmp: &TempDir,
    tool: &str,
    payload: &Value,
    json_mode: bool,
    expected_code: i32,
) -> Vec<u8> {
    let mut cmd = maenifold_cmd(tmp);
    cmd.arg("--tool")
        .arg(tool)
        .arg("--payload")
        .arg(payload.to_string());
    if json_mode {
        cmd.arg("--json");
    }
    cmd.assert().code(expected_code).get_output().stdout.clone()
}

pub fn run_json(tmp: &TempDir, tool: &str, payload: &Value, expected_code: i32) -> Value {
    let output = run_output(tmp, tool, payload, true, expected_code);
    serde_json::from_slice(&output).expect("valid json")
}

pub fn run_text(
    tmp: &TempDir,
    tool: &str,
    payload: &Value,
    json_mode: bool,
    expected_code: i32,
) -> String {
    String::from_utf8(run_output(tmp, tool, payload, json_mode, expected_code)).expect("utf8")
}

pub fn write_memory_json(tmp: &TempDir, title: &str, content: &str) -> Value {
    // T-CLI-JSON-001: RTM FR-8.2
    run_json(
        tmp,
        "WriteMemory",
        &json!({
            "title": title,
            "content": content,
            "folder": TEST_FOLDER,
        }),
        0,
    )
}

pub fn sync_memory_graph(tmp: &TempDir) {
    let _ = run_text(tmp, "Sync", &json!({}), true, 0);
}

pub fn assert_no_ansi_escape_sequences(text: &str) {
    // T-CLI-JSON-001: RTM FR-8.2
    assert!(
        !text.contains("\u{1b}[") && !text.contains("\x1b["),
        "output should not contain ANSI escape codes: {text}"
    );
}
