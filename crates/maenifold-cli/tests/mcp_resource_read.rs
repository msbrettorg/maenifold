// T-10.2: RTM FR-11
//! `ReadMcpResource` entry-point tests split from `mcp_resource_tests.rs`.

mod mcp_resource_support;

use mcp_resource_support::{invoke_err, invoke_ok, isolated_root, read_catalog};

#[test]
fn read_mcp_resource_missing_uri_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ReadMcpResource", "{}");

    assert_eq!(err["code"].as_str(), Some("INVALID_ARGUMENT"));
    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("uri"));
}

#[test]
fn read_mcp_resource_empty_uri_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ReadMcpResource", r#"{"uri":""}"#);

    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Invalid resource URI format"));
}

#[test]
fn read_mcp_resource_catalog_uri_returns_catalog_json() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    assert!(result.get("singleAgentWorkflows").is_some());
    assert!(result.get("multiAgentWorkflows").is_some());
    assert!(result.get("roles").is_some());
    assert!(result.get("colors").is_some());
    assert!(result.get("perspectives").is_some());
}

#[test]
fn read_mcp_resource_valid_workflow_uri_returns_workflow_json() {
    let tmp = isolated_root();
    let catalog = read_catalog(&tmp);

    let workflows = catalog["singleAgentWorkflows"]
        .as_array()
        .expect("singleAgentWorkflows");
    assert!(!workflows.is_empty());

    let workflow_id = workflows[0]["id"].as_str().expect("workflow id");
    let payload =
        serde_json::json!({"uri": format!("asset://workflows/{workflow_id}")}).to_string();
    let result = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert_eq!(result["id"].as_str(), Some(workflow_id));
}

#[test]
fn read_mcp_resource_valid_role_uri_returns_role_json() {
    let tmp = isolated_root();
    let catalog = read_catalog(&tmp);

    let roles = catalog["roles"].as_array().expect("roles");
    assert!(!roles.is_empty());

    let role_id = roles[0]["id"].as_str().expect("role id");
    let payload = serde_json::json!({"uri": format!("asset://roles/{role_id}")}).to_string();
    let result = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert!(result.get("id").is_some());
}

#[test]
fn read_mcp_resource_invalid_uri_format_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ReadMcpResource", r#"{"uri":"invalid-uri"}"#);

    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Invalid resource URI format"));
}

#[test]
fn read_mcp_resource_uri_missing_path_segment_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ReadMcpResource", r#"{"uri":"asset://workflows"}"#);

    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Invalid resource URI format"));
}

#[test]
fn read_mcp_resource_unknown_resource_type_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(
        &tmp,
        "ReadMcpResource",
        r#"{"uri":"asset://bogus/some-id"}"#,
    );

    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Unknown asset type"));
    assert!(msg.contains("bogus"));
}

#[test]
fn read_mcp_resource_nonexistent_workflow_id_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(
        &tmp,
        "ReadMcpResource",
        r#"{"uri":"asset://workflows/nonexistent-workflow-xyz-99999"}"#,
    );

    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Asset not found"));
}

#[test]
fn read_mcp_resource_nonexistent_role_id_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(
        &tmp,
        "ReadMcpResource",
        r#"{"uri":"asset://roles/nonexistent-role-xyz-99999"}"#,
    );

    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Asset not found"));
}
