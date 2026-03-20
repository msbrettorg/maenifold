use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    append_h2_section, extract_wikilinks, file_exists, read_markdown, write_markdown,
};
use crate::sessions::{load_help_doc, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;

use super::{
    ResumeWorkflowResult, append_workflow_queue, build_workflow_response_content,
    format_workflow_step, frontmatter_usize, handle_cancelled_workflow_status,
    handle_completed_workflow_status, handle_queue_exhaustion, maybe_resume_workflow_submachine,
    parse_workflow_identifiers, read_workflow_definition, read_workflow_queue,
    register_workflow_submachine, set_current_step, start_workflow_session,
    transition_to_next_workflow, view_workflow_session, workflow_steps,
};

// T-12.0: RTM NFR-1 keeps Workflow request dispatch and step advancement in a focused handler.

pub fn workflow(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "Workflow")
            .unwrap_or_else(|| "ERROR: Help file not found for Workflow".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let workflow_ids = parse_workflow_identifiers(payload.get("workflowId"))?;
    let session_id = payload.get("sessionId").and_then(Value::as_str);
    let response = payload.get("response").and_then(Value::as_str);
    let thoughts = payload.get("thoughts").and_then(Value::as_str);
    let status = payload.get("status").and_then(Value::as_str);
    let conclusion = payload.get("conclusion").and_then(Value::as_str);
    let view = payload
        .get("view")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let append = payload.get("append");
    let submachine_session_id = payload.get("submachineSessionId").and_then(Value::as_str);

    if workflow_ids.is_some() && session_id.is_some() {
        return Err(MaenifoldError::Message(format!(
            "Cannot provide both workflowId and sessionId together: workflowId={}, sessionId={}",
            payload
                .get("workflowId")
                .map(Value::to_string)
                .unwrap_or_default(),
            session_id.unwrap_or_default()
        )));
    }

    AssetManager::initialize_assets(&context.config)?;

    if let Some(workflow_ids) = workflow_ids {
        return start_workflow_session(context, &workflow_ids);
    }

    if let Some(session_id) = session_id {
        if view {
            return view_workflow_session(context, session_id);
        }
        if let Some(append) = append {
            return append_workflow_queue(context, session_id, append);
        }
        if let Some(response) = response {
            return continue_workflow_session(
                context,
                session_id,
                response,
                thoughts,
                status,
                conclusion,
                submachine_session_id,
            );
        }
    }

    // T-15.14: RTM FR-8 — C# throws InvalidOperationException; Rust must return Err, not Ok.
    Err(MaenifoldError::Message(
        "Invalid parameter combination.\n\n\
         VALID OPERATIONS:\n  \
         Start new workflow: workflowId only\n  \
         Continue session: sessionId + response\n  \
         View queue: sessionId + view=true\n  \
         Append to queue: sessionId + append\n\n\
         TIP: view=true and append both require a sessionId to identify which session to operate on."
            .to_string(),
    ))
}

fn continue_workflow_session(
    context: &ToolContext,
    session_id: &str,
    response: &str,
    thoughts: Option<&str>,
    status: Option<&str>,
    conclusion: Option<&str>,
    submachine_session_id: Option<&str>,
) -> Result<String> {
    if let Some(message) = validate_workflow_response_concepts(response, thoughts) {
        return Ok(message);
    }

    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} doesn't exist"
        )));
    }

    let (frontmatter, existing_content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;
    frontmatter =
        gate_workflow_session_status(context, &uri, session_id, frontmatter, &existing_content)?;

    match maybe_resume_workflow_submachine(context, &uri, frontmatter, &existing_content)? {
        ResumeWorkflowResult::Ready(updated_frontmatter) => frontmatter = updated_frontmatter,
        ResumeWorkflowResult::Waiting(message) => return Ok(message),
    }

    if let Some(submachine_session_id) = submachine_session_id.filter(|value| !value.is_empty()) {
        return register_workflow_submachine(
            context,
            &uri,
            frontmatter,
            response,
            thoughts,
            submachine_session_id,
        );
    }

    let mut current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let mut current_step = frontmatter_usize(&frontmatter, "currentStep");
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_id = queue
        .get(current_workflow_index)
        .ok_or_else(|| MaenifoldError::Message("Workflow queue is empty".to_string()))?;
    let workflow_value = read_workflow_definition(&context.config, current_workflow_id)?;
    let steps = workflow_steps(&workflow_value, current_workflow_id)?;

    append_h2_section(
        &uri,
        &format!("Step {}/{} Response", current_step + 1, steps.len()),
        &build_workflow_response_content(response, thoughts),
        &context.config,
    )?;

    current_step += 1;
    set_current_step(&mut frontmatter, current_step);
    let (_, existing_content, _) = read_markdown(&uri, &context.config)?;

    if status == Some("completed") {
        return handle_completed_workflow_status(
            context,
            &uri,
            frontmatter,
            current_step,
            conclusion,
        );
    }

    if status == Some("cancelled") {
        return handle_cancelled_workflow_status(
            context,
            &uri,
            frontmatter,
            &existing_content,
            current_step,
        );
    }

    if current_step >= steps.len() {
        current_workflow_index += 1;
        if current_workflow_index >= queue.len() {
            return handle_queue_exhaustion(context, &uri, frontmatter, current_step, conclusion);
        }

        return transition_to_next_workflow(
            context,
            &uri,
            frontmatter,
            &existing_content,
            current_workflow_index,
            &queue,
        );
    }

    advance_to_next_workflow_step(
        context,
        &uri,
        frontmatter,
        &existing_content,
        current_step,
        steps,
    )
}

