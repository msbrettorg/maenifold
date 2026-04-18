use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use serde_json::{Value, json};
use serial_test::serial;
use tokio_util::sync::CancellationToken;

use crate::util::json::OutputFormat;

use super::executor::execute_pipeline;
use super::session_read::{frontmatter_value, read_pipeline_session};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode, RetryPolicy,
};

fn pending_node_state(session_id: &str) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status: NodeStatus::Pending,
        attempts: 0,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        output: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn read_raw_node_states(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

fn write_mcp_server_config(context: &crate::tools::ToolContext, mcp_servers: Value) {
    fs::write(
        context.config.maenifold_root.join("maenifold.json"),
        serde_json::to_vec_pretty(&json!({ "mcpServers": mcp_servers }))
            .expect("serialize MCP server config"),
    )
    .expect("write MCP server config");
}

fn write_sleep_mcp_server_config(context: &crate::tools::ToolContext) {
    write_mcp_server_config(
        context,
        json!([{
            "type": "stdio",
            "name": "sleepmcp",
            "command": "/bin/sleep",
            "args": ["5"]
        }]),
    );
}

// NOTE: flaky retry integration test removed — requires python3 MCP server
// fixture that's environment-dependent. The retry unit tests in
// executor_retry_tests.rs cover the logic; this integration path needs
// a stable test-harness binary (not an inline Python script).

#[allow(dead_code)]
fn write_flaky_retry_mcp_server_config(
    context: &crate::tools::ToolContext,
    counter_file: &std::path::Path,
) {
    let script = r#"
import json
import os
import sys

COUNTER_PATH = sys.argv[1] if len(sys.argv) > 1 else ""

def read_message():
    while True:
        line = sys.stdin.readline()
        if line == "":
            return None
        line = line.strip()
        if not line:
            continue
        return json.loads(line)

def write_message(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":"), ensure_ascii=False) + "\n")
    sys.stdout.flush()

def read_counter():
    if not COUNTER_PATH:
        return 0
    try:
        with open(COUNTER_PATH, "r", encoding="utf-8") as file:
            return int((file.read() or "0").strip())
    except Exception:
        return 0

def write_counter(value):
    if not COUNTER_PATH:
        return
    with open(COUNTER_PATH, "w", encoding="utf-8") as file:
        file.write(str(value))

def text_result(text):
    return {
        "content": [{"type": "text", "text": text}],
        "isError": False
    }

TOOLS = [
    {"name": "route", "description": "routes conditional edge", "inputSchema": {"type": "object"}},
    {"name": "flaky", "description": "fails first call then succeeds", "inputSchema": {"type": "object"}},
    {"name": "sink", "description": "terminal sink", "inputSchema": {"type": "object"}}
]

while True:
    message = read_message()
    if message is None:
        break

    method = message.get("method")
    msg_id = message.get("id")
    params = message.get("params") or {}

    if method == "initialize":
        if msg_id is not None:
            write_message({
                "jsonrpc": "2.0",
                "id": msg_id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "flakymcp", "version": "0.0.1"}
                }
            })
        continue

    if method == "notifications/initialized":
        continue

    if method == "tools/list":
        if msg_id is not None:
            write_message({
                "jsonrpc": "2.0",
                "id": msg_id,
                "result": {"tools": TOOLS}
            })
        continue

    if method == "tools/call":
        tool_name = params.get("name")
        if tool_name == "route":
            write_message({
                "jsonrpc": "2.0",
                "id": msg_id,
                "result": text_result("{\"route\":\"run\"}")
            })
            continue

        if tool_name == "sink":
            write_message({
                "jsonrpc": "2.0",
                "id": msg_id,
                "result": text_result("{\"done\":true}")
            })
            continue

        if tool_name == "flaky":
            current = read_counter() + 1
            write_counter(current)
            if current == 1:
                write_message({
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "error": {
                        "code": -32001,
                        "message": "transient first-call failure"
                    }
                })
            else:
                write_message({
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "result": text_result("{\"ok\":true}")
                })
            continue

        write_message({
            "jsonrpc": "2.0",
            "id": msg_id,
            "error": {"code": -32601, "message": "unknown tool"}
        })
        continue

    if msg_id is not None:
        write_message({
            "jsonrpc": "2.0",
            "id": msg_id,
            "error": {"code": -32601, "message": "unknown method"}
        })
"#;

    write_mcp_server_config(
        context,
        json!([{
            "type": "stdio",
            "name": "flakymcp",
            "command": "/usr/bin/python3",
            "args": ["-u", "-c", script, counter_file]
        }]),
    );
}

