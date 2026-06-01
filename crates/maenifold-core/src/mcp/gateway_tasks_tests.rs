use super::gateway_runtime::GatewayState;
use super::gateway_tasks::{DOMAIN_NAME, invoke_tasks};
use super::resource_routing::ResourceRoutingRegistry;
use super::stdio_pool::StdioPool;
use crate::acp_config::IntentResolverSettings;
use crate::config::Config;
use crate::mcp::gateway_registry::GatewayRegistry;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use crate::workflow::pipeline::persist_final_pipeline_status;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
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

fn definition(nodes: usize) -> Value {
    let node_values = (0..nodes)
        .map(|idx| json!({ "id": format!("n{idx}"), "type": "tool", "toolName": "GetConfig" }))
        .collect::<Vec<_>>();
    let edge_values = (1..nodes)
        .map(|idx| json!({ "source": format!("n{}", idx - 1), "target": format!("n{idx}"), "type": "success" }))
        .collect::<Vec<_>>();
    json!({ "nodes": node_values, "edges": edge_values })
}

fn text(result: &CallToolResult) -> String {
    result.content[0].raw.as_text().expect("text").text.clone()
}

fn parsed(result: &CallToolResult) -> Value {
    serde_json::from_str(&text(result)).expect("json")
}

async fn create_task(name: &str, nodes: usize) -> String {
    let driver = crate::test_util::fake_exit_zero_binary();
    let previous = std::env::var("MAENIFOLD_PIPELINE_DRIVER").ok();
    unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", &driver) };
    let result = invoke_tasks(
        "TaskCreate".to_string(),
        json!({ "definition": definition(nodes), "name": name }),
    )
    .await
    .expect("task create");
    match previous {
        Some(value) => unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", value) },
        None => unsafe { std::env::remove_var("MAENIFOLD_PIPELINE_DRIVER") },
    }
    assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
    parsed(&result)["taskId"]
        .as_str()
        .expect("task id")
        .to_string()
}

async fn state() -> GatewayState {
    let pool = Arc::new(
        StdioPool::start(
            Vec::<crate::mcp::gateway_session::GatewayServerConfig>::new(),
            std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
        )
        .await
        .expect("pool"),
    );
    let registry = GatewayRegistry::build(Vec::new(), &BTreeMap::new()).expect("registry");
    let resources = Arc::new(ResourceRoutingRegistry::build(pool.as_ref()).expect("resources"));
    GatewayState::new(
        pool,
        resources,
        registry,
        IntentResolverSettings::default(),
        PathBuf::from("."),
        crate::mcp::config_tools::mutation_state_for_tests(),
    )
}

#[tokio::test]
#[serial]
async fn task_create_returns_task_id_matching_pipeline_session_id() {
    let (_tmp, _guard, context) = setup();
    let task_id = create_task("task-create", 1).await;
    let viewed = crate::workflow::pipeline::pipeline(
        &context,
        &json!({ "sessionId": task_id, "view": true }),
    )
    .expect("view");
    assert_eq!(
        serde_json::from_str::<Value>(&viewed).expect("json")["data"]["pipelineId"],
        task_id
    );
}

#[tokio::test]
#[serial]
async fn task_get_returns_current_status_without_blocking() {
    let (_tmp, _guard, _context) = setup();
    let task_id = create_task("task-get", 1).await;
    let result = invoke_tasks("TaskGet".to_string(), json!({ "taskId": task_id }))
        .await
        .expect("get");
    assert_eq!(parsed(&result)["status"], "working");
}

#[tokio::test]
#[serial]
async fn task_result_blocks_then_returns_final_status_for_2_node_fixture() {
    let (_tmp, _guard, context) = setup();
    let task_id = create_task("task-result", 2).await;
    let thread_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let thread_id = task_id.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        persist_final_pipeline_status(&thread_context, &thread_id, "completed").expect("complete");
    });
    let result = invoke_tasks(
        "TaskResult".to_string(),
        json!({ "taskId": task_id, "timeoutMs": 1000 }),
    )
    .await
    .expect("result");
    assert_eq!(parsed(&result)["status"], "completed");
}

