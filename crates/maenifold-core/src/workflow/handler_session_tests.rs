use super::handler_session::continue_workflow_session;
use crate::config::Config;
use crate::markdown::{Frontmatter, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

struct ConfigResetGuard;

impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn workflow_test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    fs::create_dir_all(config.assets_path.join("workflows")).expect("create workflows dir");
    fs::write(
        config.assets_path.join("workflows/test-workflow.json"),
        serde_json::to_string_pretty(&json!({
            "steps": [
                {
                    "id": "step-1",
                    "name": "Step 1",
                    "description": "Investigate [[status]]"
                },
                {
                    "id": "step-2",
                    "name": "Step 2",
                    "description": "Confirm [[lifecycle]]"
                }
            ]
        }))
        .expect("serialize workflow"),
    )
    .expect("write workflow");
    (
        tmp,
        ToolContext::new(config, OutputFormat::Markdown),
        ConfigResetGuard,
    )
}

fn seed_workflow_session(context: &ToolContext, session_id: &str, status: Option<&str>) -> String {
    let uri = workflow_session_uri(session_id);
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Workflow Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    if let Some(status) = status {
        frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    }
    frontmatter.insert(
        "queue".to_string(),
        YamlValue::Sequence(vec![YamlValue::String("test-workflow".to_string())]),
    );
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );

    write_markdown(
        &uri,
        Some(&frontmatter),
        "## Step 1/2\n\n{\"id\":\"step-1\"}\n",
        &context.config,
    )
    .expect("write workflow session");
    uri
}

#[test]
#[serial(config)]
fn status_lifecycle_rejects_pending_session() {
    let (_tmp, context, _reset) = workflow_test_context();
    let session_id = "workflow-1735689600000-pending";
    seed_workflow_session(&context, session_id, Some("pending"));

    let error = continue_workflow_session(
        &context,
        session_id,
        "[[status]] response",
        None,
        None,
        None,
        None,
    )
    .expect_err("pending session should be rejected");

    assert_eq!(
        error.to_string(),
        format!("Session {session_id} is pending — cannot continue until activated")
    );
}

#[test]
#[serial(config)]
fn status_lifecycle_rejects_completed_session() {
    let (_tmp, context, _reset) = workflow_test_context();
    let session_id = "workflow-1735689600001-completed";
    seed_workflow_session(&context, session_id, Some("completed"));

    let error = continue_workflow_session(
        &context,
        session_id,
        "[[status]] response",
        None,
        None,
        None,
        None,
    )
    .expect_err("completed session should be rejected");

    assert_eq!(
        error.to_string(),
        format!("Session {session_id} is already completed")
    );
}

#[test]
#[serial(config)]
fn status_lifecycle_rejects_abandoned_session() {
    let (_tmp, context, _reset) = workflow_test_context();
    let session_id = "workflow-1735689600002-abandoned";
    seed_workflow_session(&context, session_id, Some("abandoned"));

    let error = continue_workflow_session(
        &context,
        session_id,
        "[[status]] response",
        None,
        None,
        None,
        None,
    )
    .expect_err("abandoned session should be rejected");

    assert_eq!(
        error.to_string(),
        format!("Session {session_id} is abandoned — use restart to create a new attempt")
    );
}

#[test]
#[serial(config)]
fn status_lifecycle_promotes_ready_session_to_active() {
    let (_tmp, context, _reset) = workflow_test_context();
    let session_id = "workflow-1735689600003-ready";
    let uri = seed_workflow_session(&context, session_id, Some("ready"));

    let result = continue_workflow_session(
        &context,
        session_id,
        "[[status]] response",
        None,
        None,
        None,
        None,
    )
    .expect("ready session should continue");

    assert!(
        result.starts_with("Step 2/2"),
        "expected continuation to advance to next step, got: {result}"
    );

    let (frontmatter, _, _) = read_markdown(&uri, &context.config).expect("read session");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
}

#[test]
#[serial(config)]
fn status_lifecycle_treats_missing_status_as_active() {
    let (_tmp, context, _reset) = workflow_test_context();
    let session_id = "workflow-1735689600004-legacy";
    let uri = seed_workflow_session(&context, session_id, None);

    let result = continue_workflow_session(
        &context,
        session_id,
        "[[status]] response",
        None,
        None,
        None,
        None,
    )
    .expect("legacy session should continue");

    assert!(
        result.starts_with("Step 2/2"),
        "expected continuation to advance to next step, got: {result}"
    );

    let (frontmatter, _, _) = read_markdown(&uri, &context.config).expect("read session");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        None,
        "legacy sessions should remain backward compatible without forcing a new status field"
    );
}
