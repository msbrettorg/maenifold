use super::*;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use crate::workflow::pipeline::persist_final_pipeline_status;
use rmcp::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CancelTaskParams, GetTaskInfoParams, TaskSupport, ToolExecution,
};
use serde_json::{Map, Value, json};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

fn setup() -> (TempDir, ConfigResetGuard, ToolContext) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("dirs");
    fs::write(
        config.maenifold_root.join("maenifold.json"),
        r#"{"agents":{},"settings":{}}"#,
    )
    .expect("config");
    (
        tmp,
        ConfigResetGuard,
        ToolContext::new(config, OutputFormat::Json),
    )
}

fn definition() -> Value {
    json!({
        "nodes": [{ "id": "n0", "type": "tool", "toolName": "GetConfig" }],
        "edges": []
    })
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("object")
}

fn pipeline_request(name: &str) -> CallToolRequestParams {
    CallToolRequestParams::new("Pipeline")
        .with_arguments(object(json!({ "definition": definition(), "name": name })))
        .with_task(object(json!({ "ttlMs": 60_000 })))
}

async fn enqueue_pipeline(name: &str) -> String {
    let previous = std::env::var("MAENIFOLD_PIPELINE_DRIVER").ok();
    unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", "/usr/bin/true") };
    let result = task_hooks::enqueue_task(pipeline_request(name))
        .await
        .expect("enqueue");
    match previous {
        Some(value) => unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", value) },
        None => unsafe { std::env::remove_var("MAENIFOLD_PIPELINE_DRIVER") },
    }
    result.task.task_id
}

#[test]
fn initialize_advertises_tasks_capability() {
    let handler = MaenifoldHandler::new(TransportKind::Stdio).expect("handler");
    let info = handler.get_info();
    assert!(info.capabilities.tasks.is_some());
    assert!(
        info.capabilities
            .tasks
            .as_ref()
            .expect("tasks")
            .supports_tools_call()
    );
}

#[test]
fn tools_list_marks_pipeline_as_optional_task() {
    let handler = MaenifoldHandler::new(TransportKind::Stdio).expect("handler");
    let pipeline = handler
        .tools
        .iter()
        .find(|tool| tool.name.as_ref() == "Pipeline")
        .expect("Pipeline tool");
    assert_eq!(
        pipeline.execution.as_ref(),
        Some(&ToolExecution::new().with_task_support(TaskSupport::Optional))
    );
}

#[tokio::test]
#[serial]
async fn enqueue_task_for_pipeline_returns_create_task_result_with_session_id() {
    let (_tmp, _guard, context) = setup();
    let task_id = enqueue_pipeline("hook-enqueue").await;
    let viewed = crate::workflow::pipeline::pipeline(
        &context,
        &json!({ "sessionId": task_id, "view": true }),
    )
    .expect("view");
    let payload: Value = serde_json::from_str(&viewed).expect("json");
    assert_eq!(payload["data"]["pipelineId"], task_id);
}

#[tokio::test]
#[serial]
async fn cancel_task_idempotent_on_terminal_pipeline() {
    let (_tmp, _guard, context) = setup();
    let task_id = enqueue_pipeline("hook-cancel-terminal").await;
    persist_final_pipeline_status(&context, &task_id, "completed").expect("complete");
    let result = task_hooks::cancel_task(CancelTaskParams {
        meta: None,
        task_id: task_id.clone(),
    })
    .expect("cancel");
    assert_eq!(result.task.task_id, task_id);
    assert_eq!(
        result.task.status_message.as_deref(),
        Some("alreadyTerminal")
    );
}

#[tokio::test]
#[serial]
async fn get_task_info_maps_completed_pipeline_status() {
    let (_tmp, _guard, context) = setup();
    let task_id = enqueue_pipeline("hook-info").await;
    persist_final_pipeline_status(&context, &task_id, "completed").expect("complete");
    let result = task_hooks::get_task_info(GetTaskInfoParams {
        meta: None,
        task_id,
    })
    .expect("info");
    assert_eq!(format!("{:?}", result.task.status), "Completed");
}