#[tokio::test]
#[serial]
async fn task_result_with_timeout_returns_timed_out_flag_when_pipeline_still_running() {
    let (_tmp, _guard, _context) = setup();
    let task_id = create_task("task-timeout", 1).await;
    let result = invoke_tasks(
        "TaskResult".to_string(),
        json!({ "taskId": task_id, "timeoutMs": 5 }),
    )
    .await
    .expect("timeout");
    assert_eq!(parsed(&result)["result"]["timed_out"], true);
}

#[tokio::test]
#[serial]
async fn task_cancel_idempotent_on_terminal_pipeline_returns_already_terminal() {
    let (_tmp, _guard, context) = setup();
    let task_id = create_task("task-cancel-terminal", 1).await;
    persist_final_pipeline_status(&context, &task_id, "completed").expect("complete");
    let result = invoke_tasks("TaskCancel".to_string(), json!({ "taskId": task_id }))
        .await
        .expect("cancel");
    assert_eq!(parsed(&result)["status"], "alreadyTerminal");
}

#[tokio::test]
#[serial]
async fn task_cancel_running_pipeline_marks_cancelled() {
    let (_tmp, _guard, context) = setup();
    let task_id = create_task("task-cancel-running", 1).await;
    persist_final_pipeline_status(&context, &task_id, "running").expect("running");
    let result = invoke_tasks("TaskCancel".to_string(), json!({ "taskId": task_id }))
        .await
        .expect("cancel");
    assert_eq!(parsed(&result)["status"], "cancelled");
}

#[tokio::test]
#[serial]
async fn task_list_paginates_with_cursor() {
    let (_tmp, _guard, _context) = setup();
    create_task("task-list-a", 1).await;
    create_task("task-list-b", 1).await;
    let first = invoke_tasks("TaskList".to_string(), json!({ "limit": 1 }))
        .await
        .expect("list");
    let cursor = parsed(&first)["nextCursor"]
        .as_str()
        .expect("cursor")
        .to_string();
    let second = invoke_tasks(
        "TaskList".to_string(),
        json!({ "cursor": cursor, "limit": 1 }),
    )
    .await
    .expect("list page");
    assert_eq!(parsed(&second)["tasks"].as_array().expect("tasks").len(), 1);
}

#[tokio::test]
#[serial]
async fn task_list_returns_empty_for_empty_root() {
    let (_tmp, _guard, _context) = setup();
    let result = invoke_tasks("TaskList".to_string(), json!({}))
        .await
        .expect("list");
    assert!(
        parsed(&result)["tasks"]
            .as_array()
            .expect("tasks")
            .is_empty()
    );
}

#[tokio::test]
async fn gateway_registry_lists_maenifold_tasks_domain_with_5_commands() {
    let state = state().await;
    let commands = state
        .registry()
        .commands(DOMAIN_NAME)
        .expect("tasks domain");
    assert_eq!(commands.len(), 5);
    assert!(commands.iter().any(|command| command.name == "TaskCreate"));
}

#[tokio::test]
#[serial]
async fn gateway_execute_routes_task_create_to_invoke_tasks() {
    let (_tmp, _guard, _context) = setup();
    let state = state().await;
    let driver = crate::test_util::fake_exit_zero_binary();
    let previous = std::env::var("MAENIFOLD_PIPELINE_DRIVER").ok();
    unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", &driver) };
    let result = state
        .handle_gateway_execute(json!({
            "domain": DOMAIN_NAME,
            "command": "TaskCreate",
            "parameters": { "definition": definition(1), "name": "gateway-dispatch" }
        }))
        .await
        .expect("dispatch");
    match previous {
        Some(value) => unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", value) },
        None => unsafe { std::env::remove_var("MAENIFOLD_PIPELINE_DRIVER") },
    }
    assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
    assert!(parsed(&result)["taskId"].as_str().is_some());
}
