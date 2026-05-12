use crate::markdown::{Frontmatter, delete_file, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::util::json::OutputFormat;
use serial_test::serial;
use std::collections::HashMap;

use super::session_read::{derive_pipeline_node_states, frontmatter_value, read_pipeline_session};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::update_restart::restart_pipeline_node;
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

const TS: i64 = 1_735_689_600_000;

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

fn states(specs: &[(&str, NodeStatus, u32, Option<&str>)]) -> HashMap<String, NodeState> {
    specs
        .iter()
        .map(|(node_id, status, attempts, output)| {
            (
                (*node_id).to_string(),
                NodeState {
                    session_id: format!("workflow-{TS}-{node_id}"),
                    status: status.clone(),
                    attempts: *attempts,
                    started_at: None,
                    completed_at: None,
                    acp_session_id: None,
                    thinking_session_id: None,
                    tool_provenance: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    output: output.map(str::to_string),
                    failure_reason: None,
                    retry_after: None,
                    retry_count: None,
                },
            )
        })
        .collect()
}

fn read_raw_node_states(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

fn read_workflow_frontmatter_and_content(
    context: &crate::tools::ToolContext,
    workflow_session_id: &str,
) -> (Frontmatter, String) {
    let (frontmatter, content, _) =
        read_markdown(&workflow_session_uri(workflow_session_id), &context.config)
            .expect("read workflow");
    (frontmatter.expect("workflow frontmatter"), content)
}

#[test]
#[serial]
fn orphan_cascade_loader_keeps_abandoned_pipeline_state_over_stale_active_workflow() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-orphan-cascade-loader-active";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("blocked")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = states(&[("blocked", NodeStatus::Abandoned, 1, None)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states.clone(), 2);

    let (mut frontmatter, content) =
        read_workflow_frontmatter_and_content(&context, "workflow-1735689600000-blocked");
    frontmatter.insert("status".to_string(), "active".into());
    write_markdown(
        &workflow_session_uri("workflow-1735689600000-blocked"),
        Some(&frontmatter),
        &content,
        &context.config,
    )
    .expect("write stale active workflow status");

    let derived = derive_pipeline_node_states(&context, &node_states).expect("derive states");
    assert_eq!(derived["blocked"].status, NodeStatus::Abandoned);
}

#[test]
#[serial]
fn orphan_cascade_restart_reload_errors_when_persisted_workflow_is_missing() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-orphan-cascade-restart-missing-workflow";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("restartable")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    seed_pipeline_session_with_workflows(
        &context,
        session_id,
        definition,
        states(&[("restartable", NodeStatus::Abandoned, 1, None)]),
        2,
    );

    restart_pipeline_node(&context, session_id, "restartable").expect("restart node");

    let raw_node_states = read_raw_node_states(&context, session_id);
    let restarted_session_id = raw_node_states["restartable"].session_id.clone();
    delete_file(
        &workflow_session_uri(&restarted_session_id),
        &context.config,
    )
    .expect("delete restarted workflow");

    let reload = derive_pipeline_node_states(&context, &raw_node_states);
    assert!(
        reload.is_err(),
        "reload should fail when restarted workflow is missing"
    );
}
