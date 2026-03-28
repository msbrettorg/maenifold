use super::handler::pipeline;
use super::session_read::{frontmatter_value, workflow_status_to_node_status};
use super::session_write::{delete_pipeline_session_files, update_workflow_node_frontmatter};
use super::test_support::*;
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
use crate::markdown::{file_exists, read_markdown, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::thinking::sequential_session_uri;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

#[test]
fn pipeline_workflow_status_cancelled_maps_to_abandoned() {
    assert_eq!(
        workflow_status_to_node_status("cancelled"),
        Some(NodeStatus::Abandoned)
    );
}

#[test]
fn pipeline_workflow_status_unknown_returns_none() {
    assert_eq!(workflow_status_to_node_status("garbage"), None);
}

#[test]
#[serial(config)]
fn pipeline_view_rejects_missing_session() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-1735689600999", "view": true }),
    )
    .expect_err("missing session should fail");

    assert_eq!(
        error.to_string(),
        "Session pipeline-1735689600999 doesn't exist"
    );
}

#[test]
#[serial(config)]
fn pipeline_view_rejects_non_pipeline_session() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600001";
    seed_pipeline_view_session(&context, session_id, "workflow");

    let error = pipeline(&context, &json!({ "sessionId": session_id, "view": true }))
        .expect_err("non-pipeline session should fail");

    assert_eq!(
        error.to_string(),
        format!("Session {session_id} isn't a pipeline session")
    );
}

#[test]
#[serial(config)]
fn pipeline_resume_view_derives_state_from_workflow_sessions() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600002";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600002-root".to_string(),
                status: NodeStatus::Active,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600002-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
    update_workflow_node_frontmatter(
        &context,
        "workflow-1735689600002-root",
        "completed",
        Some("Completed [[root]]"),
    )
    .expect("mark workflow completed");

    let output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
        .expect("resume via view");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
    assert_eq!(
        data.get("ready").and_then(Value::as_array),
        Some(&vec![Value::String("child".to_string())])
    );
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    let root = nodes
        .iter()
        .find(|node| node.get("nodeId") == Some(&Value::String("root".to_string())))
        .expect("root node");
    assert_eq!(
        root.get("status").and_then(Value::as_str),
        Some("completed")
    );

    let (pipeline_fm, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
    let updated_states: HashMap<String, NodeState> =
        frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
    assert_eq!(updated_states["root"].status, NodeStatus::Active);

    let (workflow_fm, _, _) = read_markdown(
        &workflow_session_uri("workflow-1735689600002-root"),
        &context.config,
    )
    .expect("read workflow session");
    let workflow_fm = workflow_fm.expect("workflow frontmatter");
    assert_eq!(
        workflow_fm.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
}

#[test]
#[serial(config)]
fn delete_pipeline_session_files_removes_sequential_thinking_sessions() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600003";
    let workflow_session_id = "workflow-1735689600003-root".to_string();
    let thinking_session_id = "session-1735689600003-root".to_string();
    let node_states = HashMap::from([(
        "root".to_string(),
        NodeState {
            session_id: workflow_session_id.clone(),
            status: NodeStatus::Completed,
            attempts: 1,
            thinking_session_id: Some(thinking_session_id.clone()),
            output: None,
        },
    )]);
    let definition = PipelineDefinition {
        nodes: vec![PipelineNode::Workflow {
            id: "root".to_string(),
            workflow_id: "test-workflow-a".to_string(),
            acp_client: None,
            prompt: None,
        }],
        edges: vec![],
    };
    seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states.clone(), 1);

    let thinking_uri = sequential_session_uri(&thinking_session_id);
    write_markdown(
        &thinking_uri,
        None,
        "# Sequential Thinking Session\n",
        &context.config,
    )
    .expect("write sequential thinking session");
    assert!(file_exists(&thinking_uri, &context.config).expect("thinking session exists"));

    let result = delete_pipeline_session_files(&context, pipeline_id, &node_states)
        .expect("cleanup succeeds");

    assert_eq!(result.cleaned_up_files, 3);
    assert_eq!(result.skipped_files, 0);
    assert!(
        !file_exists(&pipeline_session_uri(pipeline_id), &context.config)
            .expect("pipeline removed")
    );
    assert!(
        !file_exists(&workflow_session_uri(&workflow_session_id), &context.config)
            .expect("workflow removed")
    );
    assert!(!file_exists(&thinking_uri, &context.config).expect("thinking removed"));
}
