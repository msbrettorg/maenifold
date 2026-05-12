use maenifold_core::config::Config;
use maenifold_core::registry::ToolRegistry;
use maenifold_core::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;

#[test]
#[serial]
fn save_workflow_asset_dispatched_by_name_writes_file_and_returns_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    Config::override_root(dir.path());

    let registry = ToolRegistry::new();
    let payload = json!({
        "id": "dispatch-test-flow",
        "content": r#"{"id":"dispatch-test-flow","nodes":[]}"#
    });

    let result = registry
        .try_invoke("SaveWorkflowAsset", &payload, OutputFormat::Json)
        .expect("dispatch should succeed");

    Config::reset_overrides();

    let parsed: Value = serde_json::from_str(&result).expect("response is JSON");
    assert_eq!(parsed["success"], true, "response success flag");
    assert_eq!(parsed["data"]["id"], "dispatch-test-flow", "echoed id");

    let expected = dir
        .path()
        .join("assets")
        .join("workflows")
        .join("dispatch-test-flow.json");
    assert!(expected.exists(), "workflow file must exist at expected path");

    let on_disk: Value = serde_json::from_str(
        &std::fs::read_to_string(&expected).expect("read file"),
    )
    .expect("file is valid JSON");
    assert_eq!(on_disk["id"], "dispatch-test-flow", "persisted id matches");
}

#[test]
fn save_workflow_asset_is_registered_with_all_aliases() {
    let registry = ToolRegistry::new();
    assert!(registry.contains("SaveWorkflowAsset"), "canonical name");
    assert!(registry.contains("saveworkflowasset"), "lowercase alias");
    assert!(registry.contains("save_workflow_asset"), "snake_case alias");
}
