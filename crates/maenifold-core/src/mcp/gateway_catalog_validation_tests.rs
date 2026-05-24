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
