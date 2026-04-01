#![allow(dead_code)]

// T-12.0: RTM NFR-1 - share CLI security fixtures across themed integration test files while preserving coverage.
use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

pub(crate) fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

pub(crate) fn maenifold_cmd(tmp: &TempDir) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

pub(crate) fn run_tool_bytes(
    tmp: &TempDir,
    tool: &str,
    payload: &Value,
    json_mode: bool,
    expected_code: i32,
) -> Vec<u8> {
    let payload = serde_json::to_string(payload).expect("payload json");
    let mut cmd = maenifold_cmd(tmp);
    let mut args = vec!["tool".to_string(), tool.to_string(), payload];
    if json_mode {
        args.push("--json".to_string());
    }

    cmd.args(&args)
        .assert()
        .code(expected_code)
        .get_output()
        .stdout
        .clone()
}

pub(crate) fn run_tool_json(
    tmp: &TempDir,
    tool: &str,
    payload: &Value,
    expected_code: i32,
) -> Value {
    serde_json::from_slice(&run_tool_bytes(tmp, tool, payload, true, expected_code))
        .expect("valid json")
}

pub(crate) fn run_tool_text(
    tmp: &TempDir,
    tool: &str,
    payload: &Value,
    expected_code: i32,
) -> String {
    String::from_utf8(run_tool_bytes(tmp, tool, payload, false, expected_code)).expect("utf8")
}

pub(crate) fn create_sequential_session(tmp: &TempDir) -> String {
    let value = run_tool_json(
        tmp,
        "SequentialThinking",
        &json!({
            "response": "Starting [[cli-json-security]] session.",
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "nextThoughtNeeded": true
        }),
        0,
    );
    value["data"]["sessionId"]
        .as_str()
        .expect("sessionId")
        .to_string()
}

pub(crate) fn assert_envelope_shape(value: &Value) {
    assert!(value.get("success").is_some(), "missing success: {value}");
    assert!(value.get("data").is_some(), "missing data: {value}");
    assert!(value.get("error").is_some(), "missing error: {value}");
}

pub(crate) fn create_deep_object(depth: usize) -> Value {
    let mut value = json!("leaf");
    for _ in 0..depth {
        value = json!({ "nested": value });
    }
    value
}
