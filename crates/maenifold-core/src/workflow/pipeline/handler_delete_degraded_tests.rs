use crate::error::MaenifoldError;
use crate::markdown::{file_exists, uri_to_path};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;

use super::handler::pipeline;
use super::handler_crud::delete_unreadable_pipeline;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
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
        output_schema: None,
    }
}

fn node_state(node_id: &str) -> NodeState {
    NodeState {
        session_id: format!("workflow-{TS}-{node_id}"),
        status: NodeStatus::Completed,
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
        output_artifact: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn seed_single_node_pipeline(context: &crate::tools::ToolContext, session_id: &str) -> String {
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let state = node_state("node");
    let workflow_uri = workflow_session_uri(&state.session_id);
    seed_pipeline_session_with_workflows(
        context,
        session_id,
        definition,
        std::collections::HashMap::from([("node".to_string(), state)]),
        1,
    );
    workflow_uri
}

#[test]
#[serial]
fn delete_pipeline_accepts_pipeline_uri_session_id() {
    let (_tmp, mut context, _guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = Some(30);
    let session_id = "pipeline-1735689600000-delete-uri-id";
    seed_single_node_pipeline(&context, session_id);

    let response = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_session_uri(session_id),
            "delete": true,
            "force": true,
        }),
    )
    .expect("delete by URI");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["data"]["deletedSessionId"], json!(session_id));
    assert!(
        !file_exists(&pipeline_session_uri(session_id), &context.config)
            .expect("check pipeline deletion"),
        "pipeline session should be deleted when addressed by memory URI"
    );
}

#[test]
#[serial]
fn force_delete_unreadable_pipeline_deletes_parent_only() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-dataless-parent";
    let workflow_uri = seed_single_node_pipeline(&context, session_id);
    let pipeline_uri = pipeline_session_uri(session_id);
    let pipeline_path = uri_to_path(&pipeline_uri, &context.config.memory_path).expect("path");
    let error = MaenifoldError::FileDataless {
        path: pipeline_path,
        size: 64,
    };

    let response = delete_unreadable_pipeline(&context, session_id, true, error)
        .expect("force delete unreadable pipeline");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], json!(true));
    assert_eq!(payload["data"]["cleanedUpFiles"], json!(1));
    assert_eq!(payload["data"]["degraded"]["reason"], "dataless");
    assert_eq!(
        payload["data"]["degraded"]["nodeCleanup"],
        "skipped_unreadable_parent"
    );
    assert!(
        !file_exists(&pipeline_uri, &context.config).expect("pipeline absent"),
        "unreadable parent pipeline file should be deleted"
    );
    assert!(
        file_exists(&workflow_uri, &context.config).expect("workflow preserved"),
        "node cleanup must not guess child sessions when parent is unreadable"
    );
}

#[test]
#[serial]
fn unreadable_pipeline_requires_force() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-dataless-no-force";
    seed_single_node_pipeline(&context, session_id);
    let pipeline_uri = pipeline_session_uri(session_id);
    let pipeline_path = uri_to_path(&pipeline_uri, &context.config.memory_path).expect("path");
    let error = MaenifoldError::FileDataless {
        path: pipeline_path,
        size: 64,
    };

    let error = delete_unreadable_pipeline(&context, session_id, false, error)
        .expect_err("non-force delete must refuse unreadable parent");

    assert!(
        error.to_string().contains("pass force:true"),
        "expected force guidance, got {error}"
    );
    assert!(
        file_exists(&pipeline_uri, &context.config).expect("pipeline remains"),
        "unreadable parent should remain without explicit force"
    );
}
