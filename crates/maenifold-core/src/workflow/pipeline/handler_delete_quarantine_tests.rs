use crate::markdown::{file_exists, uri_to_path};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;
use std::path::PathBuf;

use super::audit;
use super::handler::pipeline;
use super::handler_crud::delete_pipeline_with_force;
use super::session_delete_files::{
    delete_pipeline_session_files, restore_pipeline_from_quarantine, run_quarantine_cleanup,
};
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

const TS: i64 = 1_735_689_600_000;
const TRASH_DATE: &str = "2025-01-01";

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

fn context_with_retention() -> (
    tempfile::TempDir,
    crate::tools::ToolContext,
    crate::test_util::ConfigResetGuard,
) {
    let (tmp, mut context, guard) = pipeline_test_context(OutputFormat::Json);
    context.config.settings.quarantine_retention_days = Some(30);
    assert_eq!(
        context
            .config
            .quarantine_config()
            .expect("explicit retention config")
            .retention_days,
        30
    );
    (tmp, context, guard)
}

fn seed_pipeline(
    context: &crate::tools::ToolContext,
    session_id: &str,
    node_ids: &[&str],
) -> Vec<String> {
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

fn file_name(context: &crate::tools::ToolContext, uri: &str) -> String {
    uri_to_path(uri, &context.config.memory_path)
        .expect("resolve source path")
        .file_name()
        .expect("source file name")
        .to_string_lossy()
        .to_string()
}

fn quarantined(context: &crate::tools::ToolContext, session_id: &str, name: &str) -> PathBuf {
    context
        .config
        .maenifold_root
        .join(".trash")
        .join(TRASH_DATE)
        .join(session_id)
        .join(name)
}

#[test]
#[serial]
fn delete_moves_pipeline_files_to_quarantine_and_preserves_audit_jsonl() {
    let (_tmp, context, _guard) = context_with_retention();
    let session_id = "pipeline-1735689600000-quarantine-delete";
    let uris = seed_pipeline(&context, session_id, &["node-a", "node-b"]);

    let response = delete_pipeline_with_force(&context, session_id, true).expect("delete pipeline");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["data"]["deletedSessionId"], session_id);
    for uri in &uris {
        assert!(!file_exists(uri, &context.config).expect("source absent"));
        assert!(
            quarantined(&context, session_id, &file_name(&context, uri)).is_file(),
            "missing quarantined copy for {uri}"
        );
    }
    let audit_path = audit::pipeline_delete_audit_path(&context.config);
    let audit_jsonl = std::fs::read_to_string(audit_path).expect("audit JSONL exists");
    assert!(audit_jsonl.contains(session_id));
}

#[test]
#[serial]
fn delete_quarantine_collision_uses_deterministic_numeric_suffix() {
    let (_tmp, context, _guard) = context_with_retention();
    let session_id = "pipeline-1735689600000-quarantine-collision";
    let uris = seed_pipeline(&context, session_id, &["shared"]);
    let workflow_name = file_name(&context, &uris[0]);
    delete_pipeline_with_force(&context, session_id, true).expect("first delete");
    seed_pipeline(&context, session_id, &["shared"]);

    delete_pipeline_with_force(&context, session_id, true).expect("second delete");

    assert!(quarantined(&context, session_id, &workflow_name).is_file());
    assert!(quarantined(&context, session_id, &format!("{workflow_name}.1")).is_file());
}

#[test]
#[serial]
fn delete_quarantines_all_files_for_one_session_under_one_directory() {
    let (_tmp, context, _guard) = context_with_retention();
    let session_id = "pipeline-1735689600000-quarantine-single-directory";
    let uris = seed_pipeline(&context, session_id, &["one", "two", "three"]);

    delete_pipeline_with_force(&context, session_id, true).expect("delete pipeline");

    let expected_dir = context
        .config
        .maenifold_root
        .join(".trash")
        .join(TRASH_DATE)
        .join(session_id);
    assert!(expected_dir.is_dir(), "single quarantine directory missing");
    for uri in uris {
        let path = quarantined(&context, session_id, &file_name(&context, &uri));
        assert_eq!(path.parent(), Some(expected_dir.as_path()));
        assert!(path.is_file(), "missing {}", path.display());
    }
}

