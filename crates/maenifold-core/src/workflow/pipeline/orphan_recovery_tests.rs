use crate::markdown::{file_exists, read_markdown, write_markdown};
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serde_yml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;
use std::io::Write as _;

use super::test_support::pipeline_test_context;
use super::{
    NodeState, NodeStatus, PipelineDefinition, PipelineNode, lease, pipeline,
    session_read::read_pipeline_session, test_support::seed_pipeline_session_with_node_sessions,
};

const TS: i64 = 1_735_689_600_000;

fn tool_node(id: &str) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        display_label: None,
        tool_name: "GetConfig".to_string(),
        tool_args: None,
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        retry_policy: None,
    }
}

fn state(node_id: &str, status: NodeStatus) -> NodeState {
    NodeState {
        session_id: format!("workflow-{TS}-{node_id}"),
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
        output: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn definition(node_ids: &[&str]) -> PipelineDefinition {
    PipelineDefinition {
        nodes: node_ids.iter().map(|node_id| tool_node(node_id)).collect(),
        edges: vec![],
        inputs: None,
        template_id: None,
    }
}

fn states(specs: &[(&str, NodeStatus)]) -> HashMap<String, NodeState> {
    specs
        .iter()
        .map(|(node_id, status)| ((*node_id).to_string(), state(node_id, status.clone())))
        .collect()
}

fn seed_running_pipeline(
    context: &crate::tools::ToolContext,
    session_id: &str,
    node_states: HashMap<String, NodeState>,
) {
    let node_ids: Vec<&str> = node_states.keys().map(String::as_str).collect();
    seed_pipeline_session_with_node_sessions(
        context,
        session_id,
        definition(&node_ids),
        node_states,
        1,
    );
    set_pipeline_status(context, session_id, "running");
}

fn set_pipeline_status(context: &crate::tools::ToolContext, session_id: &str, status: &str) {
    let uri = pipeline_session_uri(session_id);
    let (frontmatter, content, _) =
        read_markdown(&uri, &context.config).expect("read pipeline markdown");
    let mut frontmatter = frontmatter.expect("pipeline frontmatter");
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)
        .expect("write pipeline status");
}

fn frontmatter_node_status(frontmatter: &crate::markdown::Frontmatter, node_id: &str) -> String {
    let node_states = frontmatter
        .get("nodeStates")
        .and_then(YamlValue::as_mapping)
        .expect("node states mapping");
    let node = node_states
        .get(YamlValue::String(node_id.to_string()))
        .and_then(YamlValue::as_mapping)
        .expect("node state mapping");
    node.get(YamlValue::String("status".to_string()))
        .and_then(YamlValue::as_str)
        .expect("node status")
        .to_string()
}

fn pipeline_status(payload: &Value, session_id: &str) -> String {
    payload["data"]["pipelines"]
        .as_array()
        .expect("pipelines array")
        .iter()
        .find(|pipeline| pipeline["pipelineId"] == Value::String(session_id.to_string()))
        .and_then(|pipeline| pipeline["status"].as_str())
        .expect("pipeline status")
        .to_string()
}

#[test]
#[serial]
fn cancel_pipeline_succeeds_when_lock_missing_and_status_running() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-orphan-cancel";
    seed_running_pipeline(
        &context,
        session_id,
        states(&[
            ("active-node", NodeStatus::Active),
            ("retrying-node", NodeStatus::Retrying),
            ("done-node", NodeStatus::Completed),
        ]),
    );

    let lock_path = lease::pipeline_lock_path(session_id, &context.config);
    assert!(
        !lock_path.exists(),
        "fixture must represent an orphan with no lock file"
    );

    let response = pipeline(
        &context,
        &json!({ "sessionId": session_id, "cancel": true }),
    )
    .expect("orphan cancel should succeed");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], Value::Bool(true));
    assert_eq!(payload["data"]["cancelledOrphan"], Value::Bool(true));
    assert_eq!(
        payload["data"]["cancelledNodes"],
        Value::Number(serde_json::Number::from(2))
    );

    let (_, frontmatter, _) = read_pipeline_session(&context, session_id).expect("read pipeline");
    assert_eq!(
        frontmatter_node_status(&frontmatter, "active-node"),
        "cancelled"
    );
    assert_eq!(
        frontmatter_node_status(&frontmatter, "retrying-node"),
        "cancelled"
    );
    assert_eq!(
        frontmatter_node_status(&frontmatter, "done-node"),
        "completed"
    );
}

#[test]
#[serial]
fn delete_pipeline_force_true_bypasses_active_work_check() {
    let (_tmp, mut context, _guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = Some(30);
    let session_id = "pipeline-1735689600000-orphan-force-delete";
    seed_running_pipeline(
        &context,
        session_id,
        states(&[("active-node", NodeStatus::Active)]),
    );

    let response = pipeline(
        &context,
        &json!({ "sessionId": session_id, "delete": true, "force": true }),
    )
    .expect("force delete should bypass active work");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], Value::Bool(true));
    assert_eq!(
        payload["data"]["deletedSessionId"],
        Value::String(session_id.to_string())
    );
    assert!(
        !file_exists(&pipeline_session_uri(session_id), &context.config)
            .expect("check pipeline deletion"),
        "force delete should remove the pipeline session"
    );
}

#[test]
#[serial]
fn delete_pipeline_force_false_default_still_rejects_active_work() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-orphan-force-default";
    seed_running_pipeline(
        &context,
        session_id,
        states(&[("active-node", NodeStatus::Active)]),
    );

    let explicit_false = pipeline(
        &context,
        &json!({ "sessionId": session_id, "delete": true, "force": false }),
    )
    .expect_err("force=false must reject active work");
    assert!(
        explicit_false
            .to_string()
            .contains("has active work; cancel it before deleting"),
        "expected active-work refusal, got: {explicit_false}"
    );

    let omitted = pipeline(
        &context,
        &json!({ "sessionId": session_id, "delete": true }),
    )
    .expect_err("omitted force must reject active work");
    assert!(
        omitted
            .to_string()
            .contains("has active work; cancel it before deleting"),
        "expected active-work refusal, got: {omitted}"
    );
}

#[test]
#[serial]
fn list_pipelines_marks_running_without_lock_as_abandoned() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let live_session_id = "pipeline-1735689600000-live-list";
    let orphan_session_id = "pipeline-1735689600000-orphan-list";
    seed_running_pipeline(
        &context,
        live_session_id,
        states(&[("live-node", NodeStatus::Active)]),
    );
    seed_running_pipeline(
        &context,
        orphan_session_id,
        states(&[("orphan-node", NodeStatus::Active)]),
    );

    let lock_path = lease::pipeline_lock_path(live_session_id, &context.config);
    std::fs::create_dir_all(lock_path.parent().expect("lock parent")).expect("create lock dir");
    let file = std::fs::File::create(&lock_path).expect("create lock file");
    let mut rw = fd_lock::RwLock::new(file);
    let mut guard = rw.try_write().expect("acquire pipeline lock");
    guard.write_all(b"test-holder").expect("write lock holder");

    let response = pipeline(&context, &json!({ "list": true })).expect("list pipelines");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(pipeline_status(&payload, live_session_id), "running");
    assert_eq!(pipeline_status(&payload, orphan_session_id), "abandoned");
}
