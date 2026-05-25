// T-240.1 regression tests: lock in the rendered JSON Schema shape for
// GatewayExecute and GatewayBatch so the `parameters` object cannot be
// silently dropped from the MCP tool catalog (see todo/T-240.1).

use super::gateway_tools::{gateway_batch_tool, gateway_execute_tool, gateway_intent_tool};
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

    let required = required_set(schema);
    assert!(
        !required.contains("calls"),
        "GatewayBatch schema must not require `calls` at the top level because \
         status mode uses `batchId`; got {required:?}"
    );
    assert!(
        !required.contains("items"),
        "GatewayBatch schema must not require legacy `items`; got {required:?}"
    );

    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("GatewayBatch schema must declare properties");
    assert!(
        properties.contains_key("calls"),
        "GatewayBatch schema must expose `calls`; got {properties:?}"
    );
    assert!(
        !properties.contains_key("items"),
        "GatewayBatch schema must not expose legacy `items`; got {properties:?}"
    );

    let item_schema = properties
        .get("calls")
        .and_then(|calls| calls.get("items"))
        .expect("GatewayBatch.calls.items schema must be inlined");

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

#[test]
fn gateway_intent_schema_requires_goal_not_intent() {
    let tool = gateway_intent_tool();
    let schema = &tool.input_schema;

    let required = required_set(schema);
    assert!(
        required.contains("goal"),
        "GatewayIntent schema must require `goal`; got {required:?}"
    );
    assert!(
        !required.contains("intent"),
        "GatewayIntent schema must not require legacy `intent`; got {required:?}"
    );

    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("GatewayIntent schema must declare properties");
    assert!(
        properties.contains_key("goal"),
        "GatewayIntent schema must expose `goal`; got {properties:?}"
    );
    assert!(
        !properties.contains_key("intent"),
        "GatewayIntent schema must not expose legacy `intent`; got {properties:?}"
    );
}
