use super::*;
use crate::config::Config;
use crate::markdown::{read_markdown, uri_to_path, write_markdown};
use crate::sessions::pipeline_session_uri;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serde_yml::Value as YamlValue;
use serial_test::serial;
use std::fs::OpenOptions;
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

fn mark_pipeline_running(context: &ToolContext, pipeline_id: &str) {
    let uri = pipeline_session_uri(pipeline_id);
    let (frontmatter, content, _) = read_markdown(&uri, &context.config).expect("read pipeline");
    let mut frontmatter = frontmatter.expect("pipeline frontmatter");
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("running".to_string()),
    );
    write_markdown(&uri, Some(&frontmatter), &content, &context.config).expect("write pipeline");
}

fn pipeline_lock_path(context: &ToolContext, pipeline_id: &str) -> std::path::PathBuf {
    let mut path = uri_to_path(
        &pipeline_session_uri(pipeline_id),
        &context.config.memory_path,
    )
    .expect("pipeline path");
    path.set_extension("lock");
    path
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
    mark_pipeline_running(&context, &pipeline_id);
    let lock_path = pipeline_lock_path(&context, &pipeline_id);
    std::fs::create_dir_all(lock_path.parent().expect("lock path parent"))
        .expect("create lock parent");
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .expect("open lock file");
    let mut lock = fd_lock::RwLock::new(lock_file);
    let _guard = lock.try_write().expect("hold pipeline lock");

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
    let expected = asset_path(&context, "listed-flow");

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
fn delete_workflow_asset_rejects_symlink() {
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "target");
    let target = asset_path(&context, "target");
    let link = asset_path(&context, "link");
    std::os::unix::fs::symlink(&target, &link).expect("create symlink");

    let error = delete_workflow_asset(&context, &json!({ "name": "link" }))
        .expect_err("symlink delete should fail");

    let message = error.to_string();
    assert!(message.contains("symlink"), "message: {message}");
    assert!(target.exists(), "symlink target should remain present");
}

#[test]
#[serial]
fn delete_workflow_asset_ignores_abandoned_pipeline() {
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "delete-me");
    let pipeline_id = create_referencing_pipeline(&context, "delete-me");
    mark_pipeline_running(&context, &pipeline_id);
    let path = asset_path(&context, "delete-me");

    delete_workflow_asset(&context, &json!({ "name": "delete-me" }))
        .expect("abandoned running-status pipeline without lock should not block delete");

    assert!(!path.exists(), "asset should be removed");
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
