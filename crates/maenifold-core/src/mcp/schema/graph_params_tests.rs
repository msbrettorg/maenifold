use super::mcp_tool_from_registry_name;

#[test]
fn visualize_schema_exposes_format_enum_for_tools_list() {
    let visualize = mcp_tool_from_registry_name("Visualize").expect("visualize schema");
    let properties = visualize
        .input_schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .expect("input schema properties");
    let format = properties
        .get("format")
        .and_then(serde_json::Value::as_object)
        .expect("format schema");
    let values = format
        .get("enum")
        .and_then(serde_json::Value::as_array)
        .expect("format enum");
    assert_eq!(
        values,
        &vec![serde_json::json!("mermaid"), serde_json::json!("graph")]
    );
}
