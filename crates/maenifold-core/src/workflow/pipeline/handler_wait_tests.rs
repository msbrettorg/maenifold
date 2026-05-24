use serde_json::{Value, json};
use serial_test::serial;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::markdown::{read_markdown, write_markdown};
use crate::registry::ToolRegistry;
use crate::tools::ToolContext;
use crate::tools::with_cancellation_scope;
use crate::util::json::OutputFormat;

use super::handler::pipeline;
use super::handler_wait::{active_wait_count, await_terminal_view};
use super::lease::pipeline_lock_path;
use super::session_write::persist_final_pipeline_status;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
use tokio_util::sync::CancellationToken;

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
        join_policy: None,
        input_schema: None,
        output_schema: None,
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
        input_artifact: None,
        output_artifact: None,
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
        input_schema: None,
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

fn set_pipeline_status(context: &ToolContext, session_id: &str, status: &str) {
    let uri = crate::sessions::pipeline_session_uri(session_id);
    let (frontmatter, content, _) = read_markdown(&uri, &context.config).expect("read pipeline");
    let mut frontmatter = frontmatter.expect("frontmatter");
    frontmatter.insert(
        "status".to_string(),
        serde_yml::Value::String(status.to_string()),
    );
    write_markdown(&uri, Some(&frontmatter), &content, &context.config).expect("write pipeline");
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
    set_pipeline_status(&context, session_id, "running");
    let cancel_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        pipeline(
            &cancel_context,
            &json!({ "sessionId": session_id, "cancel": true }),
        )
        .expect("cancel pipeline");
    });

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
    set_pipeline_status(&context, session_id, "running");
    let lock_path = pipeline_lock_path(session_id, &context.config);
    std::fs::create_dir_all(lock_path.parent().expect("lock parent")).expect("lock parent dir");
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .expect("lock file");
    let mut lock = fd_lock::RwLock::new(lock_file);
    let _guard = lock.try_write().expect("hold pipeline lock");

    let result = pipeline(
        &context,
        &json!({ "sessionId": session_id, "wait": true, "timeoutMs": 20 }),
    )
    .expect("timeout view");
    let data = parse_data(&result);
    assert_eq!(data["timedOut"], true);
    assert_eq!(data["final"], false);
    let follow_up = pipeline(&context, &json!({ "sessionId": session_id, "view": true }))
        .expect("follow-up view");
    let follow_up = parse_data(&follow_up);
    assert_eq!(follow_up["status"], "running");
    assert_eq!(follow_up["running"], true);
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

#[tokio::test]
#[serial]
async fn drop_mcp_blocking_wait_observes_request_cancel() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000000000-wait-mcp-drop";
    seed_active(&context, session_id);
    let finished = Arc::new(AtomicBool::new(false));
    let finished_in_blocking = finished.clone();

    let outer = tokio::spawn(async move {
        let token = CancellationToken::new();
        let _cancel_on_drop = CancelOnDrop(token.clone());
        let payload = json!({ "sessionId": session_id, "wait": true });
        let handle = tokio::task::spawn_blocking(move || {
            let result = with_cancellation_scope(token, || {
                ToolRegistry::global().try_invoke("Pipeline", &payload, OutputFormat::Json)
            });
            finished_in_blocking.store(true, Ordering::SeqCst);
            result
        });
        let _ = handle.await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    outer.abort();
    let deadline = Instant::now() + Duration::from_millis(200);
    while Instant::now() < deadline {
        if finished.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        finished.load(Ordering::SeqCst),
        "blocking MCP wait did not observe request cancellation"
    );
}

#[tokio::test]
#[serial]
async fn drop_gateway_native_wait_observes_request_cancel() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000000000-wait-gateway-drop";
    seed_active(&context, session_id);

    let request = tokio::spawn(async move {
        let _ = crate::mcp::gateway_runtime::invoke_native(
            "Pipeline".to_string(),
            json!({ "sessionId": session_id, "wait": true }),
        )
        .await;
    });

    let start_deadline = Instant::now() + Duration::from_secs(3);
    while active_wait_count() == 0 && Instant::now() < start_deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        active_wait_count() > 0,
        "gateway native wait did not start a pipeline waiter"
    );

    request.abort();
    let stop_deadline = Instant::now() + Duration::from_secs(3);
    while active_wait_count() > 0 && Instant::now() < stop_deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        active_wait_count(),
        0,
        "gateway native wait did not observe request cancellation"
    );
}

struct CancelOnDrop(CancellationToken);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
