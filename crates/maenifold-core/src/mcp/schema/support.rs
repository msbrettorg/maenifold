use std::collections::HashSet;

use crate::mcp::McpToolDefinition;
use schemars::{JsonSchema, Schema};
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
        execution: None,
        meta: None,
    }
}

pub(super) fn derived_tool_definition<T: JsonSchema>(
    name: &str,
    description: &str,
) -> McpToolDefinition {
    tool_definition(name, description, schema_value::<T>())
}

pub(super) fn schema_value<T: JsonSchema>() -> Value {
    let settings = schemars::generate::SchemaSettings::draft07();
    let generator = settings.into_generator();
    root_schema_to_value(generator.into_root_schema_for::<T>())
}

/// Generates a JSON Schema value for `T` and post-processes it to remove `null` type
/// unions and `anyOf: [{type:"null"}, S]` wrappers introduced by `Option<T>` fields.
///
/// GPT-5.4 and other strict schema validators prefer `required: [...]` to signal
/// mandatory fields rather than `null` entries in a type union.  Removing the null
/// wrapper also makes nested struct schemas directly accessible at their property key,
/// which simplifies both schema inspection and function-call schema validation.
#[allow(dead_code)]
pub(super) fn schema_value_no_null_options<T: JsonSchema>() -> Value {
    let mut value = schema_value::<T>();
    strip_null_options(&mut value);
    value
}

/// Recursively rewrite `{"type": ["X", "null"]}` -> `{"type": "X"}` and
/// `{"anyOf": [..., {"type": "null"}, ...]}` to drop the null branch (unwrapping
/// the single remaining branch into the parent object when only one branch is left).
fn strip_null_options(node: &mut Value) {
    match node {
        Value::Object(map) => {
            if let Some(Value::Array(types)) = map.get_mut("type") {
                types.retain(|t| t.as_str() != Some("null"));
                if types.len() == 1 {
                    let only = types.remove(0);
                    map.insert("type".to_string(), only);
                }
            }
            if let Some(Value::Array(branches)) = map.get("anyOf").cloned() {
                let kept: Vec<Value> = branches
                    .into_iter()
                    .filter(|b| {
                        b.as_object()
                            .and_then(|o| o.get("type"))
                            .and_then(Value::as_str)
                            != Some("null")
                    })
                    .collect();
                if kept.is_empty() {
                    map.remove("anyOf");
                } else if kept.len() == 1 {
                    map.remove("anyOf");
                    if let Some(only) = kept.into_iter().next().and_then(|v| match v {
                        Value::Object(o) => Some(o),
                        _ => None,
                    }) {
                        for (k, v) in only {
                            map.entry(k).or_insert(v);
                        }
                    }
                } else {
                    map.insert("anyOf".to_string(), Value::Array(kept));
                }
            }
            for v in map.values_mut() {
                strip_null_options(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                strip_null_options(v);
            }
        }
        _ => {}
    }
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
    if let Some(obj) = node.as_object()
        && let Some(ref_val) = obj.get("$ref").and_then(|v| v.as_str())
    {
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

/// Serialises a schemars `Schema` to a `serde_json::Value` and post-processes it
/// so that the output is safe for strict JSON Schema validators (e.g. GPT-5.4):
///
/// 1. Every `{"$ref": "#/definitions/TypeName"}` is replaced inline with the
///    definition body it points to, recursively until no `$ref` values remain.
/// 2. The top-level `definitions` object is removed (all refs have been inlined).
/// 3. The top-level `$schema` key is removed (not accepted by OpenAI tool schemas).
pub(super) fn root_schema_to_value(schema: Schema) -> Value {
    let mut value = schema.to_value();

    if let Value::Object(ref mut map) = value {
        map.remove("$schema");
    }

    inline_refs(&mut value);
    value
}
