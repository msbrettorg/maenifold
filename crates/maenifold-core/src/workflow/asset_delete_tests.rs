use super::*;
use crate::config::Config;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use tempfile::TempDir;

fn test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    std::fs::write(
        config.maenifold_root.join("maenifold.json"),
        r#"{"agents":{},"settings":{}}"#,
    )
    .expect("write config");
    (
        tmp,
        ToolContext::new(config, OutputFormat::Json),
        ConfigResetGuard,
    )
}

fn save_asset(context: &ToolContext, name: &str) {
    let path = asset_path(context, name);
    std::fs::create_dir_all(path.parent().expect("asset path parent"))
        .expect("create workflow asset directory");
    std::fs::write(
        path,
        serde_json::to_string(&json!({
            "id": name,
            "steps": [{
                "id": "step-1",
                "name": "Step 1",
                "description": "Run [[workflow-asset-delete]]"
            }]
        }))
        .expect("serialize workflow asset"),
    )
    .expect("write workflow asset");
}

fn asset_path(context: &ToolContext, name: &str) -> std::path::PathBuf {
    context
        .config
        .assets_path
        .join("workflows")
        .join(format!("{name}.json"))
}

fn create_referencing_pipeline(context: &ToolContext, workflow_id: &str) -> String {
    let response = crate::workflow::pipeline::pipeline(
        context,
        &json!({
            "definition": {
                "nodes": [{
                    "id": "wf",
                    "type": "workflow",
                    "workflowId": workflow_id,
                    "acpClient": "test-agent"
                }],
                "edges": []
            }
        }),
    )
    .expect("create pipeline");
    let payload: Value = serde_json::from_str(&response).expect("parse pipeline response");
    payload["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id")
        .to_string()
}

#[test]
#[serial]
fn delete_workflow_asset_succeeds_for_unreferenced_asset() {
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "delete-me");
    let path = asset_path(&context, "delete-me");
    assert!(path.exists(), "asset should exist before delete");

    delete_workflow_asset(&context, &json!({ "name": "delete-me" }))
        .expect("delete should succeed");

    assert!(!path.exists(), "asset should be removed");
}

#[test]
#[serial]
fn delete_workflow_asset_refuses_when_referenced_by_running_pipeline() {
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "blocked-flow");
    let pipeline_id = create_referencing_pipeline(&context, "blocked-flow");

    let error = delete_workflow_asset(&context, &json!({ "name": "blocked-flow" }))
        .expect_err("delete should refuse referenced asset");

    let message = error.to_string();
    assert!(message.contains("blocked-flow"), "message: {message}");
    assert!(message.contains(&pipeline_id), "message: {message}");
    assert!(asset_path(&context, "blocked-flow").exists());
}

#[test]
#[serial]
fn delete_workflow_asset_response_lists_deleted_files() {
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "listed-flow");
    let expected = std::fs::canonicalize(asset_path(&context, "listed-flow"))
        .expect("canonicalize asset path");

    let response = delete_workflow_asset(&context, &json!({ "name": "listed-flow" }))
        .expect("delete should succeed");

    let payload: Value = serde_json::from_str(&response).expect("parse delete response");
    assert_eq!(payload["success"], true);
    assert_eq!(
        payload["data"]["deletedFiles"],
        json!([expected.to_string_lossy().to_string()])
    );
}

#[test]
#[serial]
fn delete_workflow_asset_rejects_missing_name() {
    let (_tmp, context, _guard) = test_context();

    let error = delete_workflow_asset(&context, &json!({})).expect_err("missing name should fail");

    assert!(
        error.to_string().contains("Required property 'name'"),
        "unexpected error: {error}"
    );
}
