// T-10.2: RTM FR-11
//! MCP resource lookup and round-trip tests split from `mcp_resource_tests.rs`.

mod mcp_resource_support;

use mcp_resource_support::{invoke_err, invoke_ok, isolated_root, read_catalog};

#[test]
fn get_workflow_with_valid_id_returns_json_with_id_and_name_fields() {
    let tmp = isolated_root();
    let catalog = read_catalog(&tmp);

    let workflows = catalog["singleAgentWorkflows"]
        .as_array()
        .expect("singleAgentWorkflows");
    let first_id = workflows[0]["id"].as_str().expect("workflow id");

    let payload = serde_json::json!({"uri": format!("asset://workflows/{first_id}")}).to_string();
    let workflow = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert_eq!(workflow["id"].as_str(), Some(first_id));
    let name = workflow["name"].as_str().expect("workflow name");
    assert!(!name.is_empty());
}

#[test]
fn get_role_with_valid_id_returns_valid_json() {
    let tmp = isolated_root();
    let catalog = read_catalog(&tmp);

    let roles = catalog["roles"].as_array().expect("roles");
    let first_id = roles[0]["id"].as_str().expect("role id");

    let payload = serde_json::json!({"uri": format!("asset://roles/{first_id}")}).to_string();
    let role = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert_eq!(role["id"].as_str(), Some(first_id));
}

#[test]
fn get_workflow_with_nonexistent_id_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(
        &tmp,
        "ReadMcpResource",
        r#"{"uri":"asset://workflows/non-existent-workflow-xyz"}"#,
    );

    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Asset not found"));
    assert!(msg.contains("non-existent-workflow-xyz"));
}

#[test]
fn get_role_with_nonexistent_id_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(
        &tmp,
        "ReadMcpResource",
        r#"{"uri":"asset://roles/non-existent-role-xyz"}"#,
    );

    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Asset not found"));
}

#[test]
fn asset_discovery_and_retrieval_flow_is_consistent() {
    let tmp = isolated_root();

    let workflows = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflow"}"#);
    let workflow_items = workflows.as_array().expect("workflows array");
    assert!(!workflow_items.is_empty());

    for workflow in workflow_items {
        let id = workflow["id"].as_str().expect("workflow id");
        let payload = serde_json::json!({"uri": format!("asset://workflows/{id}")}).to_string();
        let fetched = invoke_ok(&tmp, "ReadMcpResource", &payload);
        assert!(fetched.get("id").is_some());
    }

    let roles = invoke_ok(&tmp, "ListAssets", r#"{"type":"role"}"#);
    let role_items = roles.as_array().expect("roles array");

    for role in role_items {
        let id = role["id"].as_str().expect("role id");
        let payload = serde_json::json!({"uri": format!("asset://roles/{id}")}).to_string();
        let fetched = invoke_ok(&tmp, "ReadMcpResource", &payload);
        assert!(fetched.get("id").is_some());
    }
}
