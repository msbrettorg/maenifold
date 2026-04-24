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
        execute_pipeline(&context, session_id, "", CancellationToken::new(), None),
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
