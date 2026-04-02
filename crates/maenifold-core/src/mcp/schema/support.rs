use std::collections::HashSet;

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

/// Generates a JSON Schema value for `T` using draft-07 settings that represent
/// optional fields as the bare field type (no `anyOf: [schema, null]` wrapping).
///
/// GPT-5.4 and other strict schema validators prefer `required: [...]` to signal
/// mandatory fields rather than `null` entries in a type union.  Removing the null
/// wrapper also makes nested struct schemas directly accessible at their property key,
/// which simplifies both schema inspection and function-call schema validation.
pub(super) fn schema_value_no_null_options<T: JsonSchema>() -> Value {
    let mut settings = schemars::r#gen::SchemaSettings::draft07();
    settings.option_add_null_type = false;
    let generator = settings.into_generator();
    root_schema_to_value(generator.into_root_schema_for::<T>())
}

fn inline_refs(schema: &mut Value) {
    let defs = match schema.get("definitions").cloned() {
        Some(d) => d,
        None => return,
    };
    resolve_refs(schema, &defs, &mut HashSet::new());
    schema.as_object_mut().map(|o| o.remove("definitions"));
}

fn resolve_refs(node: &mut Value, defs: &Value, visited: &mut HashSet<String>) {
    if let Some(obj) = node.as_object() {
        if let Some(ref_val) = obj.get("$ref").and_then(|v| v.as_str()) {
            let name = ref_val.trim_start_matches("#/definitions/");
            if visited.contains(name) {
                return;
            }
            if let Some(def) = defs.get(name) {
                visited.insert(name.to_string());
                let mut replacement = def.clone();
                resolve_refs(&mut replacement, defs, visited);
                visited.remove(name);
                *node = replacement;
                return;
            }
        }
    }
    match node {
        Value::Object(map) => {
            for v in map.values_mut() {
                resolve_refs(v, defs, visited);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                resolve_refs(v, defs, visited);
            }
        }
        _ => {}
    }
}

/// Serialises a schemars `RootSchema` to a `serde_json::Value` and post-processes it
/// so that the output is safe for strict JSON Schema validators (e.g. GPT-5.4):
///
/// 1. Every `{"$ref": "#/definitions/TypeName"}` is replaced inline with the
///    definition body it points to, recursively until no `$ref` values remain.
/// 2. The top-level `definitions` object is removed (all refs have been inlined).
/// 3. The top-level `$schema` key is removed (not accepted by OpenAI tool schemas).
pub(super) fn root_schema_to_value(schema: RootSchema) -> Value {
    let mut value = serde_json::to_value(schema).expect("schemars root schema should serialize");

    if let Value::Object(ref mut map) = value {
        map.remove("$schema");
    }

    inline_refs(&mut value);
    value
}
