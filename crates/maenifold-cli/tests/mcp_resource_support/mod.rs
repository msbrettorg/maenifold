// T-10.2: RTM FR-11
#![allow(dead_code)]
//! Shared helpers for MCP resource CLI integration tests.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

pub fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

pub fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-10.2: RTM FR-11
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

pub fn invoke_json(tmp: &TempDir, tool: &str, payload: &str) -> (Value, i32) {
    let output = maenifold_cmd(tmp)
        .args(["--tool", tool, "--payload", payload, "--json"])
        .output()
        .expect("run maenifold");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("expected JSON output, got: {stdout}"));
    (value, code)
}

pub fn invoke_ok(tmp: &TempDir, tool: &str, payload: &str) -> Value {
    let (envelope, code) = invoke_json(tmp, tool, payload);
    assert_eq!(
        code, 0,
        "expected exit code 0, got {code}; envelope = {envelope}"
    );
    assert_eq!(
        envelope["success"], true,
        "expected success=true; envelope = {envelope}"
    );
    let data_str = envelope["data"]
        .as_str()
        .unwrap_or_else(|| panic!("data should be a string; envelope = {envelope}"));
    serde_json::from_str(data_str)
        .unwrap_or_else(|_| panic!("data should be valid JSON; raw = {data_str}"))
}

pub fn invoke_err(tmp: &TempDir, tool: &str, payload: &str) -> Value {
    let (envelope, code) = invoke_json(tmp, tool, payload);
    assert_ne!(
        code, 0,
        "expected non-zero exit code; envelope = {envelope}"
    );
    assert_eq!(
        envelope["success"], false,
        "expected success=false; envelope = {envelope}"
    );
    envelope["error"].clone()
}

pub fn read_catalog(tmp: &TempDir) -> Value {
    invoke_ok(tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#)
}
