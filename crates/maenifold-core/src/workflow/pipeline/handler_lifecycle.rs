use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use serde_json::Value;

#[path = "handler_lifecycle_rewind.rs"]
mod rewind;

pub(super) fn handle_lifecycle_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<Option<String>> {
    if payload.get("rewind").is_some() {
        return rewind::handle_rewind_operation(context, session_id, payload).map(Some);
    }
    if payload.get("run").and_then(Value::as_bool).unwrap_or(false) {
        let prompt = payload.get("prompt").and_then(Value::as_str);
        return run_pipeline(context, session_id, prompt).map(Some);
    }
    if payload
        .get("pause")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return pause_pipeline(context, session_id).map(Some);
    }
    if payload
        .get("resume")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return resume_pipeline(context, session_id).map(Some);
    }
    if payload
        .get("cancel")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return cancel_pipeline(context, session_id).map(Some);
    }

    Ok(None)
}
fn run_pipeline(_context: &ToolContext, session_id: &str, _prompt: Option<&str>) -> Result<String> {
    Err(MaenifoldError::Message(format!(
        "Pipeline {session_id}: run=true requires an external pipeline driver; \
         use the pipeline-ui or `gate dispatch` to start autonomous execution"
    )))
}

fn pause_pipeline(_context: &ToolContext, session_id: &str) -> Result<String> {
    Err(MaenifoldError::Message(format!(
        "Pipeline {session_id}: pause requires an external pipeline driver; \
         the in-process supervisor has been removed"
    )))
}

fn resume_pipeline(_context: &ToolContext, session_id: &str) -> Result<String> {
    Err(MaenifoldError::Message(format!(
        "Pipeline {session_id}: resume requires an external pipeline driver; \
         the in-process supervisor has been removed"
    )))
}

fn cancel_pipeline(_context: &ToolContext, session_id: &str) -> Result<String> {
    Err(MaenifoldError::Message(format!(
        "Pipeline {session_id}: cancel requires an external pipeline driver; \
         the in-process supervisor has been removed"
    )))
}
