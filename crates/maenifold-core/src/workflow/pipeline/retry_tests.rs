use std::collections::HashMap;

use serial_test::serial;

use crate::markdown::write_markdown;
use crate::util::json::OutputFormat;

use super::super::engine::{compute_pipeline_status, compute_ready_set, has_remaining_work};
use super::super::executor_nodes::{FailurePhase, NodeOutcome};
use super::super::session_read::{frontmatter_value, read_pipeline_session};
use super::super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
use super::*;

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

fn set_max_attempts(context: &crate::tools::ToolContext, session_id: &str, max_attempts: u32) {
    let (uri, mut frontmatter, content) =
        read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter.insert(
        "maxAttempts".to_string(),
        serde_yaml::to_value(max_attempts).expect("serialize max attempts"),
    );
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)
        .expect("write pipeline session");
}

#[test]
fn retrying_status_serializes_roundtrip() {
    let status = NodeStatus::Retrying;
    let yaml = serde_yaml::to_string(&status).unwrap();
    assert_eq!(yaml.trim(), "retrying");

    let deserialized: NodeStatus = serde_yaml::from_str(&yaml).unwrap();
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
            mapping: None,
        }],
    };
    let node_states = HashMap::from([
        (
            "upstream".to_string(),
            NodeState {
                session_id: "workflow-1735689600000-upstream".to_string(),
                status: NodeStatus::Retrying,
                attempts: 1,
                thinking_session_id: None,
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
                thinking_session_id: None,
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
            thinking_session_id: None,
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
            thinking_session_id: None,
            output: None,
            failure_reason: None,
            retry_after: Some("2025-01-01T00:00:00Z".to_string()),
            retry_count: Some(1),
        },
    )]);

    assert_eq!(compute_pipeline_status(&node_states), "active");
}

#[test]
#[serial]
fn pre_exec_failure_transitions_to_retrying() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689601000-pre-exec-retry";
    let original_session_id = "workflow-1735689601001-node".to_string();
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: original_session_id.clone(),
            status: NodeStatus::Active,
            attempts: 1,
            thinking_session_id: None,
            output: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    let mut counts = ExecutionCounts::default();
    apply_node_outcome(
        &context,
        session_id,
        "node",
        NodeOutcome::Abandoned {
            reason: "transport failed before prompt".to_string(),
            phase: FailurePhase::PreExecution,
        },
        &mut counts,
    )
    .expect("apply retryable outcome");

    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["node"];
    assert_eq!(state.status, NodeStatus::Retrying);
    assert_ne!(state.session_id, original_session_id);
    assert_eq!(counts.abandoned, 0);
}

#[test]
#[serial]
fn post_exec_failure_stays_abandoned() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689602000-post-exec-abandoned";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689602001-node".to_string(),
            status: NodeStatus::Active,
            attempts: 1,
            thinking_session_id: Some("thinking-1735689602001-node".to_string()),
            output: Some("stale output".to_string()),
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    let mut counts = ExecutionCounts::default();
    apply_node_outcome(
        &context,
        session_id,
        "node",
        NodeOutcome::Abandoned {
            reason: "prompt failed".to_string(),
            phase: FailurePhase::PostPrompt,
        },
        &mut counts,
    )
    .expect("apply abandoned outcome");

    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["node"];
    assert_eq!(state.status, NodeStatus::Abandoned);
    assert_eq!(state.failure_reason.as_deref(), Some("prompt failed"));
    assert_eq!(counts.abandoned, 1);
}

#[test]
#[serial]
fn max_attempts_exceeded_becomes_stuck() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689603000-max-attempts-stuck";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689603001-node".to_string(),
            status: NodeStatus::Active,
            attempts: 1,
            thinking_session_id: None,
            output: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);
    set_max_attempts(&context, session_id, 1);

    let mut counts = ExecutionCounts::default();
    apply_node_outcome(
        &context,
        session_id,
        "node",
        NodeOutcome::Abandoned {
            reason: "transport failed before prompt".to_string(),
            phase: FailurePhase::PreExecution,
        },
        &mut counts,
    )
    .expect("apply retryable outcome");

    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["node"];
    assert_eq!(state.status, NodeStatus::Stuck);
    assert_eq!(counts.abandoned, 0);
}

#[test]
#[serial]
fn retrying_node_state_has_retry_metadata() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689604000-retry-metadata";
    let original_session_id = "workflow-1735689604001-node".to_string();
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: original_session_id.clone(),
            status: NodeStatus::Active,
            attempts: 1,
            thinking_session_id: Some("thinking-1735689604001-node".to_string()),
            output: Some("stale output".to_string()),
            failure_reason: Some("stale failure".to_string()),
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    let mut counts = ExecutionCounts::default();
    apply_node_outcome(
        &context,
        session_id,
        "node",
        NodeOutcome::Abandoned {
            reason: "transport failed before prompt".to_string(),
            phase: FailurePhase::PreExecution,
        },
        &mut counts,
    )
    .expect("apply retryable outcome");

    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["node"];
    assert_eq!(state.status, NodeStatus::Retrying);
    assert_ne!(state.session_id, original_session_id);
    assert_eq!(state.output, None);
    assert_eq!(state.thinking_session_id, None);
    assert_eq!(state.failure_reason, None);
    let retry_after = state
        .retry_after
        .as_deref()
        .expect("retry_after should be set");
    chrono::DateTime::parse_from_rfc3339(retry_after).expect("retry_after should be ISO 8601");
    assert!(state.retry_count.expect("retry_count should be set") >= 1);
}
