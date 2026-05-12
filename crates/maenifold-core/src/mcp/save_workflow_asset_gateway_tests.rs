use super::domain_mapping::{SafetyClass, tool_domain, tool_safety};
use super::schema::mcp_tool_from_registry_name;

#[test]
fn save_workflow_asset_routes_to_workflow_domain() {
    assert_eq!(
        tool_domain("SaveWorkflowAsset"),
        Some("maenifold.workflow"),
        "SaveWorkflowAsset must be routed to maenifold.workflow"
    );
}

#[test]
fn save_workflow_asset_has_mutate_safety_class() {
    assert_eq!(
        tool_safety("SaveWorkflowAsset"),
        SafetyClass::Mutate,
        "SaveWorkflowAsset writes a file; must be Mutate, not Destructive or Read"
    );
}

#[test]
fn save_workflow_asset_has_id_and_content_in_schema() {
    let tool = mcp_tool_from_registry_name("SaveWorkflowAsset")
        .expect("SaveWorkflowAsset must have a native MCP schema");
    let props = tool
        .input_schema
        .get("properties")
        .and_then(|v| v.as_object())
        .expect("schema must have a properties object");
    assert!(props.contains_key("id"), "schema must include 'id' field");
    assert!(
        props.contains_key("content"),
        "schema must include 'content' field"
    );
}