fn gate_workflow_session_status(
    context: &ToolContext,
    uri: &str,
    session_id: &str,
    mut frontmatter: crate::markdown::Frontmatter,
    existing_content: &str,
) -> Result<crate::markdown::Frontmatter> {
    let status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .unwrap_or("active");

    match status {
        "pending" => Err(MaenifoldError::Message(format!(
            "Session {session_id} is pending — cannot continue until activated"
        ))),
        "completed" => Err(MaenifoldError::Message(format!(
            "Session {session_id} is already completed"
        ))),
        "abandoned" => Err(MaenifoldError::Message(format!(
            "Session {session_id} is abandoned — use restart to create a new attempt"
        ))),
        "ready" => {
            frontmatter.insert(
                "status".to_string(),
                YamlValue::String("active".to_string()),
            );
            write_markdown(uri, Some(&frontmatter), existing_content, &context.config)?;
            Ok(frontmatter)
        }
        _ => Ok(frontmatter),
    }
}

fn advance_to_next_workflow_step(
    context: &ToolContext,
    workflow_uri: &str,
    frontmatter: crate::markdown::Frontmatter,
    existing_content: &str,
    current_step: usize,
    steps: &[Value],
) -> Result<String> {
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;
    let next_step = steps
        .get(current_step)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&next_step)?;
    append_h2_section(
        workflow_uri,
        &format!("Step {}/{}", current_step + 1, steps.len()),
        &rendered_step,
        &context.config,
    )?;
    Ok(format!(
        "Step {}/{}\n\n{}",
        current_step + 1,
        steps.len(),
        rendered_step,
    ))
}

pub(crate) fn validate_workflow_response_concepts(
    response: &str,
    thoughts: Option<&str>,
) -> Option<String> {
    let total_concepts = extract_wikilinks(response).len()
        + thoughts.map(extract_wikilinks).unwrap_or_default().len();
    if total_concepts == 0 {
        return Some(
            "ERROR: Response or thoughts must contain at least one [[WikiLink]] in double brackets.\nExample: 'Implementing [[REST API]] using [[ASP.NET Core]]'\nThis ensures your workflow contributions build the knowledge graph.".to_string(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::markdown::{Frontmatter, read_markdown, write_markdown};
    use crate::util::json::OutputFormat;
    use serde_json::json;
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

    fn seed_workflow_session(
        context: &ToolContext,
        session_id: &str,
        status: Option<&str>,
    ) -> String {
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
}
