use std::collections::HashMap;

use serial_test::serial;

use crate::util::json::OutputFormat;

use super::super::runner::{ActiveNodeSessionPersistence, active_node_session_callback};
use super::session_read::{frontmatter_value, read_pipeline_session};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode, persist_active_node_session};

fn active_node_state(session_id: &str) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status: NodeStatus::Active,
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
        retry_after: None,
        retry_count: None,
    }
}

fn workflow_definition(node_id: &str) -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![PipelineNode::Workflow {
            id: node_id.to_string(),
            display_label: None,
            workflow_id: "test-workflow-a".to_string(),
            acp_client: Some("test-agent".to_string()),
            prompt: Some("Run workflow".to_string()),
            cwd: None,
            timeout: None,
            agent_config: None,
            adopt: None,
            requires_role: None,
            retry_policy: None,
            join_policy: None,
        }],
        edges: vec![],
        inputs: None,
        template_id: None,
    }
}

fn thinking_definition(node_id: &str) -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![PipelineNode::SequentialThinking {
            id: node_id.to_string(),
            display_label: None,
            topic: "Plan active session persistence".to_string(),
            total_thoughts: 2,
            agent: "test-agent".to_string(),
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            cwd: None,
            timeout: None,
            agent_config: None,
            adopt: None,
            requires_role: None,
            retry_policy: None,
            join_policy: None,
        }],
        edges: vec![],
        inputs: None,
        template_id: None,
    }
}

fn frontmatter_node_state(
    context: &crate::tools::ToolContext,
    pipeline_session_id: &str,
    node_id: &str,
) -> NodeState {
    let (_, frontmatter, _) =
        read_pipeline_session(context, pipeline_session_id).expect("read pipeline session");
    let node_states: HashMap<String, NodeState> =
        frontmatter_value(&frontmatter, "nodeStates").expect("nodeStates frontmatter");
    node_states
        .get(node_id)
        .cloned()
        .expect("node state present")
}

#[tokio::test]
#[serial]
async fn pipeline_workflow_node_persists_acp_session_id_while_active() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let pipeline_session_id = "pipeline-active-workflow";
    let node_id = "workflow-node";
    let definition = workflow_definition(node_id);
    let node_states = HashMap::from([(
        node_id.to_string(),
        active_node_state("workflow-active-session"),
    )]);
    seed_pipeline_session_with_node_sessions(
        &context,
        pipeline_session_id,
        definition,
        node_states,
        1,
    );

    persist_active_node_session(
        &context,
        pipeline_session_id,
        node_id,
        Some("acp-workflow-active"),
        None,
        Some("gpt-test"),
    )
    .await
    .expect("persist active workflow ACP session id");

    let node_state = frontmatter_node_state(&context, pipeline_session_id, node_id);
    assert_eq!(node_state.status, NodeStatus::Active);
    assert_eq!(
        node_state.acp_session_id.as_deref(),
        Some("acp-workflow-active")
    );
    assert_eq!(node_state.model.as_deref(), Some("gpt-test"));
}

#[tokio::test]
#[serial]
async fn pipeline_thinking_node_persists_both_session_ids_while_active() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let pipeline_session_id = "pipeline-active-thinking";
    let node_id = "think-node";
    let definition = thinking_definition(node_id);
    let node_states = HashMap::from([(
        node_id.to_string(),
        active_node_state("workflow-active-thinking"),
    )]);
    seed_pipeline_session_with_node_sessions(
        &context,
        pipeline_session_id,
        definition,
        node_states,
        1,
    );

    persist_active_node_session(
        &context,
        pipeline_session_id,
        node_id,
        Some("acp-thinking-active"),
        Some("thinking-active-session"),
        Some("gpt-test"),
    )
    .await
    .expect("persist active thinking session ids");

    let node_state = frontmatter_node_state(&context, pipeline_session_id, node_id);
    assert_eq!(node_state.status, NodeStatus::Active);
    assert_eq!(
        node_state.acp_session_id.as_deref(),
        Some("acp-thinking-active")
    );
    assert_eq!(
        node_state.thinking_session_id.as_deref(),
        Some("thinking-active-session")
    );
}

#[tokio::test]
#[serial]
async fn pipeline_node_state_updates_when_acp_callback_fires_late() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let pipeline_session_id = "pipeline-late-acp-callback";
    let node_id = "think-node";
    let definition = thinking_definition(node_id);
    let node_states = HashMap::from([(
        node_id.to_string(),
        active_node_state("workflow-late-callback"),
    )]);
    seed_pipeline_session_with_node_sessions(
        &context,
        pipeline_session_id,
        definition,
        node_states,
        1,
    );

    let callback = active_node_session_callback(
        &context,
        ActiveNodeSessionPersistence {
            pipeline_session_id: pipeline_session_id.to_string(),
            node_id: node_id.to_string(),
            thinking_session_id: Some("thinking-from-seed".to_string()),
        },
        Some("gpt-late".to_string()),
    );

    callback("acp-from-late-callback".to_string())
        .await
        .expect("late ACP callback persists node state");

    let node_state = frontmatter_node_state(&context, pipeline_session_id, node_id);
    assert_eq!(
        node_state.acp_session_id.as_deref(),
        Some("acp-from-late-callback")
    );
    assert_eq!(
        node_state.thinking_session_id.as_deref(),
        Some("thinking-from-seed")
    );
    assert_eq!(node_state.model.as_deref(), Some("gpt-late"));
}
