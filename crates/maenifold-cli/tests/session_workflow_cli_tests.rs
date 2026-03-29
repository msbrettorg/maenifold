use assert_cmd::Command;
use chrono::Datelike;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn run_json(tmp: &TempDir, args: &[&str]) -> Value {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    let output = cmd
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(args)
        .assert()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("json output")
}

fn run_text(tmp: &TempDir, args: &[&str]) -> String {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    let output = cmd
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(args)
        .assert()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("utf8");
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::String(rendered)) => rendered,
        Ok(value) => value
            .get("data")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or(text),
        Err(_) => text,
    }
}

#[test]
fn sequential_thinking_invalid_payload_returns_cli_json_error_envelope() {
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        &["tool", "SequentialThinking", "{", "--json"],
    );
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "INVALID_PAYLOAD");
}

#[test]
fn workflow_invalid_payload_returns_cli_json_error_envelope() {
    let tmp = isolated_root();
    let value = run_json(&tmp, &["tool", "Workflow", "{", "--json"]);
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "INVALID_PAYLOAD");
}

#[test]
fn assumption_ledger_invalid_payload_returns_cli_json_error_envelope() {
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        &["tool", "AssumptionLedger", "{", "--json"],
    );
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "INVALID_PAYLOAD");
}

#[test]
fn sequential_thinking_json_success_should_include_session_id_and_next_thought() {
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        &[
            "tool",
            "SequentialThinking",
            r#"{"response":"Exploring [[session-paths]] and [[workflow-state]].","thoughtNumber":0,"totalThoughts":2,"nextThoughtNeeded":true}"#,
            "--json",
        ],
    );
    assert_eq!(value["success"], true);
    assert!(
        value["data"]["sessionId"]
            .as_str()
            .unwrap_or_default()
            .starts_with("session-")
    );
    assert_eq!(value["data"]["thoughtNumber"], 0);
    assert_eq!(value["data"]["totalThoughts"], 2);
}

#[test]
fn workflow_output_should_include_session_id_and_step_state() {
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        &[
            "tool",
            "Workflow",
            r#"{"workflowId":"deductive-reasoning"}"#,
            "--json",
        ],
    );
    let session_id = text
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("Created session: "))
        .expect("session line");
    assert!(session_id.starts_with("workflow-"));
    assert!(text.contains("Step 1/"));
    let path = tmp.path().join("memory").join("thinking").join("workflow");
    assert!(fs::read_dir(path).is_ok());
}

#[test]
fn assumption_ledger_append_output_should_return_memory_uri() {
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        &[
            "tool",
            "AssumptionLedger",
            r#"{"action":"append","assumption":"Track [[session-contract]] parity","concepts":["session-contract"],"confidence":"medium"}"#,
            "--json",
        ],
    );
    assert!(text.contains("memory://assumptions/"));
    assert!(text.contains("Track [[session-contract]] parity"));
}

#[test]
fn assumption_ledger_append_uses_date_based_memory_uri_path() {
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        &[
            "tool",
            "AssumptionLedger",
            r#"{"action":"append","assumption":"Track [[session-contract]] parity","concepts":["session-contract"],"confidence":"medium"}"#,
            "--json",
        ],
    );

    let now = chrono::Utc::now();
    let expected_prefix = format!(
        "memory://assumptions/{:04}/{:02}/assumption-",
        now.year(),
        now.month()
    );
    assert!(text.contains(&expected_prefix), "unexpected output: {text}");
}

#[test]
fn sequential_thinking_without_wikilinks_should_return_csharp_validation_error() {
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        &[
            "tool",
            "SequentialThinking",
            r#"{"response":"This thought has no concepts at all.","thoughtNumber":0,"totalThoughts":1,"nextThoughtNeeded":true}"#,
            "--json",
        ],
    );
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "WIKILINK_REQUIRED");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Must include [[WikiLinks]]")
    );
}

#[test]
fn workflow_learn_mode_should_return_help_without_creating_session() {
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        &["tool", "Workflow", r#"{"learn":true}"#],
    );
    assert!(text.contains("# Workflow"), "unexpected output: {text}");
    assert!(
        !text.contains("Created session:"),
        "learn mode should not create sessions"
    );
    assert!(
        !text.contains("Cannot provide both"),
        "learn mode should bypass parameter validation"
    );
}

#[test]
fn workflow_with_both_workflow_and_session_ids_should_return_cli_json_error() {
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        &[
            "tool",
            "Workflow",
            r#"{"workflowId":"deductive-reasoning","sessionId":"workflow-123"}"#,
            "--json",
        ],
    );
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "INTERNAL_ERROR");
    let message = value["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("Cannot provide both workflowId and sessionId together"));
    assert!(message.contains("workflow-123"));
    assert!(message.contains("deductive-reasoning"));
}

#[test]
fn assumption_ledger_append_requires_concepts() {
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        &[
            "tool",
            "AssumptionLedger",
            r#"{"action":"append","assumption":"hello"}"#,
            "--json",
        ],
    );
    assert!(text.starts_with("ERROR:"), "unexpected output: {text}");
    assert!(text.contains("At least one [[WikiLink]] tag must be provided"));
}

#[test]
fn assumption_ledger_append_rejects_concepts_with_brackets() {
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        &[
            "tool",
            "AssumptionLedger",
            r#"{"action":"append","assumption":"hello","concepts":["[[bad]]"]}"#,
            "--json",
        ],
    );
    assert!(text.starts_with("ERROR:"), "unexpected output: {text}");
    assert!(text.contains("Concept tags should not include brackets"));
}

#[test]
fn assumption_ledger_update_rejects_invalid_status() {
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        &[
            "tool",
            "AssumptionLedger",
            r#"{"action":"update","uri":"memory://assumptions/2026/02/assumption-abc123","status":"bogus"}"#,
            "--json",
        ],
    );
    assert!(text.starts_with("ERROR:"), "unexpected output: {text}");
    assert!(text.contains("Invalid status. Use one of: active, validated, invalidated, refined"));
}
