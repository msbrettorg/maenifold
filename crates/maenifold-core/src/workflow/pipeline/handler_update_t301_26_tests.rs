use std::collections::HashMap;

use crate::markdown::uri_to_path;
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;

use super::handler::pipeline;
use super::session_io::{
    frontmatter_value, pipeline_definition_from_frontmatter, read_pipeline_session,
};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{NodeState, NodeStatus, PipelineDefinition};

fn workflow_definition(prompt: &str) -> PipelineDefinition {
    serde_json::from_value(json!({
        "nodes": [{
            "id": "node-a",
            "type": "workflow",
            "workflowId": "test-workflow-a",
            "acpClient": "test-agent",
            "prompt": prompt
        }],
        "edges": []
    }))
    .expect("definition")
}

fn step_one_workflow_definition() -> PipelineDefinition {
    serde_json::from_value(json!({
        "nodes": [{
            "id": "step-1",
            "type": "workflow",
            "workflowId": "test-workflow-a",
            "acpClient": "test-agent",
            "prompt": "old workflow prompt"
        }],
        "edges": []
    }))
    .expect("definition")
}

fn step_one_tool_definition() -> PipelineDefinition {
    serde_json::from_value(json!({
        "nodes": [{
            "id": "step-1",
            "type": "tool",
            "toolName": "memory_status"
        }],
        "edges": []
    }))
    .expect("definition")
}

