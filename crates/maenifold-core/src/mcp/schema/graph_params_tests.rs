use super::mcp_tool_from_registry_name;
use serial_test::serial;

fn find_enum(value: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    if let Some(values) = value.get("enum").and_then(serde_json::Value::as_array) {
        return Some(values);
    }

    match value {
        serde_json::Value::Array(values) => values.iter().find_map(find_enum),
        serde_json::Value::Object(map) => map.values().find_map(find_enum),
        _ => None,
    }
}

#[test]
#[serial]
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
    let values = format.values().find_map(find_enum).expect("format enum");
    assert_eq!(
        values,
        &vec![serde_json::json!("mermaid"), serde_json::json!("graph")]
    );
}
