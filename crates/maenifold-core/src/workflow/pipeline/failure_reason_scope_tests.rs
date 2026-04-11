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
        display_label: None,
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

fn node_state(node_id: &str, status: NodeStatus) -> NodeState {
    NodeState {
        session_id: format!("workflow-1735689609999-{node_id}"),
        status,
        attempts: 1,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        output: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

#[test]
#[serial]
fn update_pipeline_node_status_persists_abandoned_failure_reason() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-failure-reason-abandoned";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        template_id: None,
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600000-node".to_string(),
            status: NodeStatus::Active,
            attempts: 1,
            acp_session_id: None,
            thinking_session_id: Some("session-1735689600000-node".to_string()),
            tool_provenance: None,
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
    assert_eq!(
        node_states["node"].thinking_session_id.as_deref(),
        Some("session-1735689600000-node")
    );
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
        template_id: None,
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600001-node".to_string(),
            status: NodeStatus::Active,
            attempts: 1,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
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
        template_id: None,
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600002-node".to_string(),
            status: NodeStatus::Abandoned,
            attempts: 1,
            acp_session_id: None,
            thinking_session_id: Some("session-1735689600002-node".to_string()),
            tool_provenance: None,
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
        template_id: None,
    };
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600003-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 3,
                acp_session_id: None,
                thinking_session_id: Some("session-1735689600003-root".to_string()),
                tool_provenance: None,
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
                acp_session_id: None,
                thinking_session_id: Some("session-1735689600003-child".to_string()),
                tool_provenance: None,
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

#[test]
#[serial]
fn update_pipeline_node_status_cascades_abandon_only_over_success_edges_to_fixpoint() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600100-cascade-abandon-fixpoint";
    let definition = PipelineDefinition {
        nodes: vec![
            workflow_node("root"),
            workflow_node("pending-child"),
            workflow_node("ready-child"),
            workflow_node("retrying-grandchild"),
            workflow_node("always-child"),
            workflow_node("other-completed"),
            workflow_node("mixed-child"),
        ],
        edges: vec![
            PipelineEdge {
                source: "root".to_string(),
                target: "pending-child".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "root".to_string(),
                target: "ready-child".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "ready-child".to_string(),
                target: "retrying-grandchild".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "root".to_string(),
                target: "always-child".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
            },
            PipelineEdge {
                source: "root".to_string(),
                target: "mixed-child".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "other-completed".to_string(),
                target: "mixed-child".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
        ],
        template_id: None,
    };
    let node_states = HashMap::from([
        ("root".to_string(), node_state("root", NodeStatus::Active)),
        (
            "pending-child".to_string(),
            node_state("pending-child", NodeStatus::Pending),
        ),
        (
            "ready-child".to_string(),
            node_state("ready-child", NodeStatus::Ready),
        ),
        (
            "retrying-grandchild".to_string(),
            NodeState {
                status: NodeStatus::Retrying,
                retry_after: Some("2025-01-01T00:00:00Z".to_string()),
                retry_count: Some(1),
                ..node_state("retrying-grandchild", NodeStatus::Retrying)
            },
        ),
        (
            "always-child".to_string(),
            node_state("always-child", NodeStatus::Pending),
        ),
        (
            "other-completed".to_string(),
            node_state("other-completed", NodeStatus::Completed),
        ),
        (
            "mixed-child".to_string(),
            node_state("mixed-child", NodeStatus::Pending),
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    update_pipeline_node_status(
        &context,
        session_id,
        "root",
        "abandoned",
        None,
        None,
        None,
        None,
        Some("root failed"),
    )
    .expect("abandon root");

    let node_states = read_raw_node_states(&context, session_id);
    assert_eq!(node_states["root"].status, NodeStatus::Abandoned);
    assert_eq!(
        node_states["root"].failure_reason.as_deref(),
        Some("root failed")
    );

    for node_id in ["pending-child", "ready-child", "retrying-grandchild"] {
        assert_eq!(
            node_states[node_id].status,
            NodeStatus::Abandoned,
            "{node_id}"
        );
        assert_eq!(
            node_states[node_id].failure_reason.as_deref(),
            Some("predecessor abandoned"),
            "{node_id}"
        );
    }

    assert_eq!(node_states["always-child"].status, NodeStatus::Pending);
    assert_eq!(node_states["always-child"].failure_reason, None);
    assert_eq!(node_states["mixed-child"].status, NodeStatus::Pending);
    assert_eq!(node_states["mixed-child"].failure_reason, None);
}

#[test]
#[serial]
fn update_pipeline_node_status_supports_stuck_cascade_over_success_edges() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600200-cascade-stuck";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("root"), workflow_node("child")],
        edges: vec![PipelineEdge {
            source: "root".to_string(),
            target: "child".to_string(),
            edge_type: EdgeType::Success,
            mapping: None,
        }],
        template_id: None,
    };
    let node_states = HashMap::from([
        (
            "root".to_string(),
            node_state("root-stuck", NodeStatus::Active),
        ),
        (
            "child".to_string(),
            node_state("child-stuck", NodeStatus::Pending),
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    update_pipeline_node_status(
        &context,
        session_id,
        "root",
        "stuck",
        None,
        None,
        None,
        None,
        Some("lease expired"),
    )
    .expect("mark root stuck");

    let node_states = read_raw_node_states(&context, session_id);
    assert_eq!(node_states["root"].status, NodeStatus::Stuck);
    assert_eq!(
        node_states["root"].failure_reason.as_deref(),
        Some("lease expired")
    );
    assert_eq!(node_states["child"].status, NodeStatus::Abandoned);
    assert_eq!(
        node_states["child"].failure_reason.as_deref(),
        Some("predecessor abandoned")
    );
}
