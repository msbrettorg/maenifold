#![allow(unused_imports, dead_code)]

use std::collections::HashMap;

use serial_test::serial;

use crate::markdown::write_markdown;
use crate::util::json::OutputFormat;

use super::super::engine_ready::{compute_ready_set, has_remaining_work};
use super::super::engine_status::compute_pipeline_status;
use super::super::executor_nodes::{FailurePhase, NodeOutcome};
use super::super::session_read::{frontmatter_value, read_pipeline_session};
use super::super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
use super::*;

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
        join_policy: None,
    }
}

fn read_raw_node_states(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

fn set_max_attempts(context: &crate::tools::ToolContext, session_id: &str, max_attempts: u32) {
    let (uri, mut frontmatter, content) =
        read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter.insert(
        "maxAttempts".to_string(),
        serde_yml::to_value(max_attempts).expect("serialize max attempts"),
    );
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)
        .expect("write pipeline session");
}

#[test]
fn retrying_status_serializes_roundtrip() {
    let status = NodeStatus::Retrying;
    let yaml = serde_yml::to_string(&status).unwrap();
    assert_eq!(yaml.trim(), "retrying");

    let deserialized: NodeStatus = serde_yml::from_str(&yaml).unwrap();
    assert_eq!(deserialized, NodeStatus::Retrying);
}

#[test]
fn retrying_node_not_in_ready_set() {
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("upstream"), workflow_node("downstream")],
        edges: vec![super::super::PipelineEdge {
            source: "upstream".to_string(),
            target: "downstream".to_string(),
            edge_type: super::super::EdgeType::Always,
            mode: Default::default(),
            mapping: None,
        }],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([
        (
            "upstream".to_string(),
            NodeState {
                session_id: "workflow-1735689600000-upstream".to_string(),
                status: NodeStatus::Retrying,
                attempts: 1,
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
                retry_after: Some("2025-01-01T00:00:00Z".to_string()),
                retry_count: Some(1),
            },
        ),
        (
            "downstream".to_string(),
            NodeState {
                session_id: "workflow-1735689600000-downstream".to_string(),
                status: NodeStatus::Pending,
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
            },
        ),
    ]);

    let ready = compute_ready_set(&definition, &node_states);

    assert!(
        !ready.contains(&"downstream".to_string()),
        "downstream nodes must remain blocked while predecessors are retrying"
    );
}

#[test]
fn retrying_is_remaining_work() {
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600001-node".to_string(),
            status: NodeStatus::Retrying,
            attempts: 1,
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
            retry_after: Some("2025-01-01T00:00:00Z".to_string()),
            retry_count: Some(1),
        },
    )]);

    assert!(has_remaining_work(&node_states));
}

#[test]
fn retrying_pipeline_status_is_active() {
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600002-node".to_string(),
            status: NodeStatus::Retrying,
            attempts: 1,
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
            retry_after: Some("2025-01-01T00:00:00Z".to_string()),
            retry_count: Some(1),
        },
    )]);

    assert_eq!(compute_pipeline_status(&node_states), "active");
}
