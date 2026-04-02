use crate::mcp::McpToolDefinition;
use schemars::{JsonSchema, schema::RootSchema};
use serde_json::{Map, Value};

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

/// Serialises a schemars `RootSchema` to a `serde_json::Value` and post-processes it
/// so that the output is safe for strict JSON Schema validators (e.g. GPT-5.4):
///
/// 1. Every `{"$ref": "#/definitions/TypeName"}` is replaced inline with the
///    definition body it points to, recursively until no `$ref` values remain.
/// 2. The top-level `definitions` object is removed (all refs have been inlined).
/// 3. The top-level `$schema` key is removed (not accepted by OpenAI tool schemas).
pub(super) fn root_schema_to_value(schema: RootSchema) -> Value {
    let mut value = serde_json::to_value(schema).expect("schemars root schema should serialize");

    // Snapshot the definitions map before mutating `value`.
    let definitions: Map<String, Value> = value
        .get("definitions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    // Replace every $ref with its inlined definition body.
    inline_refs(&mut value, &definitions);

    // Strip keys that strict validators reject.
    if let Value::Object(ref mut map) = value {
        map.remove("definitions");
        map.remove("$schema");
    }

    value
}

/// Recursively walks `value` and replaces every JSON object of the form
/// `{"$ref": "#/definitions/TypeName"}` with the corresponding entry from
/// `definitions`.  After substitution the replacement node is itself walked,
/// which handles multi-level ref chains.
///
/// Objects that have a `$ref` key alongside other keys (unusual in schemars 0.8
/// output but legal JSON Schema) are replaced wholesale with the definition body;
/// the sibling keys are discarded, which is the standard JSON Schema `$ref`
/// semantics for draft-07.
fn inline_refs(value: &mut Value, definitions: &Map<String, Value>) {
    match value {
        Value::Object(map) => {
            // If this object IS a $ref, replace the whole node and recurse into
            // the replacement (to handle chained / nested refs).
            if let Some(Value::String(ref_str)) = map.get("$ref").cloned() {
                if let Some(type_name) = ref_str.strip_prefix("#/definitions/") {
                    if let Some(definition) = definitions.get(type_name) {
                        *value = definition.clone();
                        inline_refs(value, definitions);
                        return;
                    }
                }
            }
            // Otherwise recurse into every value in the object.
            for v in map.values_mut() {
                inline_refs(v, definitions);
            }
        }
        Value::Array(items) => {
            for v in items.iter_mut() {
                inline_refs(v, definitions);
            }
        }
        // Primitives (string, number, bool, null) contain no refs.
        _ => {}
    }
}
