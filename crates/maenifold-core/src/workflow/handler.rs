use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::sessions::load_help_doc;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::Value;
use std::collections::HashMap;

use super::handler_session::continue_workflow_session;
use super::{
    append_workflow_queue, parse_workflow_identifiers, start_workflow_session,
    view_workflow_session,
};

// T-12.0: RTM NFR-1 keeps Workflow request dispatch in a focused handler entrypoint.

#[allow(clippy::too_many_lines)]
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
    let inputs = workflow_inputs(payload.get("inputs"))?;
    let submachine_session_id = payload.get("submachineSessionId").and_then(Value::as_str);
    let submachine_type = payload.get("submachineType").and_then(Value::as_str);

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
        return start_workflow_session(context, &workflow_ids, inputs);
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
                submachine_type,
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

fn workflow_inputs(value: Option<&Value>) -> Result<Option<HashMap<String, Value>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let object = value.as_object().ok_or_else(|| {
        MaenifoldError::InvalidArgument("Workflow inputs must be an object".to_string())
    })?;
    if object.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        object
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    ))
}
