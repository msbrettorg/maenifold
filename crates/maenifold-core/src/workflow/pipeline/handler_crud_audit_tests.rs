use crate::error::{MaenifoldError, Result};
use crate::markdown::{file_exists, uri_to_path};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use chrono::DateTime;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;

use super::audit;
use super::audit::AuditWriter;
use super::handler_crud::delete_pipeline_with_force;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

const TS: i64 = 1_735_689_600_000;

struct FailingAuditWriter;

impl AuditWriter for FailingAuditWriter {
    fn write_pipeline_delete(&self, _record: &audit::PipelineDeleteAuditRecord) -> Result<()> {
        Err(MaenifoldError::Message("audit writer failed".to_string()))
    }
}

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
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn seed_completed_pipeline(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> (String, String) {
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
        HashMap::from([("node".to_string(), state)]),
        1,
    );
    (pipeline_session_uri(session_id), workflow_uri)
}

fn absolute_path(context: &crate::tools::ToolContext, uri: &str) -> String {
    uri_to_path(uri, &context.config.memory_path)
        .expect("resolve uri")
        .canonicalize()
        .expect("canonical path")
        .to_string_lossy()
        .to_string()
}

fn audit_record_for_session(audit_jsonl: &str, session_id: &str) -> Value {
    audit_jsonl
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("audit line is json"))
        .find(|record| record["sessionId"] == json!(session_id))
        .expect("audit record for deleted pipeline")
}

#[test]
#[serial]
fn delete_pipeline_writes_audit_before_unlinking_pipeline_and_workflow_files() {
    let (_tmp, mut context, _guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = Some(30);
    let session_id = "pipeline-1735689600000-delete-audit-success";
    let (pipeline_uri, workflow_uri) = seed_completed_pipeline(&context, session_id);
    let pipeline_path = absolute_path(&context, &pipeline_uri);
    let workflow_path = absolute_path(&context, &workflow_uri);
    let audit_path = audit::pipeline_delete_audit_path(&context.config);

    let response = delete_pipeline_with_force(&context, session_id, true).expect("delete pipeline");
    let payload: Value = serde_json::from_str(&response).expect("json response");
    assert_eq!(payload["data"]["deletedSessionId"], json!(session_id));
    assert!(!file_exists(&pipeline_uri, &context.config).expect("pipeline check"));
    assert!(!file_exists(&workflow_uri, &context.config).expect("workflow check"));

    let audit_jsonl = std::fs::read_to_string(&audit_path).expect("durable audit JSONL exists");
    let record = audit_record_for_session(&audit_jsonl, session_id);
    assert_eq!(record["op"], json!("pipeline_delete"));
    assert_eq!(record["sessionId"], json!(session_id));
    assert_eq!(record["force"], json!(true));
    assert_eq!(record["files"], json!([workflow_path, pipeline_path]));
    assert!(
        record["requesterAgent"]
            .as_str()
            .is_some_and(|agent| !agent.is_empty())
    );
    assert!(record["requesterPid"].as_u64().is_some_and(|pid| pid > 0));
    let ts = record["ts"].as_str().expect("timestamp string");
    DateTime::parse_from_rfc3339(ts).expect("ISO 8601 timestamp");
}

#[test]
#[serial]
fn delete_pipeline_fails_closed_when_audit_destination_cannot_be_written() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-audit-fail-closed";
    let (pipeline_uri, workflow_uri) = seed_completed_pipeline(&context, session_id);
    let logs_path = context.config.maenifold_root.join("logs");
    std::fs::remove_dir_all(&logs_path).expect("remove temp logs dir");
    std::fs::write(&logs_path, b"not a directory").expect("make audit parent fail");
    let failing_record = audit::PipelineDeleteAuditRecord {
        op: "pipeline_delete",
        session_id: session_id.to_string(),
        force: true,
        files: vec![],
        requester_agent: "test-agent".to_string(),
        requester_pid: std::process::id(),
        ts: "2026-05-12T00:00:00Z".to_string(),
    };
    FailingAuditWriter
        .write_pipeline_delete(&failing_record)
        .expect_err("fixture writer must fail");

    let error = delete_pipeline_with_force(&context, session_id, true)
        .expect_err("audit write failure must abort delete");

    assert!(
        error.to_string().contains("audit"),
        "error should identify audit write failure, got: {error}"
    );
    assert!(file_exists(&pipeline_uri, &context.config).expect("pipeline remains"));
    assert!(file_exists(&workflow_uri, &context.config).expect("workflow remains"));
}
