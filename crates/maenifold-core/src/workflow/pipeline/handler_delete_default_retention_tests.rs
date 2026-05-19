use crate::markdown::{file_exists, uri_to_path};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::handler_crud::delete_pipeline_with_force;
use super::quarantine::run_quarantine_cleanup;
use super::session_delete_files::restore_pipeline_from_quarantine;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

const TS: i64 = 1_735_689_600_000;
const TRASH_DATE: &str = "2025-01-01";

#[rustfmt::skip]
fn workflow_node(id: &str) -> PipelineNode {
    PipelineNode::Workflow { id: id.to_string(), display_label: None, workflow_id: "test-workflow-a".to_string(), acp_client: Some(format!("agent-{id}")), prompt: None, cwd: None, timeout: None, agent_config: None, adopt: None, requires_role: None, retry_policy: None, join_policy: None, output_schema: None }
}

#[rustfmt::skip]
fn node_state(node_id: &str) -> NodeState {
    NodeState { session_id: format!("workflow-{TS}-{node_id}"), status: NodeStatus::Completed, attempts: 1, started_at: None, completed_at: None, acp_session_id: None, thinking_session_id: None, tool_provenance: None, input_tokens: None, output_tokens: None, model: None, output: None, output_artifact: None, failure_reason: None, retry_after: None, retry_count: None }
}

fn seed_pipeline(context: &crate::tools::ToolContext, session_id: &str) -> Vec<String> {
    let node_ids = ["default-retention-a", "default-retention-b"];
    let definition = PipelineDefinition {
        nodes: node_ids.iter().map(|id| workflow_node(id)).collect(),
        edges: vec![],
        inputs: None,
        input_schema: None,
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

fn quarantine_day_root(context: &crate::tools::ToolContext, session_id: &str) -> PathBuf {
    context
        .config
        .maenifold_root
        .join(".trash")
        .join(TRASH_DATE)
        .join(session_id)
}

fn dot_quarantine_day_root(context: &crate::tools::ToolContext) -> PathBuf {
    context
        .config
        .memory_path
        .join("pipelines")
        .join(".quarantine")
        .join("2025")
        .join("01")
        .join("01")
}

fn directory_has_entries(path: &Path) -> bool {
    path.exists()
        && std::fs::read_dir(path)
            .expect("read quarantine directory")
            .next()
            .is_some()
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
fn run_quarantine_cleanup_t301_35_succeeds_with_default_retention_when_setting_unset() {
    let (_tmp, mut context, _guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = None;

    let report = run_quarantine_cleanup(&context.config)
        .expect("cleanup without configured retention should use the 30-day default");

    assert!(report.deleted_files.is_empty());
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
    assert!(
        payload["data"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("30")
    );
    assert_eq!(payload["data"]["deletedSessionId"], session_id);
    assert_eq!(
        payload["data"]["deletedFiles"]
            .as_array()
            .expect("deleted files")
            .len(),
        uris.len()
    );
    for (uri, source) in uris.iter().zip(source_paths.iter()) {
        assert!(!file_exists(uri, &context.config).expect("source removed"));
        assert!(
            !source.exists(),
            "source path remains: {}",
            source.display()
        );
        assert_quarantined(&quarantine_day_root(&context, session_id), source);
    }
}

#[test]
#[serial]
fn quarantine_file_t301_34_does_not_write_to_dot_quarantine_path() {
    let (_tmp, mut context, _guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = Some(30);
    let session_id = "pipeline-1735689600000-t301-34-no-dot-quarantine";
    let uris = seed_pipeline(&context, session_id);
    let source_paths: Vec<PathBuf> = uris.iter().map(|uri| source_path(&context, uri)).collect();

    delete_pipeline_with_force(&context, session_id, true).expect("delete pipeline");

    for source in &source_paths {
        assert_quarantined(&quarantine_day_root(&context, session_id), source);
    }
    assert!(
        !directory_has_entries(&dot_quarantine_day_root(&context)),
        ".quarantine mirror path must stay empty"
    );
}

#[test]
#[serial]
fn restore_pipeline_t301_34_finds_files_in_trash_only() {
    let (_tmp, mut context, _guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = Some(30);
    let session_id = "pipeline-1735689600000-t301-34-restore-from-trash";
    let uris = seed_pipeline(&context, session_id);
    let source_paths: Vec<PathBuf> = uris.iter().map(|uri| source_path(&context, uri)).collect();
    let trash_paths: Vec<PathBuf> = source_paths
        .iter()
        .map(|source| {
            quarantine_day_root(&context, session_id)
                .join(source.file_name().expect("source file name"))
        })
        .collect();

    delete_pipeline_with_force(&context, session_id, true).expect("delete pipeline");
    for trash in &trash_paths {
        assert!(trash.is_file(), "trash seed missing: {}", trash.display());
    }
    let dot_quarantine = dot_quarantine_day_root(&context);
    if dot_quarantine.exists() {
        std::fs::remove_dir_all(&dot_quarantine).expect("remove non-canonical mirror");
    }

    let report = restore_pipeline_from_quarantine(&context, session_id, TRASH_DATE)
        .expect("restore from trash quarantine");

    assert_eq!(report.skipped_files, 0);
    assert_eq!(report.restored_files.len(), source_paths.len());
    for (source, trash) in source_paths.iter().zip(trash_paths.iter()) {
        let restored = source.display().to_string();
        assert!(
            report.restored_files.contains(&restored),
            "missing restored file {restored}"
        );
        assert!(
            source.is_file(),
            "source was not restored: {}",
            source.display()
        );
        assert!(
            !trash.exists(),
            "trash source should be consumed: {}",
            trash.display()
        );
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
        assert_quarantined(&quarantine_day_root(&context, session_id), source);
    }
}
