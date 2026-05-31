use super::adapter_env::validate_env;
use serde_json::{Value, json};
use std::collections::HashMap;

// T-301.31 test design: validate_env is the new fail-closed contract for
// envFromFile/envFromCommand. Missing or empty sources are errors by default;
// explicitly optional envFromFile entries use { "path": "...", "optional": true }.
// Fixture pattern follows tempfile TempDir docs: create files under TempDir::path().

fn resolved_map(entry: &Value) -> HashMap<String, String> {
    validate_env(entry)
        .expect("validate env")
        .into_iter()
        .collect()
}

fn single_failure(entry: &Value) -> String {
    let err = validate_env(entry).expect_err("validate_env should fail");
    assert_eq!(err.failures.len(), 1, "expected one structured failure");
    let failure = &err.failures[0];
    format!("{}|{}|{}", failure.var_name, failure.source, failure.reason)
}

#[test]
fn validate_env_t301_31_passes_when_env_from_file_resolves() {
    let root = tempfile::tempdir().expect("create temp dir");
    let secret_path = root.path().join("test-var");
    std::fs::write(&secret_path, "from-file\n").expect("write secret file");
    let entry = json!({
        "envFromFile": { "TEST_VAR": secret_path.to_string_lossy() }
    });

    let env = resolved_map(&entry);

    assert_eq!(env.get("TEST_VAR"), Some(&"from-file".to_string()));
}

#[test]
fn validate_env_t301_31_fails_when_env_from_file_file_missing() {
    let missing = "/nonexistent/t301-31-missing-token";
    let entry = json!({ "envFromFile": { "TEST_VAR": missing } });

    let failure = single_failure(&entry);

    assert!(failure.contains("TEST_VAR"), "{failure}");
    assert!(failure.contains(missing), "{failure}");
    assert!(
        failure.contains("not found") || failure.contains("No such file"),
        "{failure}"
    );
}

#[test]
fn validate_env_t301_31_fails_when_env_from_file_file_empty() {
    let root = tempfile::tempdir().expect("create temp dir");
    let secret_path = root.path().join("empty-token");
    std::fs::write(&secret_path, "").expect("write empty secret file");
    let entry = json!({
        "envFromFile": { "TEST_VAR": secret_path.to_string_lossy() }
    });

    let failure = single_failure(&entry);

    assert!(failure.contains("TEST_VAR"), "{failure}");
    assert!(
        failure.contains(&secret_path.to_string_lossy().to_string()),
        "{failure}"
    );
    assert!(failure.contains("empty"), "{failure}");
}

#[test]
fn validate_env_t301_31_fails_when_env_from_command_command_fails() {
    let entry = json!({ "envFromCommand": { "TOKEN": "false" } });

    let failure = single_failure(&entry);

    assert!(failure.contains("TOKEN"), "{failure}");
    assert!(failure.contains("false"), "{failure}");
    assert!(
        failure.contains("exit") || failure.contains("status") || failure.contains("failed"),
        "{failure}"
    );
}

#[test]
fn validate_env_t301_31_passes_when_env_from_command_command_succeeds() {
    let echo = crate::test_util::fake_echo_binary();
    let entry = json!({
        "envFromCommand": { "TOKEN": format!("{} my-token", echo.display()) }
    });

    let env = resolved_map(&entry);

    assert_eq!(env.get("TOKEN"), Some(&"my-token".to_string()));
}

#[test]
fn validate_env_t301_31_passes_when_no_env_from_file_or_env_from_command() {
    let entry = json!({ "command": "/bin/echo adapter" });

    let env = resolved_map(&entry);

    assert!(env.is_empty(), "no declared env should add no variables");
}

#[test]
fn validate_env_t301_31_passes_when_only_env_from_file_with_optional_true_and_missing() {
    let entry = json!({
        "envFromFile": {
            "TEST_VAR": { "path": "/nonexistent/t301-31-optional", "optional": true }
        }
    });

    let env = resolved_map(&entry);

    assert!(
        env.is_empty(),
        "optional missing vars are skipped explicitly"
    );
}
