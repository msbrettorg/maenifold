#![allow(unused_imports, dead_code)]

use std::collections::HashMap;

use serde_json::json;
use serial_test::serial;

use crate::util::json::OutputFormat;

use super::handler_lifecycle::handle_lifecycle_operation;
use super::session_read::{frontmatter_value, read_pipeline_session};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::update_restart::restart_pipeline_node;
use super::update_status::update_pipeline_node_status;
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    compute_pipeline_status_with_definition,
};

fn workflow_node(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        display_label: None,
        workflow_id: "test-workflow-a".to_string(),
        acp_client: Some(format!("agent-{id}")),
        prompt: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
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
        inputs: None,
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

#[test]
#[serial]
fn conditional_false_branch_is_abandoned_and_pipeline_leaves_active_state() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600300-conditional-false-branch-abandoned";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("a"), workflow_node("b"), workflow_node("c")],
        edges: vec![
            PipelineEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: EdgeType::Conditional {
                    path: "verdict".to_string(),
                    equals: serde_json::Value::String("FAIL".to_string()),
                },
                mapping: None,
            },
            PipelineEdge {
                source: "b".to_string(),
                target: "c".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
        ],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([
        (
            "a".to_string(),
            NodeState {
                session_id: "workflow-1735689600300-a".to_string(),
                status: NodeStatus::Active,
                attempts: 1,
                acp_session_id: None,
                thinking_session_id: None,
                tool_provenance: None,
                output: None,
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        ),
        (
            "b".to_string(),
            NodeState {
                session_id: "workflow-1735689600300-b".to_string(),
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
        ),
        (
            "c".to_string(),
            NodeState {
                session_id: "workflow-1735689600300-c".to_string(),
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
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, session_id, definition.clone(), node_states, 3);

    update_pipeline_node_status(
        &context,
        session_id,
        "a",
        "completed",
        Some(r#"{"verdict":"PASS"}"#),
        None,
        None,
        None,
        None,
    )
    .expect("complete node a");

    let node_states = read_raw_node_states(&context, session_id);
    assert_eq!(node_states["a"].status, NodeStatus::Completed);
    assert_eq!(node_states["b"].status, NodeStatus::Abandoned);
    assert_eq!(node_states["c"].status, NodeStatus::Abandoned);
    assert_eq!(
        node_states["b"].failure_reason.as_deref(),
        Some("conditional predicate not satisfied")
    );
    assert_eq!(
        node_states["c"].failure_reason.as_deref(),
        Some("predecessor abandoned")
    );
    assert_eq!(
        compute_pipeline_status_with_definition(&definition, &node_states),
        "abandoned",
        "pipeline should not remain active when conditional-only downstream is unreachable"
    );
}
