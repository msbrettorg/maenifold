//! T-174.4: Real-engine timeout enforcement tests.
//!
//! These tests prove that:
//!   - When `timeout_seconds` is `Some(n)`, the engine wraps node execution in
//!     `tokio::time::timeout(Duration::from_secs(n), _)` and produces a
//!     `NodeStatus::Abandoned` whose `failure_reason` contains `"timeout after"`.
//!   - When `timeout_seconds` is `None`, no wrapper is applied and there is no
//!     silent default fallback (covered by the `extract_timeout` unit tests in
//!     `executor.rs`).
//!
//! No mocks, no fake futures: the test seeds a real pipeline session, configures
//! a real agent that blocks on `/bin/sleep`, and drives the production
//! `execute_pipeline` entry point. The agent sleep is the real awaiting future
//! that the outer `tokio::time::timeout` wrapper must interrupt.
//!
//! Why a Prompt node, not a Tool node? `Tool` nodes invoke `ToolRegistry::global()`
//! which dispatches synchronously through built-in tools — none of which expose a
//! configurable async sleep. External MCP tool dispatch funnels through
//! `invoke_via_stdio` which performs its own JSON-RPC handshake (subject to its
//! own protocol-level timeout) before any user-controlled wait, making the outer
//! engine timeout indistinguishable from inner handshake failures. The Prompt
//! node path with a `/bin/sleep` ACP agent is the cleanest real-engine vehicle to
//! prove that the outer `extract_timeout` → `tokio::time::timeout` wrapper at
//! `executor.rs::schedule_dispatched_node` actually elapses and produces the
//! exact `"timeout after {N}s"` failure_reason that downstream consumers
//! (UI, retry classifier) depend on.

use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use serde_json::json;
use serial_test::serial;
use tokio_util::sync::CancellationToken;

use crate::util::json::OutputFormat;

use super::super::executor::execute_pipeline;
use super::super::session_read::{frontmatter_value, read_pipeline_session};
use super::super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::super::types::NodeStatus;
use super::super::{NodeState, PipelineDefinition, PipelineNode};

fn read_raw_node_states(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

fn write_sleep_agent_config(context: &crate::tools::ToolContext, agent_name: &str) {
    fs::write(
        context.config.maenifold_root.join("maenifold.json"),
        serde_json::to_vec_pretty(&json!({
            "agents": {
                agent_name: {
                    "command": "/bin/sleep 5"
                }
            }
        }))
        .expect("serialize agents config"),
    )
    .expect("write agents config");
}

/// AC-2 / AC-3: With `timeout_seconds = Some(1)` the engine must wrap the node
/// execution in `tokio::time::timeout(Duration::from_secs(1), _)`, elapse against
/// the real `/bin/sleep 5` agent, mark the node `Abandoned`, and persist a
/// `failure_reason` containing `"timeout after"`.
#[tokio::test]
#[serial]
async fn explicit_timeout_seconds_elapses_and_abandons_node_with_timeout_after_reason() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    write_sleep_agent_config(&context, "sleep-agent");

    let session_id = "pipeline-1735689600000-t174-4-timeout";
    let definition = PipelineDefinition {
        nodes: vec![PipelineNode::Prompt {
            id: "sleep-node".to_string(),
            display_label: None,
            prompt: "Block until the outer timeout fires".to_string(),
            agent: "sleep-agent".to_string(),
            tool_mode: "auto".to_string(),
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            cwd: None,
            timeout: Some(1),
            agent_config: None,
        }],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([(
        "sleep-node".to_string(),
        NodeState {
            session_id: "workflow-1735689600001-sleep-node".to_string(),
            status: NodeStatus::Pending,
            attempts: 0,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            output: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 1);

    // Outer guard timeout strictly larger than the configured node timeout: if the
    // engine fails to enforce the 1s node TTL the test fails fast at 4s rather
    // than blocking the suite for the full 5s sleep.
    let result = tokio::time::timeout(
        Duration::from_secs(4),
        execute_pipeline(&context, session_id, "", CancellationToken::new()),
    )
    .await
    .expect("engine must enforce the 1s node timeout instead of waiting on the agent sleep")
    .expect("pipeline execution should return a structured result, not an error");

    assert_eq!(result.completed, 0, "no nodes should complete");
    assert_eq!(result.abandoned, 1, "the timed-out node must be abandoned");
    assert_eq!(
        result.status, "abandoned",
        "pipeline status must reflect the abandoned node"
    );

    let states = read_raw_node_states(&context, session_id);
    let state = &states["sleep-node"];
    assert_eq!(
        state.status,
        NodeStatus::Abandoned,
        "node status must be Abandoned after timeout"
    );
    let reason = state
        .failure_reason
        .as_deref()
        .expect("timeout must persist a failure_reason on the node state");
    assert!(
        reason.contains("timeout after"),
        "failure_reason must contain the spec-mandated substring 'timeout after'; got: {reason}"
    );
}
