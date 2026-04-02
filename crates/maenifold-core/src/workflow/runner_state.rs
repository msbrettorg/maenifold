use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use serde_json::json;

use super::runner_helpers::parse_step_progress;
use super::{frontmatter_usize, read_workflow_definition, workflow_steps};

pub(super) fn init_workflow_session(
    context: &ToolContext,
    workflow_id: &str,
    existing_session_id: Option<String>,
) -> Result<(String, String)> {
    if let Some(sid) = existing_session_id {
        let text = super::handler::workflow(
            context,
            &json!({
                "sessionId": sid,
                "response": "Resuming pre-activated pipeline session"
            }),
        )?;
        return Ok((sid, text));
    }
    let text = super::handler::workflow(context, &json!({ "workflowId": workflow_id }))?;
    let sid = parse_session_id_from_creation(&text).ok_or_else(|| {
        MaenifoldError::Message("Workflow session ID missing from response".to_string())
    })?;
    Ok((sid, text))
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

fn parse_session_id_from_creation(text: &str) -> Option<String> {
    text.strip_prefix("Created session: ")
        .and_then(|rest| rest.split_whitespace().next())
        .filter(|id| id.starts_with("workflow-"))
        .map(str::to_string)
}
