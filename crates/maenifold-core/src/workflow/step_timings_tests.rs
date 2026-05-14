use super::handler_session::continue_workflow_session;
use super::queue::start_workflow_session;
use super::step_timings::read_step_timings;
use crate::config::Config;
use crate::markdown::read_markdown;
use crate::sessions::workflow_session_uri;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

fn context_with_workflow(workflow_json: &str) -> (TempDir, ToolContext, ConfigResetGuard) {
    let temp = TempDir::new().expect("temp dir");
    Config::override_root(temp.path());
    Config::set_database_path(temp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    std::fs::create_dir_all(config.assets_path.join("workflows")).expect("create workflows dir");
    std::fs::write(
        config.assets_path.join("workflows/test.json"),
        workflow_json,
    )
    .expect("write workflow");
    (
        temp,
        ToolContext::new(config, OutputFormat::Markdown),
        ConfigResetGuard,
    )
}

fn start_session(context: &ToolContext) -> String {
    let response =
        start_workflow_session(context, &["test".to_string()], None).expect("start workflow");
    let value: Value = serde_json::from_str(&response).expect("workflow start json");
    value
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("session id")
        .to_string()
}

fn timings_for(
    context: &ToolContext,
    session_id: &str,
) -> super::step_timings::WorkflowStepTimings {
    let uri = workflow_session_uri(session_id);
    let (frontmatter, _, _) = read_markdown(&uri, &context.config).expect("read session");
    read_step_timings(&frontmatter.expect("frontmatter"))
}

#[test]
#[serial]
fn workflow_start_records_step_one_started_at() {
    let (_temp, context, _guard) = context_with_workflow(r#"{"steps":[{"name":"Step 1"}]}"#);
    let session_id = start_session(&context);

    let timings = timings_for(&context, &session_id);

    assert!(
        timings
            .get("1")
            .and_then(|timing| timing.started_at.as_deref())
            .is_some(),
        "step 1 started_at should be recorded"
    );
}

#[test]
#[serial]
fn workflow_step_complete_records_ended_at_and_next_started_at() {
    let (_temp, context, _guard) =
        context_with_workflow(r#"{"steps":[{"name":"Step 1"},{"name":"Step 2"}]}"#);
    let session_id = start_session(&context);

    continue_workflow_session(
        &context,
        &session_id,
        "Completed step 1. [[workflow-execution]]",
        None,
        None,
        None,
        None,
        None,
    )
    .expect("continue workflow");
    let timings = timings_for(&context, &session_id);

    assert!(
        timings
            .get("1")
            .and_then(|timing| timing.ended_at.as_deref())
            .is_some(),
        "step 1 ended_at should be recorded"
    );
    assert!(
        timings
            .get("2")
            .and_then(|timing| timing.started_at.as_deref())
            .is_some(),
        "step 2 started_at should be recorded"
    );
}

#[test]
#[serial]
fn workflow_complete_records_final_step_ended_at() {
    let (_temp, context, _guard) = context_with_workflow(r#"{"steps":[{"name":"Only step"}]}"#);
    let session_id = start_session(&context);

    continue_workflow_session(
        &context,
        &session_id,
        "Completed final step. [[workflow-execution]]",
        None,
        Some("completed"),
        Some("Done. [[workflow-execution]]"),
        None,
        None,
    )
    .expect("complete workflow");
    let timings = timings_for(&context, &session_id);

    assert!(
        timings
            .get("1")
            .and_then(|timing| timing.ended_at.as_deref())
            .is_some(),
        "final step ended_at should be recorded"
    );
}
