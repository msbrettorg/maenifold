use super::mcp_tool_from_registry_name;

#[test]
fn unknown_registry_tool_uses_generic_gateway_schema() {
    let tool = mcp_tool_from_registry_name("GatewayPassthrough").expect("generic gateway schema");

    assert_eq!(tool.name, "GatewayPassthrough");
    assert_eq!(
        tool.description,
        "Gateway tool (use learn=true for details)"
    );
    assert_eq!(tool.input_schema, serde_json::json!({ "type": "object" }));
}
