use std::collections::HashMap;

use serde_json::{Value, json};
use serial_test::serial;

use super::acp_reaper::{
    clear_test_live_acp_sessions, set_test_live_acp_sessions, test_reaped_acp_sessions,
};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode, pipeline};
use crate::util::json::OutputFormat;

const TS: i64 = 1_735_689_600_000;

fn prompt_node(id: &str) -> PipelineNode {
    PipelineNode::Prompt {
        id: id.to_string(),
        display_label: None,
        prompt: format!("Run {id}"),
        agent: "test-agent".to_string(),
        tool_mode: "read-only".to_string(),
        model: Some("test-model".to_string()),
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
        session_ref: None,
        join_policy: None,
    }
}

fn thinking_node(id: &str) -> PipelineNode {
    PipelineNode::SequentialThinking {
        id: id.to_string(),
        display_label: None,
        topic: format!("Think {id}"),
        total_thoughts: 1,
        agent: "test-agent".to_string(),
        model: Some("test-model".to_string()),
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
    }
}

fn state(node_id: &str, acp_session_id: &str) -> NodeState {
    NodeState {
        session_id: format!("workflow-{TS}-{node_id}"),
        status: NodeStatus::Completed,
        attempts: 1,
        started_at: None,
        completed_at: None,
        acp_session_id: Some(acp_session_id.to_string()),
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

fn definition() -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![prompt_node("prompt-node"), thinking_node("thinking-node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    }
}

fn seed_acp_pipeline(context: &crate::tools::ToolContext, session_id: &str) {
    seed_pipeline_session_with_node_sessions(
        context,
        session_id,
        definition(),
        HashMap::from([
            (
                "prompt-node".to_string(),
                state("prompt-node", "acp-prompt-live"),
            ),
            (
                "thinking-node".to_string(),
                state("thinking-node", "acp-thinking-live"),
            ),
        ]),
        2,
    );
    super::orphan_recovery_tests::set_pipeline_status(context, session_id, "completed");
}

#[tokio::test]
#[serial]
async fn terminal_state_reaps_acp_sessions() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-terminal-reap";
    seed_acp_pipeline(&context, session_id);
    set_test_live_acp_sessions(&["acp-prompt-live", "acp-thinking-live"]);

    let (_, frontmatter, _) =
        super::session_read::read_pipeline_session(&context, session_id).expect("read pipeline");
    let definition: PipelineDefinition =
        serde_yml::from_value(frontmatter["definition"].clone()).expect("definition");
    let node_states: HashMap<String, NodeState> =
        serde_yml::from_value(frontmatter["nodeStates"].clone()).expect("node states");
    let reaped = super::executor_cleanup::terminate_node_sessions(&definition, &node_states).await;

    assert_eq!(reaped.acp_sessions, 2);
    assert_eq!(test_reaped_acp_sessions().len(), 2);
    clear_test_live_acp_sessions();
}

#[test]
#[serial]
fn cancel_with_lock_gone_still_reaps() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-lock-gone-reap";
    seed_acp_pipeline(&context, session_id);
    set_test_live_acp_sessions(&["acp-prompt-live"]);

    let response = pipeline(
        &context,
        &json!({ "sessionId": session_id, "cancel": true }),
    )
    .expect("cancel with lock gone should return cleanup result");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["data"]["cancelled"], Value::Bool(false));
    assert_eq!(payload["data"]["reaped"]["acpSessions"], json!(1));
    assert_eq!(payload["data"]["reaped"]["killedProcessGroups"], json!(0));
    clear_test_live_acp_sessions();
}

#[test]
#[serial]
fn lineage_surfaces_orphaned_workers_in_list() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-list-reap";
    seed_acp_pipeline(&context, session_id);
    set_test_live_acp_sessions(&["acp-thinking-live"]);

    let response = pipeline(&context, &json!({ "list": true })).expect("list pipelines");
    let payload: Value = serde_json::from_str(&response).expect("json response");
    let entry = payload["data"]["pipelines"]
        .as_array()
        .expect("pipelines array")
        .iter()
        .find(|pipeline| pipeline["pipelineId"] == session_id)
        .expect("pipeline entry");

    assert_eq!(entry["degraded"]["orphanedAcpWorkers"], json!(1));
    assert_eq!(entry["nodes"][1]["acpProcessAlive"], Value::Bool(true));
    clear_test_live_acp_sessions();
}