fn state(status: NodeStatus) -> NodeState {
    NodeState {
        session_id: "workflow-node-a-1778851690641".to_string(),
        status,
        attempts: 1,
        started_at: None,
        completed_at: None,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
        output: Some("kept".to_string()),
        output_artifact: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn step_one_completed_state() -> NodeState {
    NodeState {
        session_id: "workflow-step-1-1778851690641".to_string(),
        ..state(NodeStatus::Completed)
    }
}

fn seed_status(status: NodeStatus) -> (tempfile::TempDir, crate::tools::ToolContext) {
    let (tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let mut states = HashMap::new();
    states.insert("node-a".to_string(), state(status));
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-1778851690641",
        workflow_definition("old prompt"),
        states,
        4,
    );
    (tmp, context)
}

fn seed_step_one_completed() -> (tempfile::TempDir, crate::tools::ToolContext) {
    let (tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let mut states = HashMap::new();
    states.insert("step-1".to_string(), step_one_completed_state());
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-1778851690641",
        step_one_workflow_definition(),
        states,
        4,
    );
    (tmp, context)
}

fn update_payload(definition: PipelineDefinition) -> Value {
    json!({
        "sessionId": "pipeline-1778851690641",
        "definition": definition,
        "name": "renamed-pipeline",
        "description": "updated description",
        "update": true
    })
}

fn set_completed_frontmatter(context: &crate::tools::ToolContext) {
    let (uri, mut frontmatter, content) =
        read_pipeline_session(context, "pipeline-1778851690641").expect("read seeded pipeline");
    frontmatter.insert(
        "status".to_string(),
        serde_yml::Value::String("completed".to_string()),
    );
    frontmatter.insert(
        "cursor".to_string(),
        serde_yml::to_value(7u64).expect("yaml"),
    );
    frontmatter.insert(
        "first".to_string(),
        serde_yml::to_value(3u64).expect("yaml"),
    );
    frontmatter.insert("last".to_string(), serde_yml::to_value(9u64).expect("yaml"));
    crate::markdown::write_markdown(&uri, Some(&frontmatter), &content, &context.config)
        .expect("rewrite completed frontmatter");
}

fn pipeline_files(context: &crate::tools::ToolContext) -> Vec<std::path::PathBuf> {
    let root = context.config.memory_path.join("pipelines");
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files
}

#[test]
#[serial]
fn update_pipeline_t301_36_resets_state_when_node_type_changes() {
    let (_tmp, context) = seed_step_one_completed();
    let original_session_id = "workflow-step-1-1778851690641";

    pipeline(&context, &update_payload(step_one_tool_definition())).expect("update pipeline");
    let (_, frontmatter, _) =
        read_pipeline_session(&context, "pipeline-1778851690641").expect("read updated pipeline");
    let states: HashMap<String, NodeState> =
        frontmatter_value(&frontmatter, "nodeStates").expect("node states");
    let updated = states.get("step-1").expect("step-1 state");

    assert_eq!(
        updated.status,
        NodeStatus::Pending,
        "same-id node type changes must reset runtime status"
    );
    assert_ne!(
        updated.session_id, original_session_id,
        "same-id node type changes must allocate a fresh session id"
    );
}

#[test]
#[serial]
fn update_pipeline_t301_36_preserves_status_when_updating_completed_pipeline() {
    let (_tmp, context) = seed_status(NodeStatus::Completed);
    set_completed_frontmatter(&context);

    pipeline(&context, &update_payload(workflow_definition("new prompt"))).expect("update");
    let (_, frontmatter, _) =
        read_pipeline_session(&context, "pipeline-1778851690641").expect("read updated pipeline");

    assert_eq!(
        frontmatter.get("status").and_then(serde_yml::Value::as_str),
        Some("completed")
    );
    assert_eq!(
        frontmatter.get("cursor").and_then(serde_yml::Value::as_u64),
        Some(7)
    );
    assert_eq!(
        frontmatter.get("first").and_then(serde_yml::Value::as_u64),
        Some(3)
    );
    assert_eq!(
        frontmatter.get("last").and_then(serde_yml::Value::as_u64),
        Some(9)
    );
}

#[test]
#[serial]
fn update_pipeline_t301_26_overwrites_existing_session_in_place_no_new_file() {
    let (_tmp, context) = seed_status(NodeStatus::Pending);
    let original_uri = pipeline_session_uri("pipeline-1778851690641");
    let original_path = uri_to_path(&original_uri, &context.config.memory_path).expect("path");
    let before_files = pipeline_files(&context);

    let response = pipeline(&context, &update_payload(workflow_definition("new prompt")))
        .expect("update pipeline");
    let payload: Value = serde_json::from_str(&response).expect("json response");
    let updated_id = payload["data"]["pipelineId"]
        .as_str()
        .or_else(|| payload["data"]["updatedSessionId"].as_str());

    let after_files = pipeline_files(&context);
    let (_, frontmatter, _) =
        read_pipeline_session(&context, "pipeline-1778851690641").expect("read updated");
    let definition: PipelineDefinition =
        pipeline_definition_from_frontmatter(&frontmatter).expect("definition");

    assert_eq!(updated_id, Some("pipeline-1778851690641"));
    assert_eq!(
        before_files, after_files,
        "update must not create a new file"
    );
    assert!(original_path.exists(), "original pipeline file must remain");
    assert_eq!(
        frontmatter.get("name").and_then(serde_yml::Value::as_str),
        Some("renamed-pipeline")
    );
    assert_eq!(
        frontmatter
            .get("description")
            .and_then(serde_yml::Value::as_str),
        Some("updated description")
    );
    assert_eq!(definition.nodes[0].prompt(), Some("new prompt"));
}

#[test]
#[serial]
fn update_pipeline_t301_26_preserves_runtime_state_for_unchanged_nodes() {
    let (_tmp, context) = seed_status(NodeStatus::Completed);

    let response =
        pipeline(&context, &update_payload(workflow_definition("new prompt"))).expect("update");
    let payload: Value = serde_json::from_str(&response).expect("json response");
    let (_, frontmatter, _) =
        read_pipeline_session(&context, "pipeline-1778851690641").expect("read updated pipeline");
    let definition: PipelineDefinition =
        pipeline_definition_from_frontmatter(&frontmatter).expect("definition");
    let states: HashMap<String, NodeState> =
        frontmatter_value(&frontmatter, "nodeStates").expect("node states");

    assert_eq!(payload["data"]["pipelineId"], "pipeline-1778851690641");
    assert_eq!(definition.nodes[0].prompt(), Some("new prompt"));
    assert_eq!(
        states.get("node-a").map(|state| &state.status),
        Some(&NodeStatus::Completed),
        "unchanged node runtime status must survive update"
    );
}

#[test]
#[serial]
fn update_pipeline_t301_26_rejects_update_for_running_pipeline() {
    let (_tmp, context) = seed_status(NodeStatus::Active);

    let error = pipeline(&context, &update_payload(workflow_definition("new prompt")))
        .expect_err("running update should fail");

    assert!(
        error
            .to_string()
            .contains("pipeline is running; cancel before updating"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial]
fn update_pipeline_t301_26_rejects_update_for_unknown_session_id() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

    let error = pipeline(
        &context,
        &json!({
            "sessionId": "pipeline-bogus-12345",
            "definition": workflow_definition("new prompt"),
            "update": true
        }),
    )
    .expect_err("unknown update should fail");

    assert!(
        matches!(
            error,
            crate::error::MaenifoldError::MissingProperty(_)
                | crate::error::MaenifoldError::InvalidArgument(_)
                | crate::error::MaenifoldError::Message(_)
        ),
        "unexpected error: {error}"
    );
    assert!(
        error.to_string().contains("pipeline-bogus-12345")
            || error.to_string().contains("sessionId"),
        "error should identify the unknown session id: {error}"
    );
}
