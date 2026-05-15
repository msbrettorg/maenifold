use crate::markdown::{file_exists, uri_to_path};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::handler_crud::delete_pipeline_with_force;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

const TS: i64 = 1_735_689_600_000;

#[rustfmt::skip]
fn workflow_node(id: &str) -> PipelineNode {
    PipelineNode::Workflow { id: id.to_string(), display_label: None, workflow_id: "test-workflow-a".to_string(), acp_client: Some(format!("agent-{id}")), prompt: None, cwd: None, timeout: None, agent_config: None, adopt: None, requires_role: None, retry_policy: None, join_policy: None }
}

#[rustfmt::skip]
fn node_state(node_id: &str) -> NodeState {
    NodeState { session_id: format!("workflow-{TS}-{node_id}"), status: NodeStatus::Completed, attempts: 1, started_at: None, completed_at: None, acp_session_id: None, thinking_session_id: None, tool_provenance: None, input_tokens: None, output_tokens: None, model: None, output: None, failure_reason: None, retry_after: None, retry_count: None }
}

fn seed_pipeline(context: &crate::tools::ToolContext, session_id: &str) -> Vec<String> {
    let node_ids = ["default-retention-a", "default-retention-b"];
    let definition = PipelineDefinition {
        nodes: node_ids.iter().map(|id| workflow_node(id)).collect(),
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let states: HashMap<String, NodeState> = node_ids
        .iter()
        .map(|id| ((*id).to_string(), node_state(id)))
        .collect();
    let uris = states
        .values()
        .map(|state| workflow_session_uri(&state.session_id))
        .chain(std::iter::once(pipeline_session_uri(session_id)))
        .collect();
    seed_pipeline_session_with_workflows(context, session_id, definition, states, 1);
    uris
}

fn source_path(context: &crate::tools::ToolContext, uri: &str) -> PathBuf {
    uri_to_path(uri, &context.config.memory_path).expect("resolve source path")
}

fn quarantine_day_root(context: &crate::tools::ToolContext) -> PathBuf {
    context
        .config
        .memory_path
        .join("pipelines")
        .join(".quarantine")
        .join("2025")
        .join("01")
        .join("01")
}

fn assert_quarantined(day_root: &Path, source: &Path) {
    let file_name = source.file_name().expect("source file name");
    assert!(
        day_root.join(file_name).is_file(),
        "missing quarantined copy for {}",
        source.display()
    );
}

#[test]
#[serial]
fn delete_t301_27_defaults_missing_retention_to_30_days_and_quarantines_files() {
    let (_tmp, mut context, _guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = None;
    let session_id = "pipeline-1735689600000-t301-27-default-retention";
    let uris = seed_pipeline(&context, session_id);
    let source_paths: Vec<PathBuf> = uris.iter().map(|uri| source_path(&context, uri)).collect();

    let response = delete_pipeline_with_force(&context, session_id, true)
        .expect("delete without configured retention should use the 30-day default");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], json!(true));
    assert_eq!(payload["data"]["defaultedRetention"], json!(true));
    assert_eq!(payload["data"]["retentionDays"], json!(30));
    assert!(payload["data"]["message"].as_str().unwrap_or_default().contains("30"));
    assert_eq!(payload["data"]["deletedSessionId"], session_id);
    assert_eq!(payload["data"]["deletedFiles"].as_array().expect("deleted files").len(), uris.len());
    for (uri, source) in uris.iter().zip(source_paths.iter()) {
        assert!(!file_exists(uri, &context.config).expect("source removed"));
        assert!(!source.exists(), "source path remains: {}", source.display());
        assert_quarantined(&quarantine_day_root(&context), source);
    }
}

#[test]
#[serial]
fn delete_t301_27_explicit_retention_does_not_report_defaulted_retention() {
    let (_tmp, mut context, _guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = Some(7);
    let session_id = "pipeline-1735689600000-t301-27-explicit-retention";
    let uris = seed_pipeline(&context, session_id);
    let source_paths: Vec<PathBuf> = uris.iter().map(|uri| source_path(&context, uri)).collect();

    let response = delete_pipeline_with_force(&context, session_id, true)
        .expect("delete with explicit retention should succeed");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], json!(true));
    assert_ne!(payload["data"]["defaultedRetention"], json!(true));
    assert_eq!(payload["data"]["retentionDays"], json!(7));
    for (uri, source) in uris.iter().zip(source_paths.iter()) {
        assert!(!file_exists(uri, &context.config).expect("source removed"));
        assert_quarantined(&quarantine_day_root(&context), source);
    }
}
