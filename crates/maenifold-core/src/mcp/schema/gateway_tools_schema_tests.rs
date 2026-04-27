// T-240.1 regression tests: lock in the rendered JSON Schema shape for
// GatewayExecute and GatewayBatch so the `parameters` object cannot be
// silently dropped from the MCP tool catalog (see todo/T-240.1).

use super::gateway_tools::{gateway_batch_tool, gateway_execute_tool};
use serde_json::Value;

fn required_set(schema: &Value) -> std::collections::BTreeSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn gateway_execute_schema_requires_parameters_object() {
    let tool = gateway_execute_tool();
    let schema = &tool.input_schema;

    let required = required_set(schema);
    assert!(
        required.contains("domain"),
        "GatewayExecute schema must require `domain`; got {required:?}"
    );
    assert!(
        required.contains("command"),
        "GatewayExecute schema must require `command`; got {required:?}"
    );
    assert!(
        required.contains("parameters"),
        "GatewayExecute schema must require `parameters` so MCP clients can \
         pass command-specific arguments; got {required:?}"
    );

    let parameters = schema
        .get("properties")
        .and_then(|p| p.get("parameters"))
        .expect("GatewayExecute schema must declare a `parameters` property");
    assert_eq!(
        parameters.get("type").and_then(Value::as_str),
        Some("object"),
        "`parameters` must be typed as a JSON object: {parameters}"
    );

    // additionalProperties: false at the top level keeps clients honest and
    // surfaces typos like `idempotencyKey` (removed in T-238.31) loudly.
    assert_eq!(
        schema.get("additionalProperties"),
        Some(&Value::Bool(false)),
        "GatewayExecute schema must set additionalProperties=false at the top level"
    );

    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("schema must have properties map");
    assert!(
        !properties.contains_key("idempotencyKey"),
        "idempotencyKey was removed in T-238.31; it must not reappear in the schema"
    );
}

#[test]
fn gateway_batch_item_schema_requires_parameters_object() {
    let tool = gateway_batch_tool();
    let schema = &tool.input_schema;

    let item_schema = schema
        .get("properties")
        .and_then(|p| p.get("items"))
        .and_then(|items| items.get("items"))
        .expect("GatewayBatch.items.items schema must be inlined");

    let required = required_set(item_schema);
    assert!(
        required.contains("domain"),
        "BatchItem must require `domain`; got {required:?}"
    );
    assert!(
        required.contains("command"),
        "BatchItem must require `command`; got {required:?}"
    );
    assert!(
        required.contains("parameters"),
        "BatchItem must require `parameters`; got {required:?}"
    );

    let parameters = item_schema
        .get("properties")
        .and_then(|p| p.get("parameters"))
        .expect("BatchItem must declare a `parameters` property");
    assert_eq!(
        parameters.get("type").and_then(Value::as_str),
        Some("object"),
        "BatchItem `parameters` must be typed as object: {parameters}"
    );
}
