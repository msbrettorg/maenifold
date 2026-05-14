use super::*;
use crate::config::Config;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::json;
use serial_test::serial;
use std::fs;

// ── ID validation ──────────────────────────────────────────────────────

#[test]
fn valid_ids_are_accepted() {
    for id in ["a", "my-workflow", "workflow-v2", "abc123", "a1-b2-c3"] {
        assert!(validate_workflow_id(id).is_ok(), "should accept '{id}'");
    }
}

#[test]
fn invalid_ids_are_rejected() {
    for id in [
        "",
        "MyWorkflow",       // uppercase
        "my_workflow",      // underscore
        "my workflow",      // space
        "-leading",         // leading dash
        "trailing-",        // trailing dash
        "double--dash",     // consecutive dashes
        "../../etc/passwd", // path traversal
        "a/b",              // forward slash
        r"a\b",             // backslash
        "workflow.json",    // dot
    ] {
        assert!(validate_workflow_id(id).is_err(), "should reject '{id}'");
    }
}

// ── Content validation ─────────────────────────────────────────────────

#[test]
fn valid_content_is_accepted() {
    let content = r#"{"id":"my-flow","nodes":[]}"#;
    assert!(validate_workflow_content("my-flow", content).is_ok());
}

#[test]
fn non_json_content_is_rejected() {
    assert!(validate_workflow_content("x", "not json").is_err());
}

#[test]
fn json_array_content_is_rejected() {
    assert!(validate_workflow_content("x", r#"[{"id":"x"}]"#).is_err());
}

#[test]
fn missing_id_field_is_rejected() {
    assert!(validate_workflow_content("x", r#"{"name":"x"}"#).is_err());
}

#[test]
fn mismatched_id_field_is_rejected() {
    let content = r#"{"id":"other","nodes":[]}"#;
    assert!(validate_workflow_content("my-flow", content).is_err());
}

#[test]
fn content_exceeding_512kb_is_rejected() {
    let inner = "x".repeat(MAX_WORKFLOW_ASSET_BYTES + 1);
    let content = format!(r#"{{"id":"x","data":"{inner}"}}"#);
    assert!(validate_workflow_content("x", &content).is_err());
}

// ── Integration: full tool invocation ─────────────────────────────────

fn make_context(root: &std::path::Path) -> ToolContext {
    Config::override_root(root);
    let config = Config::current();
    Config::reset_overrides();
    ToolContext::new(config, OutputFormat::Json)
}

#[test]
#[serial]
fn save_workflow_asset_writes_file_and_returns_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = make_context(dir.path());
    let payload = json!({
        "id": "test-flow",
        "content": r#"{"id":"test-flow","nodes":[]}"#
    });

    let result = save_workflow_asset(&ctx, &payload).expect("save should succeed");

    let parsed: serde_json::Value = serde_json::from_str(&result).expect("response is JSON");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["data"]["id"], "test-flow");

    let expected = dir
        .path()
        .join("assets")
        .join("workflows")
        .join("test-flow.json");
    assert!(expected.exists(), "workflow file should be on disk");
    let on_disk = fs::read_to_string(&expected).expect("read file");
    let on_disk_json: serde_json::Value =
        serde_json::from_str(&on_disk).expect("file is valid JSON");
    assert_eq!(on_disk_json["id"], "test-flow");
}

#[test]
#[serial]
fn save_workflow_asset_overwrites_existing_file_atomically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = make_context(dir.path());

    for version in ["v1", "v2"] {
        let content = format!(r#"{{"id":"my-flow","version":"{version}"}}"#);
        let payload = json!({ "id": "my-flow", "content": content });
        save_workflow_asset(&ctx, &payload).expect("save should succeed");
    }

    let path = dir
        .path()
        .join("assets")
        .join("workflows")
        .join("my-flow.json");
    let on_disk: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read file")).expect("valid JSON");
    assert_eq!(on_disk["version"], "v2", "file should contain v2");
}

#[test]
#[serial]
fn save_workflow_asset_rejects_invalid_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = make_context(dir.path());
    let payload = json!({
        "id": "../../etc/passwd",
        "content": r#"{"id":"../../etc/passwd"}"#
    });
    assert!(save_workflow_asset(&ctx, &payload).is_err());
}

#[test]
#[serial]
fn save_workflow_asset_rejects_id_mismatch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = make_context(dir.path());
    let payload = json!({
        "id": "my-flow",
        "content": r#"{"id":"other-flow"}"#
    });
    assert!(save_workflow_asset(&ctx, &payload).is_err());
}
