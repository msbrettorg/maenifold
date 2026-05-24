use super::gateway_catalog_domain_tests::native_catalog;
use serde_json::json;

#[tokio::test]
async fn validate_parameters_reports_wrong_type_distinct_from_missing() {
    let catalog = native_catalog().await;
    let params = json!({"query": 42});
    let err = catalog
        .validate_parameters("maenifold.memory", "SearchMemories", &params)
        .expect_err("wrong-type query should fail validation");
    let text = err.to_string();

    assert!(
        text.contains("wrong type") && text.contains("expected string"),
        "must distinguish wrong-type from missing: got {text}"
    );
    assert!(!text.contains("is missing from payload"));
}

#[tokio::test]
async fn validate_parameters_lists_valid_parameters_on_unknown_key() {
    let catalog = native_catalog().await;
    let params = json!({"root": "/tmp", "unknownKey": true});
    let err = catalog
        .validate_parameters("maenifold.gate", "gate_Status", &params)
        .expect_err("unknown key should fail validation");
    let text = err.to_string();

    assert!(text.contains("Unknown parameter 'unknownKey'"));
    assert!(text.contains("Valid parameters:"));
    assert!(text.contains("root"));
    assert!(text.contains("ready"));
}

#[tokio::test]
async fn schedule_missing_action_field_returns_required_property_error() {
    let catalog = native_catalog().await;
    let params = json!({});
    let err = catalog
        .validate_parameters("maenifold.system", "Schedule", &params)
        .expect_err("missing action should fail validation");
    let text = err.to_string();

    assert!(
        text.contains("Required property 'action'")
            || text.contains("action")
            || text.contains("anyOf"),
        "missing action error should mention action or anyOf requirement: got {text}"
    );
}

#[tokio::test]
async fn schedule_unknown_action_returns_clear_error() {
    let catalog = native_catalog().await;
    let params = json!({"action":"bogus"});
    let err = catalog
        .validate_parameters("maenifold.system", "Schedule", &params)
        .expect_err("unknown action should fail validation");
    let text = err.to_string();

    assert!(
        text.contains("action") && (text.contains("bogus") || text.contains("/action")),
        "unknown action error should identify the bad action field: got {text}"
    );
}

#[tokio::test]
async fn schedule_start_missing_job_id_returns_required_property_error() {
    let catalog = native_catalog().await;
    let params = json!({
        "action":"create",
        "schedule":"every weekday at 9am",
        "agent":"swe",
        "prompt":"Run scheduled maintenance"
    });
    let err = catalog
        .validate_parameters("maenifold.system", "Schedule", &params)
        .expect_err("missing id on create should fail validation");
    let text = err.to_string();

    assert!(text.contains("Required property 'id'"));
}

#[tokio::test]
async fn schedule_extra_property_rejected_when_additional_properties_false() {
    let catalog = native_catalog().await;
    let params = json!({"action":"list","bogus":true});
    let err = catalog
        .validate_parameters("maenifold.system", "Schedule", &params)
        .expect_err("unknown property should fail validation");
    let text = err.to_string();

    assert!(text.contains("Unknown parameter 'bogus'"));
}

#[tokio::test]
async fn schedule_learn_only_payload_is_accepted() {
    let catalog = native_catalog().await;
    let params = json!({ "learn": true });
    catalog
        .validate_parameters("maenifold.system", "Schedule", &params)
        .expect("{\"learn\": true} must be accepted (renders help)");
}

#[tokio::test]
async fn schedule_update_accepts_null_mcp_action_to_clear() {
    let catalog = native_catalog().await;
    let params = json!({
        "action": "update",
        "id": "test-task",
        "mcp_action": serde_json::Value::Null
    });
    catalog
        .validate_parameters("maenifold.system", "Schedule", &params)
        .expect("update with null mcp_action must be accepted (clear-to-null path)");
}

#[tokio::test]
async fn schedule_create_rejects_null_mcp_action() {
    let catalog = native_catalog().await;
    let params = json!({
        "action": "create",
        "id": "x",
        "schedule": "every day at 9am",
        "agent": "swe",
        "prompt": "hi",
        "mcp_action": serde_json::Value::Null
    });
    let err = catalog
        .validate_parameters("maenifold.system", "Schedule", &params)
        .expect_err("create with null mcp_action must still be rejected (create is strict)");
    let text = err.to_string();
    assert!(
        text.contains("oneOf")
            || text.contains("mcp_action")
            || text.contains("schema validation failed"),
        "create with null mcp_action must be rejected with a schema-validation error: got {text}"
    );
}
