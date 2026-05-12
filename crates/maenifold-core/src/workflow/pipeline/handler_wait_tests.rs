use serde_json::{Value, json};
use serial_test::serial;
use std::{collections::HashMap, time::Duration};

use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::handler::pipeline;
use super::handler_wait::await_terminal_view;
use super::session_write::persist_final_pipeline_status;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

fn make_tool_node(id: &str) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        display_label: None,
        tool_name: "test.tool".to_string(),
        tool_args: None,
        agent: None,
        agent_config: None,
        timeout: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        model: None,
        thought_level: None,
        retry_policy: None,
    }
}

fn make_state(session_id: &str, status: NodeStatus) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status,
        attempts: 0,
        started_at: None,
        completed_at: None,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
        output: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn seed_active(context: &ToolContext, session_id: &str) {
    let definition = PipelineDefinition {
        nodes: vec![make_tool_node("tool")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([(
        "tool".to_string(),
        make_state("workflow-1700000000000-tool", NodeStatus::Active),
    )]);
    seed_pipeline_session_with_node_sessions(context, session_id, definition, node_states, 1);
}

fn parse_data(output: &str) -> Value {
    serde_json::from_str::<Value>(output).expect("json")["data"].clone()
}

fn complete_later(context: &ToolContext, session_id: &'static str, status: &'static str) {
    let thread_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        persist_final_pipeline_status(&thread_context, session_id, status).expect("persist status");
    });
}

#[test]
#[serial]
fn wait_then_success() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000000000-wait-success";
    seed_active(&context, session_id);
    complete_later(&context, session_id, "completed");

    let previous_driver = std::env::var("MAENIFOLD_PIPELINE_DRIVER").ok();
    unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", "/usr/bin/true") };
    let result = pipeline(
        &context,
        &json!({ "sessionId": session_id, "run": true, "wait": true }),
    );
    match previous_driver {
        Some(value) => unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", value) },
        None => unsafe { std::env::remove_var("MAENIFOLD_PIPELINE_DRIVER") },
    }

    let data = parse_data(&result.expect("pipeline wait"));
    assert_eq!(data["status"], "completed");
    assert_eq!(data["final"], true);
}

#[tokio::test]
#[serial]
async fn wait_then_failure() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000000000-wait-failure";
    seed_active(&context, session_id);
    complete_later(&context, session_id, "failed");

    let data = await_terminal_view(&context, session_id, None)
        .await
        .expect("wait failure");
    assert_eq!(data["status"], "failed");
    assert_eq!(data["final"], true);
}

#[tokio::test]
#[serial]
async fn wait_then_cancel_from_other_call() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000000000-wait-cancel";
    seed_active(&context, session_id);
    complete_later(&context, session_id, "cancelled");

    let data = await_terminal_view(&context, session_id, None)
        .await
        .expect("wait cancel");
    assert_eq!(data["status"], "cancelled");
    assert_eq!(data["final"], true);
}

#[test]
#[serial]
fn wait_timeout_returns_view_with_timed_out_flag() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000000000-wait-timeout";
    seed_active(&context, session_id);

    let result = pipeline(
        &context,
        &json!({ "sessionId": session_id, "wait": true, "timeoutMs": 20 }),
    )
    .expect("timeout view");
    let data = parse_data(&result);
    assert_eq!(data["timedOut"], true);
    assert_eq!(data["final"], false);
}

#[tokio::test]
#[serial]
async fn drop_wait_does_not_leak_task() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000000000-wait-drop";
    seed_active(&context, session_id);
    let task_context = ToolContext::new(context.config.clone(), OutputFormat::Json);

    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(async move { await_terminal_view(&task_context, session_id, None).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    tasks.abort_all();
    let result = tasks.join_next().await.expect("join aborted wait");
    assert!(result.expect_err("aborted task").is_cancelled());
}
