use std::collections::HashMap;

use serde_json::json;
use serial_test::serial;

use crate::util::json::OutputFormat;

use super::handler_lifecycle::handle_lifecycle_operation;
use super::session_read::{frontmatter_value, read_pipeline_session};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::update_restart::restart_pipeline_node;
use super::update_status::update_pipeline_node_status;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};

fn workflow_node(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        workflow_id: "test-workflow-a".to_string(),
        acp_client: Some(format!("agent-{id}")),
        prompt: None,
        timeout: None,
    }
}

fn read_raw_node_states(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

#[test]
#[serial]
fn update_pipeline_node_status_persists_abandoned_failure_reason() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-failure-reason-abandoned";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600000-node".to_string(),
            status: NodeStatus::Active,
            attempts: 1,
            thinking_session_id: Some("session-1735689600000-node".to_string()),
            output: Some("stale output".to_string()),
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    update_pipeline_node_status(
        &context,
        session_id,
        "node",
        "abandoned",
        None,
        None,
        Some("agent refused prompt"),
    )
    .expect("abandon node");

    let node_states = read_raw_node_states(&context, session_id);
    assert_eq!(node_states["node"].status, NodeStatus::Abandoned);
    assert_eq!(
        node_states["node"].failure_reason.as_deref(),
        Some("agent refused prompt")
    );
    assert_eq!(node_states["node"].thinking_session_id, None);
    assert_eq!(node_states["node"].output, None);
}

#[test]
#[serial]
fn update_pipeline_node_status_clears_failure_reason_when_completed() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-failure-reason-completed";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600001-node".to_string(),
            status: NodeStatus::Active,
            attempts: 1,
            thinking_session_id: None,
            output: None,
            failure_reason: Some("stale failure".to_string()),
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    update_pipeline_node_status(
        &context,
        session_id,
        "node",
        "completed",
        Some("done"),
        None,
        None,
    )
    .expect("complete node");

    let node_states = read_raw_node_states(&context, session_id);
    assert_eq!(node_states["node"].status, NodeStatus::Completed);
    assert_eq!(node_states["node"].output.as_deref(), Some("done"));
    assert_eq!(node_states["node"].failure_reason, None);
}

#[test]
#[serial]
fn restart_pipeline_node_clears_failure_reason() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-failure-reason-restart";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600002-node".to_string(),
            status: NodeStatus::Abandoned,
            attempts: 1,
            thinking_session_id: Some("session-1735689600002-node".to_string()),
            output: Some("bad output".to_string()),
            failure_reason: Some("tool timeout".to_string()),
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    restart_pipeline_node(&context, session_id, "node").expect("restart node");

    let node_states = read_raw_node_states(&context, session_id);
    assert_eq!(node_states["node"].status, NodeStatus::Pending);
    assert_eq!(node_states["node"].attempts, 2);
    assert_eq!(node_states["node"].output, None);
    assert_eq!(node_states["node"].thinking_session_id, None);
    assert_eq!(node_states["node"].failure_reason, None);
}

#[test]
#[serial]
fn rewind_clears_failure_reason_for_selected_node_and_descendants() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-failure-reason-rewind";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("root"), workflow_node("child")],
        edges: vec![PipelineEdge {
            source: "root".to_string(),
            target: "child".to_string(),
            edge_type: EdgeType::Success,
            mapping: None,
        }],
    };
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600003-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 3,
                thinking_session_id: Some("session-1735689600003-root".to_string()),
                output: Some("root output".to_string()),
                failure_reason: Some("stale root failure".to_string()),
                retry_after: None,
                retry_count: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600003-child".to_string(),
                status: NodeStatus::Abandoned,
                attempts: 2,
                thinking_session_id: Some("session-1735689600003-child".to_string()),
                output: Some("child output".to_string()),
                failure_reason: Some("child failure".to_string()),
                retry_after: None,
                retry_count: None,
            },
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    handle_lifecycle_operation(
        &context,
        session_id,
        &json!({ "rewind": { "nodeIds": ["root"] } }),
    )
    .expect("rewind root")
    .expect("rewind response");

    let node_states = read_raw_node_states(&context, session_id);
    for node_id in ["root", "child"] {
        assert_eq!(
            node_states[node_id].status,
            NodeStatus::Pending,
            "{node_id}"
        );
        assert_eq!(node_states[node_id].attempts, 0, "{node_id}");
        assert_eq!(node_states[node_id].output, None, "{node_id}");
        assert_eq!(node_states[node_id].thinking_session_id, None, "{node_id}");
        assert_eq!(node_states[node_id].failure_reason, None, "{node_id}");
    }
}
