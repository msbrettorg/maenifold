use crate::mcp::McpToolDefinition;
use serde_json::{Map, Value, json};

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

pub(super) fn object_schema(properties: Vec<(&str, Value)>, required: &[&str]) -> Value {
    let mut property_map = Map::new();
    for (name, schema) in properties {
        property_map.insert(name.to_string(), schema);
    }

    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), Value::Object(property_map));
    schema.insert("additionalProperties".to_string(), Value::Bool(false));
    if !required.is_empty() {
        schema.insert(
            "required".to_string(),
            Value::Array(required.iter().map(|name| json!(name)).collect()),
        );
    }
    Value::Object(schema)
}

pub(super) fn string_property(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

pub(super) fn string_property_with_pattern(description: &str, pattern: &str) -> Value {
    json!({ "type": "string", "description": description, "pattern": pattern })
}

pub(super) fn integer_property(description: &str) -> Value {
    json!({ "type": "integer", "description": description })
}

pub(super) fn integer_property_with_default(description: &str, default: Value) -> Value {
    json!({ "type": "integer", "description": description, "default": default })
}

pub(super) fn number_property_with_default(description: &str, default: Value) -> Value {
    json!({ "type": "number", "description": description, "default": default })
}

pub(super) fn bool_property(description: &str, default: bool) -> Value {
    json!({ "type": "boolean", "description": description, "default": default })
}

pub(super) fn string_array_property(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": { "type": "string" }
    })
}

pub(super) fn string_or_string_array_property(description: &str) -> Value {
    json!({
        "description": description,
        "anyOf": [
            { "type": "string" },
            {
                "type": "array",
                "items": { "type": "string" }
            }
        ]
    })
}

pub(super) fn enum_string_property(description: &str, variants: &[&str]) -> SchemaProperty {
    SchemaProperty(json!({
        "type": "string",
        "description": description,
        "enum": variants,
    }))
}

pub(super) struct SchemaProperty(Value);

impl SchemaProperty {
    pub(super) fn with_default(mut self, default: Value) -> Value {
        if let Value::Object(ref mut object) = self.0 {
            object.insert("default".to_string(), default);
        }
        self.0
    }
}

impl From<SchemaProperty> for Value {
    fn from(property: SchemaProperty) -> Self {
        property.0
    }
}
