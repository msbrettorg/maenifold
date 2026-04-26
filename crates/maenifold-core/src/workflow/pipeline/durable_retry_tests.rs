use std::collections::HashMap;

use serial_test::serial;

use crate::markdown::{delete_file, file_exists, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::util::json::OutputFormat;

use super::dispatch::dispatch_pipeline;
use super::executor_apply::apply_node_outcome;
use super::executor_orchestration::ExecutionCounts;
use super::executor_run::{FailurePhase, NodeOutcome};
use super::session_read::{frontmatter_value, read_pipeline_session};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode, RetryPolicy};

fn retrying_workflow_node(id: &str, max_attempts: u32) -> PipelineNode {
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
        retry_policy: Some(RetryPolicy {
            max_attempts,
            backoff_min_ms: 1_000,
            backoff_max_ms: 5_000,
            jitter: false,
        }),
    }
}

fn active_node_state(session_id: &str) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status: NodeStatus::Active,
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
    }
}

fn read_raw_node_states(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

fn set_retry_after(
    context: &crate::tools::ToolContext,
    session_id: &str,
    node_id: &str,
    retry_after: &str,
) {
    let (uri, mut frontmatter, content) =
        read_pipeline_session(context, session_id).expect("read pipeline");
    let mut node_states: HashMap<String, NodeState> =
        frontmatter_value(&frontmatter, "nodeStates").expect("node states");
    node_states
        .get_mut(node_id)
        .expect("node state")
        .retry_after = Some(retry_after.to_string());
    frontmatter.insert(
        "nodeStates".to_string(),
        serde_yml::to_value(node_states).expect("serialize node states"),
    );
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)
        .expect("write pipeline session");
}

fn retryable_failure(reason: String) -> NodeOutcome {
    NodeOutcome::Abandoned {
        reason,
        phase: FailurePhase::PreExecution,
        transient_error_kind: Some("timeout"),
        acp_session_id: None,
        thinking_session_id: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
        retry_attempts: 0,
        is_outer_timeout: false,
    }
}

#[test]
#[serial]
fn retry_policy_persists_count_and_retry_after_before_retry() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689605000-durable-retry";
    let definition = PipelineDefinition {
        nodes: vec![retrying_workflow_node("node", 2)],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        active_node_state("workflow-1735689605001-node"),
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    let mut counts = ExecutionCounts::default();
    apply_node_outcome(
        &context,
        session_id,
        "node",
        retryable_failure("transport failed before prompt".to_string()),
        &mut counts,
    )
    .expect("apply retryable outcome");

    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["node"];
    assert_eq!(state.status, NodeStatus::Retrying);
    assert_eq!(state.retry_count, Some(1));
    let retry_after = state.retry_after.as_deref().expect("retry_after");
    let retry_at = chrono::DateTime::parse_from_rfc3339(retry_after)
        .expect("retry_after timestamp")
        .with_timezone(&chrono::Utc);
    assert!(retry_at > chrono::Utc::now());
}

#[test]
#[serial]
fn retry_after_survives_restart_and_is_honored() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689606000-retry-after-honored";
    let definition = PipelineDefinition {
        nodes: vec![retrying_workflow_node("node", 2)],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let future_retry_after = (chrono::Utc::now() + chrono::Duration::seconds(60)).to_rfc3339();
    let mut state = active_node_state("workflow-1735689606001-node");
    state.status = NodeStatus::Retrying;
    state.attempts = 1;
    state.retry_after = Some(future_retry_after.clone());
    state.retry_count = Some(1);
    let node_states = HashMap::from([("node".to_string(), state)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    let raw = dispatch_pipeline(&context, session_id).expect("dispatch pipeline");
    let response: serde_json::Value = serde_json::from_str(&raw).expect("json response");

    assert_eq!(
        response["data"]["dispatched"].as_array().expect("array").len(),
        0
    );
    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["node"];
    assert_eq!(state.status, NodeStatus::Retrying);
    assert_eq!(
        state.retry_after.as_deref(),
        Some(future_retry_after.as_str())
    );
}

#[test]
#[serial]
fn dispatch_recovers_missing_retry_workflow_session() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689606500-missing-retry-session";
    let workflow_session_id = "workflow-1735689606501-node";
    let definition = PipelineDefinition {
        nodes: vec![retrying_workflow_node("node", 2)],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let past_retry_after = (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339();
    let mut state = active_node_state(workflow_session_id);
    state.status = NodeStatus::Retrying;
    state.attempts = 1;
    state.retry_after = Some(past_retry_after);
    state.retry_count = Some(1);
    let node_states = HashMap::from([("node".to_string(), state)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);
    let workflow_uri = workflow_session_uri(workflow_session_id);
    delete_file(&workflow_uri, &context.config).expect("delete workflow session");

    let raw = dispatch_pipeline(&context, session_id).expect("dispatch pipeline");
    let response: serde_json::Value = serde_json::from_str(&raw).expect("json response");

    assert_eq!(
        response["data"]["dispatched"].as_array().expect("array").len(),
        1
    );
    assert!(file_exists(&workflow_uri, &context.config).expect("file exists check"));
    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["node"];
    assert_eq!(state.status, NodeStatus::Active);
    assert_eq!(state.retry_after, None);
    assert_eq!(state.retry_count, Some(1));
}

#[test]
#[serial]
fn durable_retry_budget_cannot_exceed_retry_max_attempts() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689607000-retry-budget";
    let definition = PipelineDefinition {
        nodes: vec![retrying_workflow_node("node", 2)],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        active_node_state("workflow-1735689607001-node"),
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    for retry_number in 1..=2 {
        let mut counts = ExecutionCounts::default();
        apply_node_outcome(
            &context,
            session_id,
            "node",
            retryable_failure(format!("transient failure {retry_number}")),
            &mut counts,
        )
        .expect("apply retryable outcome");
        let node_states = read_raw_node_states(&context, session_id);
        assert_eq!(node_states["node"].retry_count, Some(retry_number));
        let past_retry_after = (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339();
        set_retry_after(&context, session_id, "node", &past_retry_after);
        dispatch_pipeline(&context, session_id).expect("dispatch retry");
    }

    let mut counts = ExecutionCounts::default();
    apply_node_outcome(
        &context,
        session_id,
        "node",
        retryable_failure("transient failure 3".to_string()),
        &mut counts,
    )
    .expect("apply exhausted retryable outcome");

    let node_states = read_raw_node_states(&context, session_id);
    let state = &node_states["node"];
    assert_eq!(state.status, NodeStatus::Stuck);
    assert_eq!(state.retry_count, Some(2));
    assert_eq!(state.retry_after, None);
}
