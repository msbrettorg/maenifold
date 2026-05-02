use crate::error::MaenifoldError;
use crate::markdown::{file_exists, uri_to_path};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::Value;
use serial_test::serial;
use std::collections::HashMap;

use super::handler_crud::delete_pipeline;
use super::session_write_files::delete_file_verified_with;
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
    }
}

fn states(specs: &[(&str, NodeStatus, u32)]) -> HashMap<String, NodeState> {
    specs
        .iter()
        .map(|(node_id, status, attempts)| {
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
                    output: None,
                    failure_reason: None,
                    retry_after: None,
                    retry_count: None,
                },
            )
        })
        .collect()
}

#[test]
#[serial]
fn delete_pipeline_returns_actual_file_paths() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-files-list";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states.clone(), 1);

    let workflow_uri = workflow_session_uri(&node_states["node"].session_id);
    let pipeline_uri = pipeline_session_uri(session_id);
    let mut expected = vec![
        uri_to_path(&workflow_uri, &context.config.memory_path)
            .expect("workflow path")
            .canonicalize()
            .expect("workflow canonical path")
            .to_string_lossy()
            .to_string(),
        uri_to_path(&pipeline_uri, &context.config.memory_path)
            .expect("pipeline path")
            .canonicalize()
            .expect("pipeline canonical path")
            .to_string_lossy()
            .to_string(),
    ];
    expected.sort();

    let response = delete_pipeline(&context, session_id).expect("delete pipeline");
    let payload: Value = serde_json::from_str(&response).expect("json response");
    let mut actual: Vec<String> = payload["data"]["deletedFiles"]
        .as_array()
        .expect("deletedFiles array")
        .iter()
        .map(|value| value.as_str().expect("deleted path").to_string())
        .collect();
    actual.sort();

    assert_eq!(actual, expected);
    assert_eq!(
        payload["data"]["cleanedUpFiles"],
        Value::Number(serde_json::Number::from(expected.len())),
    );
    assert!(
        !file_exists(&workflow_uri, &context.config).expect("workflow absent"),
        "workflow session should be absent after verified deletion"
    );
    assert!(
        !file_exists(&pipeline_uri, &context.config).expect("pipeline absent"),
        "pipeline session should be absent after verified deletion"
    );
}

#[test]
#[serial]
fn delete_pipeline_aborts_when_post_verification_fails() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-verify-fails";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);

    let uri = pipeline_session_uri(session_id);
    let error = delete_file_verified_with(&uri, &context.config, |_, _| Ok(true))
        .expect_err("verification must fail when file still resolves");

    match error {
        MaenifoldError::DeleteVerificationFailed { path } => {
            assert!(
                path.ends_with("pipeline-1735689600000-delete-verify-fails.md"),
                "error should name the unresolved deleted file, got {}",
                path.display()
            );
        }
        other => panic!("expected DeleteVerificationFailed, got {other}"),
    }
}