#[tokio::test]
#[serial]
async fn conditional_retry_policy_transient_failure_then_success_pipeline_completes() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let counter_file = context.config.maenifold_root.join("flaky-counter.txt");
    write_flaky_retry_mcp_server_config(&context, &counter_file);

    let session_id = "pipeline-1735689605000-integration-retry-conditional";
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Tool {
                id: "route".to_string(),
                display_label: None,
                tool_name: "flakymcp.route".to_string(),
                tool_args: Some(Value::Object(Default::default())),
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                retry_policy: None,
            },
            PipelineNode::Tool {
                id: "retry-node".to_string(),
                display_label: None,
                tool_name: "flakymcp.flaky".to_string(),
                tool_args: Some(Value::Object(Default::default())),
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                retry_policy: Some(RetryPolicy {
                    max_attempts: 3,
                    backoff_min_ms: 1,
                    backoff_max_ms: 5,
                    jitter: false,
                }),
            },
            PipelineNode::Tool {
                id: "sink".to_string(),
                display_label: None,
                tool_name: "flakymcp.sink".to_string(),
                tool_args: Some(Value::Object(Default::default())),
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                retry_policy: None,
            },
        ],
        edges: vec![
            PipelineEdge {
                source: "route".to_string(),
                target: "retry-node".to_string(),
                edge_type: EdgeType::Conditional {
                    path: "route".to_string(),
                    equals: json!("run"),
                },
                mapping: None,
            },
            PipelineEdge {
                source: "retry-node".to_string(),
                target: "sink".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
        ],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([
        (
            "route".to_string(),
            pending_node_state("workflow-1735689605001-route"),
        ),
        (
            "retry-node".to_string(),
            pending_node_state("workflow-1735689605002-retry-node"),
        ),
        (
            "sink".to_string(),
            pending_node_state("workflow-1735689605003-sink"),
        ),
    ]);
    seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 1);

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        execute_pipeline(&context, session_id, "", CancellationToken::new()),
    )
    .await
    .expect("pipeline execution should finish")
    .expect("pipeline execution should succeed");

    assert_eq!(result.completed, 3);
    assert_eq!(result.abandoned, 0);
    assert_eq!(result.status, "completed");

    let node_states = read_raw_node_states(&context, session_id);
    assert_eq!(node_states["route"].status, NodeStatus::Completed);
    assert_eq!(node_states["retry-node"].status, NodeStatus::Completed);
    assert_eq!(node_states["sink"].status, NodeStatus::Completed);
    assert_eq!(node_states["retry-node"].retry_count, Some(1));
    assert_eq!(node_states["retry-node"].retry_after, None);
    assert_eq!(node_states["retry-node"].failure_reason, None);
}

#[tokio::test]
#[serial]
async fn timeout_is_terminal_even_with_retry_policy_node_abandons_without_retrying() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    write_sleep_mcp_server_config(&context);

    let session_id = "pipeline-1735689606000-integration-timeout-terminal";
    let definition = PipelineDefinition {
        nodes: vec![PipelineNode::Tool {
            id: "sleep-node".to_string(),
            display_label: None,
            tool_name: "sleepmcp.block".to_string(),
            tool_args: Some(Value::Object(Default::default())),
            agent: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            cwd: None,
            timeout: Some(1),
            agent_config: None,
            retry_policy: Some(RetryPolicy {
                max_attempts: 3,
                backoff_min_ms: 1,
                backoff_max_ms: 5,
                jitter: false,
            }),
        }],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([(
        "sleep-node".to_string(),
        pending_node_state("workflow-1735689606001-sleep-node"),
    )]);
    seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 1);

    let result = tokio::time::timeout(
        Duration::from_secs(4),
        execute_pipeline(&context, session_id, "", CancellationToken::new()),
    )
    .await
    .expect("pipeline timeout test should finish quickly")
    .expect("pipeline execution should return a structured result");

    assert_eq!(result.completed, 0);
    assert_eq!(result.abandoned, 1);
    assert_eq!(result.status, "abandoned");

    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["sleep-node"];
    assert_eq!(state.status, NodeStatus::Abandoned);
    let reason = state.failure_reason.as_deref().expect("timeout failure reason");
    assert!(
        reason.contains("timeout after"),
        "expected timeout marker in failure reason, got: {reason}"
    );
    assert!(
        !reason.contains("transient retries"),
        "outer timeout must bypass retry wrapper, got: {reason}"
    );
    assert_eq!(state.retry_after, None);
    assert_eq!(state.retry_count, None);
}

#[tokio::test]
#[serial]
async fn cancel_during_active_execution_abandons_active_nodes_without_hanging() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    write_sleep_mcp_server_config(&context);

    let session_id = "pipeline-1735689607000-integration-cancel-active";
    let definition = PipelineDefinition {
        nodes: vec![PipelineNode::Tool {
            id: "sleep-node".to_string(),
            display_label: None,
            tool_name: "sleepmcp.block".to_string(),
            tool_args: Some(Value::Object(Default::default())),
            agent: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            cwd: None,
            timeout: None,
            agent_config: None,
            retry_policy: None,
        }],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([(
        "sleep-node".to_string(),
        pending_node_state("workflow-1735689607001-sleep-node"),
    )]);
    seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 1);

    let cancel = CancellationToken::new();
    let cancel_for_timer = cancel.clone();
    let cancel_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel_for_timer.cancel();
    });

    let result = tokio::time::timeout(
        Duration::from_secs(4),
        execute_pipeline(&context, session_id, "", cancel.clone()),
    )
    .await
    .expect("executor must stop promptly after cancellation")
    .expect("executor should return a result");
    cancel_task
        .await
        .expect("cancellation helper task should not panic");

    assert_eq!(result.status, "cancelled");
    assert_eq!(result.completed, 0);
    assert_eq!(result.abandoned, 1);

    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["sleep-node"];
    assert_eq!(state.status, NodeStatus::Abandoned);
    assert_eq!(
        state.failure_reason.as_deref(),
        Some("pipeline cancelled by user")
    );
    assert!(
        !matches!(state.status, NodeStatus::Active | NodeStatus::Retrying),
        "cancelled pipeline should not leave active/retrying node state behind"
    );
}
