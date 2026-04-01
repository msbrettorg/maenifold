use crate::mcp::McpToolDefinition;
use schemars::{JsonSchema, schema::RootSchema};
use serde_json::Value;

pub(super) fn tool_definition(
    name: &str,
    description: &str,
    input_schema: Value,
) -> McpToolDefinition {
    McpToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema,
    }
}

pub(super) fn derived_tool_definition<T: JsonSchema>(
    name: &str,
    description: &str,
) -> McpToolDefinition {
    tool_definition(name, description, schema_value::<T>())
}

pub(super) fn schema_value<T: JsonSchema>() -> Value {
    root_schema_to_value(schemars::schema_for!(T))
}

pub(super) fn root_schema_to_value(schema: RootSchema) -> Value {
    serde_json::to_value(schema).expect("schemars root schema should serialize")
}
