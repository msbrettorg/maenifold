use assert_cmd::Command;
use predicates::str::contains;
use serde_json::Value;
use tempfile::TempDir;

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd() -> Command {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    Command::new(assert_cmd::cargo::cargo_bin!("maenifold"))
}

#[test]
fn unknown_tool_returns_json_error_envelope() {
    let tmp = isolated_root();
    let mut cmd = maenifold_cmd();
    let output = cmd
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(["--tool", "Nope", "--payload", "{}", "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v["success"], false);
    assert_eq!(v["error"]["code"], "UNKNOWN_TOOL");
}

#[test]
fn invalid_payload_returns_json_error_envelope() {
    let tmp = isolated_root();
    let mut cmd = maenifold_cmd();
    let output = cmd
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(["--tool", "ListAssets", "--payload", "{", "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v["success"], false);
    assert_eq!(v["error"]["code"], "INVALID_PAYLOAD");
}

#[test]
fn list_assets_returns_json_array() {
    let tmp = isolated_root();
    let mut cmd = maenifold_cmd();
    let output = cmd
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(["--tool", "ListAssets", "--payload", "{}", "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(v["success"], true);
    let rendered = v["data"].as_str().expect("string data");
    let parsed: Value = serde_json::from_str(rendered).expect("embedded json");
    assert_eq!(parsed.as_array().map(|a| a.len()), Some(4));
}

#[test]
fn usage_is_printed_without_tool_args() {
    let tmp = isolated_root();
    maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .assert()
        // T-12.7: RTM NFR-1 - bare CLI invocation must keep the established C#-parity failure contract.
        .code(1)
        .stdout(contains("maenifold CLI"));
}

// PARITY-CLI-PAYLOAD: RTM NFR-1 — when --tool is present but --payload is
// missing, the CLI prints usage and exits 1 (C# parity).
#[test]
fn missing_payload_prints_usage_and_exits_1() {
    let tmp = isolated_root();
    maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(["--tool", "GetConfig", "--json"])
        .assert()
        .code(1)
        .stdout(contains("maenifold CLI"));
}
