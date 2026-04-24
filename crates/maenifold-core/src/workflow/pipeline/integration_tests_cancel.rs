#![allow(unused_imports, dead_code)]

use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use serde_json::{Value, json};
use serial_test::serial;
use tokio_util::sync::CancellationToken;

use crate::util::json::OutputFormat;

use super::executor::execute_pipeline;
use super::integration_tests_helpers::*;
use super::session_read::{frontmatter_value, read_pipeline_session};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode, RetryPolicy,
};

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
    let reason = state
        .failure_reason
        .as_deref()
        .expect("timeout failure reason");
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
