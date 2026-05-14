use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use super::runner_helpers::parse_step_progress;
use super::{frontmatter_usize, read_workflow_definition, workflow_steps};

pub(super) fn init_workflow_session(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
    inputs: Option<HashMap<String, serde_json::Value>>,
) -> Result<(String, String)> {
    // T-306.8 Approach A: workflow creation returns a structured JSON payload, so
    // the runner reads `sessionId` from the payload instead of parsing prose.
    init_workflow_session_with_handler(
        context,
        workflow_id,
        existing_session_id,
        inputs,
        |context, payload| super::handler::workflow(context, payload),
    )
}

fn init_workflow_session_with_handler(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
    inputs: Option<HashMap<String, serde_json::Value>>,
    handler: impl Fn(&ToolContext, &serde_json::Value) -> Result<String>,
) -> Result<(String, String)> {
    if let Some(sid) = existing_session_id {
        let text = handler(
            context,
            &json!({
                "sessionId": sid,
                "response": "Resuming pre-activated pipeline session"
            }),
        )?;
        return Ok((sid, text));
    }
    let mut payload = json!({ "workflowId": workflow_id });
    if let Some(inputs) = inputs {
        payload["inputs"] = serde_json::to_value(inputs)?;
    }
    let text = handler(context, &payload)?;
    let sid = parse_session_id_from_start_response(&text)?;
    Ok((sid, text))
}

#[derive(Deserialize)]
struct WorkflowSessionStartResponse {
    #[serde(rename = "sessionId")]
    session_id: String,
}

fn parse_session_id_from_start_response(text: &str) -> Result<String> {
    let response: WorkflowSessionStartResponse = serde_json::from_str(text).map_err(|error| {
        MaenifoldError::Message(format!(
            "Workflow session ID missing from structured response: {error}"
        ))
    })?;
    if response.session_id.is_empty() {
        return Err(MaenifoldError::Message(
            "Workflow session ID missing from structured response".to_string(),
        ));
    }
    Ok(response.session_id)
}

pub(super) fn workflow_total_steps(
    context: &ToolContext,
    workflow_id: &str,
    step_text: &str,
) -> Result<usize> {
    if let Some((_, total)) = parse_step_progress(step_text) {
        return Ok(total as usize);
    }
    let workflow_value = read_workflow_definition(&context.config, workflow_id)?;
    Ok(workflow_steps(&workflow_value, workflow_id)?.len())
}

pub(super) fn current_step_index(context: &ToolContext, session_id: &str, step: &str) -> usize {
    if let Ok((Some(fm), _, _)) = read_markdown(&workflow_session_uri(session_id), &context.config)
    {
        let index = frontmatter_usize(&fm, "currentStep").saturating_add(1);
        if index > 0 {
            return index;
        }
    }
    if let Some((current, _)) = parse_step_progress(step) {
        let index = current as usize;
        if index > 0 {
            return index;
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config, test_util::ConfigResetGuard, tools::ToolContext, util::json::OutputFormat,
    };
    use serial_test::serial;
    use tempfile::TempDir;

    fn test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
        let tmp = TempDir::new().expect("temp dir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        std::fs::write(
            config.maenifold_root.join("maenifold.json"),
            r#"{"agents":{},"settings":{}}"#,
        )
        .expect("write config");
        (
            tmp,
            ToolContext::new(config, OutputFormat::Markdown),
            ConfigResetGuard,
        )
    }

    fn write_workflow_asset(context: &ToolContext, workflow_id: &str) {
        let path = context
            .config
            .assets_path
            .join("workflows")
            .join(format!("{workflow_id}.json"));
        std::fs::create_dir_all(path.parent().expect("workflow asset parent"))
            .expect("create workflow asset dir");
        std::fs::write(
            path,
            r#"{"id":"runner-state-test","steps":[{"name":"Step 1","description":"Run [[runner-state-test]]"}]}"#,
        )
        .expect("write workflow asset");
    }

    #[test]
    #[serial]
    fn init_workflow_session_returns_structured_session_id() {
        let (_tmp, context, _guard) = test_context();
        write_workflow_asset(&context, "runner-state-test");

        let (sid, text) =
            init_workflow_session(&context, "runner-state-test", None, None).expect("init session");

        assert!(sid.starts_with("workflow-"));
        assert!(
            sid["workflow-".len()..]
                .chars()
                .all(|ch| ch.is_ascii_digit())
        );
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("workflow start response should be JSON");
        assert_eq!(value["sessionId"].as_str(), Some(sid.as_str()));
    }

    #[test]
    #[serial]
    fn init_workflow_session_fails_cleanly_when_handler_returns_unexpected_shape() {
        let (_tmp, context, _guard) = test_context();
        let err = init_workflow_session_with_handler(
            &context,
            "runner-state-test",
            None,
            None,
            |_context, _payload| Ok(r#"{"message":"created"}"#.to_string()),
        )
        .expect_err("unexpected shape should fail");

        match err {
            MaenifoldError::Message(message) => {
                assert!(message.contains("Workflow session ID missing from structured response"));
            }
            other => panic!("expected message error, got {other:?}"),
        }
    }
}
