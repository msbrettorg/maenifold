use crate::mcp::gateway_catalog::CommandEntry;
use rmcp::model::Tool;
use serde_json::Value;

pub(crate) fn strip_schemas(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("schema");
            for child in map.values_mut() {
                strip_schemas(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_schemas(item);
            }
        }
        _ => {}
    }
}

pub(crate) fn command_entry(tool: &Tool, safety_class: String) -> CommandEntry {
    let name = tool.name.to_string();
    CommandEntry {
        name: name.clone(),
        description: tool.description.as_deref().unwrap_or("").to_string(),
        schema: Value::Object((*tool.input_schema).clone()),
        safety_class,
    }
}