#[test]
#[serial]
fn delete_pipeline_session_files_fails_closed_without_retention_config() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-quarantine-no-retention";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let state = node_state("node");
    let states = HashMap::from([("node".to_string(), state.clone())]);
    let pipeline_uri = pipeline_session_uri(session_id);
    let workflow_uri = workflow_session_uri(&state.session_id);
    seed_pipeline_session_with_workflows(&context, session_id, definition, states.clone(), 1);

    let error = match delete_pipeline_session_files(&context, session_id, &states) {
        Ok(_) => panic!("missing retention must refuse quarantine"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("quarantine retention not configured"),
        "unexpected error: {error}"
    );
    assert!(file_exists(&pipeline_uri, &context.config).expect("pipeline remains"));
    assert!(file_exists(&workflow_uri, &context.config).expect("workflow remains"));
}

#[test]
#[serial]
fn quarantine_cleanup_scheduler_refuses_to_run_without_retention_config() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

    let error = match run_quarantine_cleanup(&context.config) {
        Ok(_) => panic!("scheduler must refuse missing retention config"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("quarantine retention not configured"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial]
fn restore_pipeline_from_quarantine_moves_files_back_to_original_locations() {
    let (_tmp, context, _guard) = context_with_retention();
    let session_id = "pipeline-1735689600000-quarantine-restore";
    let pipeline_uri = pipeline_session_uri(session_id);
    let pipeline_path = uri_to_path(&pipeline_uri, &context.config.memory_path).expect("source");
    let file_name = pipeline_path
        .file_name()
        .expect("file name")
        .to_string_lossy();
    let trash_path = quarantined(&context, session_id, &file_name);
    std::fs::create_dir_all(trash_path.parent().expect("trash parent")).expect("trash dir");
    std::fs::create_dir_all(pipeline_path.parent().expect("source parent")).expect("source dir");
    std::fs::write(&trash_path, "# quarantined pipeline\n").expect("trash file");
    assert!(!pipeline_path.is_file());

    let report = restore_pipeline_from_quarantine(&context, session_id, TRASH_DATE)
        .expect("restore from quarantine");

    assert_eq!(
        report.restored_files,
        vec![pipeline_path.display().to_string()]
    );
    assert_eq!(report.skipped_files, 0);
    assert!(pipeline_path.is_file(), "pipeline restored");
    assert!(!trash_path.exists(), "quarantine source consumed");
}

#[test]
#[serial]
fn restore_pipeline_from_quarantine_is_exposed_through_pipeline_gateway() {
    let (_tmp, context, _guard) = context_with_retention();
    let session_id = "pipeline-1735689600000-quarantine-gateway-restore";
    let pipeline_uri = pipeline_session_uri(session_id);
    let pipeline_path = uri_to_path(&pipeline_uri, &context.config.memory_path).expect("source");
    let file_name = pipeline_path
        .file_name()
        .expect("file name")
        .to_string_lossy();
    let trash_path = quarantined(&context, session_id, &file_name);
    std::fs::create_dir_all(trash_path.parent().expect("trash parent")).expect("trash dir");
    std::fs::write(&trash_path, "# quarantined pipeline\n").expect("trash file");

    let response = pipeline(
        &context,
        &json!({
            "restoreFromQuarantine": true,
            "sessionId": session_id,
            "trashDate": TRASH_DATE
        }),
    )
    .expect("gateway restore from quarantine");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["data"]["restoredSessionId"], session_id);
    assert!(pipeline_path.is_file(), "gateway restored pipeline");
    assert!(!trash_path.exists(), "gateway consumed quarantine file");
}
