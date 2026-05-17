use crate::config::Config;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::json;
use serial_test::serial;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
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

fn context_from_config(config: Config) -> ToolContext {
    ToolContext::new(config, OutputFormat::Json)
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
                "description": "Run [[asset-store-lock]]"
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

fn create_pipeline_for(
    context: &ToolContext,
    workflow_id: &str,
    name: &str,
) -> crate::error::Result<String> {
    crate::workflow::pipeline::pipeline(
        context,
        &json!({
            "name": name,
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
}

fn assert_referenced_error(error: &crate::error::MaenifoldError, asset: &str) {
    let message = error.to_string();
    assert!(message.contains(asset), "message: {message}");
    assert!(
        message.contains("referenced by running pipeline"),
        "message: {message}"
    );
}

fn assert_missing_asset_error(error: &crate::error::MaenifoldError, asset: &str) {
    let message = error.to_string();
    assert!(message.contains(asset), "message: {message}");
    assert!(
        message.contains("not found") || message.contains("Asset"),
        "message: {message}"
    );
}

#[test]
#[serial]
fn delete_blocks_when_active_status_pipeline_references_asset() {
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "foo");
    create_pipeline_for(&context, "foo", "active-ref").expect("create active pipeline");

    let error = crate::workflow::delete_workflow_asset(&context, &json!({ "name": "foo" }))
        .expect_err("active pipeline reference should block delete");

    assert_referenced_error(&error, "foo");
    assert!(asset_path(&context, "foo").exists());
}

#[test]
#[serial]
fn delete_blocked_by_concurrent_pipeline_creation() {
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "foo");
    let config = context.config.clone();
    let barrier = Arc::new(Barrier::new(2));

    let create_barrier = Arc::clone(&barrier);
    let create_config = config.clone();
    let creator = thread::spawn(move || {
        create_barrier.wait();
        let context = context_from_config(create_config);
        create_pipeline_for(&context, "foo", "create-wins")
    });

    barrier.wait();
    let create_result = creator.join().expect("creator thread");
    create_result.expect("create should win before delete");

    let delete_context = context_from_config(config);
    let error = crate::workflow::delete_workflow_asset(&delete_context, &json!({ "name": "foo" }))
        .expect_err("delete should see active pipeline reference");

    assert_referenced_error(&error, "foo");
    assert!(asset_path(&delete_context, "foo").exists());
}

#[test]
#[serial]
fn create_after_delete_observes_missing_asset() {
    // Pre-existing invariant — validate_workflow_node_definitions errors on missing assets.
    // Not specifically a lock test.
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "foo");

    crate::workflow::delete_workflow_asset(&context, &json!({ "name": "foo" }))
        .expect("delete should win");
    let error = create_pipeline_for(&context, "foo", "missing-after-delete")
        .expect_err("create should reject missing workflow asset");

    assert_missing_asset_error(&error, "foo");
    assert!(!asset_path(&context, "foo").exists());
}

#[test]
#[serial]
fn concurrent_creates_and_one_delete_final_state_consistent() {
    // Smoke check — does NOT exercise the lock; for race coverage see
    // delete_blocked_by_concurrent_pipeline_creation.
    let (_tmp, context, _guard) = test_context();
    save_asset(&context, "foo");
    let config = context.config.clone();
    let barrier = Arc::new(Barrier::new(5));
    let outcomes = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for index in 0..4 {
        let thread_config = config.clone();
        let thread_barrier = Arc::clone(&barrier);
        let thread_outcomes = Arc::clone(&outcomes);
        handles.push(thread::spawn(move || {
            thread_barrier.wait();
            let context = context_from_config(thread_config);
            let outcome = if index == 0 {
                crate::workflow::delete_workflow_asset(&context, &json!({ "name": "foo" }))
                    .map(|_| "delete-ok".to_string())
                    .map_err(|error| error.to_string())
            } else {
                create_pipeline_for(&context, "foo", &format!("create-{index}"))
                    .map(|_| format!("create-{index}-ok"))
                    .map_err(|error| error.to_string())
            };
            thread_outcomes
                .lock()
                .expect("outcomes lock")
                .push((index, outcome));
        }));
    }

    barrier.wait();
    for handle in handles {
        handle.join().expect("worker thread");
    }

    let outcomes = outcomes.lock().expect("outcomes lock");
    assert_eq!(outcomes.len(), 4);
    let delete_succeeded = outcomes
        .iter()
        .any(|(_, outcome)| matches!(outcome, Ok(value) if value == "delete-ok"));
    for (_, outcome) in outcomes.iter() {
        if let Err(message) = outcome {
            assert!(
                message.contains("foo")
                    || message.contains("referenced by running pipeline")
                    || message.contains("not found")
                    || message.contains("Asset"),
                "unexpected error: {message}"
            );
        }
    }

    let final_context = context_from_config(config);
    assert_eq!(
        asset_path(&final_context, "foo").exists(),
        !delete_succeeded
    );
}
