use crate::error::{MaenifoldError, Result};
use crate::sessions::load_help_doc;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};

use super::handler_crud::{
    create_pipeline, delete_pipeline, delete_pipeline_with_force, list_pipelines, view_pipeline,
};
use super::handler_lifecycle::handle_lifecycle_operation;
use super::handler_nodes::handle_node_operation;
use super::handler_surgery::handle_surgery_operation;
use super::quarantine::restore_pipeline_from_quarantine;

// T-45.6a: RTM-333 / RTM-334 / RTM-314 / RTM-315 / RTM-316 / RTM-317
// Pipeline create validates graph structure, persists pipeline state, and pre-creates node sessions.
#[allow(clippy::too_many_lines)]
pub fn pipeline(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "Pipeline")
            .unwrap_or_else(|| "ERROR: Help file not found for Pipeline".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    if payload
        .get("list")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return list_pipelines(context);
    }

    if let Some(session_id) = payload.get("sessionId").and_then(Value::as_str) {
        let session_id = normalize_pipeline_session_id(session_id)?;
        if let Some(result) = handle_surgery_operation(context, &session_id, payload)? {
            return Ok(result);
        }

        if payload
            .get("delete")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let force = match payload.get("force") {
                None | Some(Value::Null) => false,
                Some(Value::Bool(force)) => *force,
                Some(other) => {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "force must be a boolean, got {other}"
                    )));
                }
            };
            return if force {
                delete_pipeline_with_force(context, &session_id, true)
            } else {
                delete_pipeline(context, &session_id)
            };
        }

        if restore_requested(payload) {
            let trash_date = payload
                .get("fromTrashDate")
                .or_else(|| payload.get("trashDate"))
                .and_then(Value::as_str)
                .ok_or(MaenifoldError::MissingProperty("fromTrashDate"))?;
            let report = restore_pipeline_from_quarantine(context, &session_id, trash_date)?;
            let result = json!({
                "restoredSessionId": session_id,
                "restoredFiles": report.restored_files,
                "skippedFiles": report.skipped_files,
            });
            return Ok(match context.output_format {
                OutputFormat::Json => success(result),
                OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
            });
        }

        if let Some(result) = handle_lifecycle_operation(context, &session_id, payload)? {
            return Ok(result);
        }

        if payload
            .get("view")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return view_pipeline(context, &session_id);
        }

        if let Some(result) = handle_node_operation(context, &session_id, payload)? {
            return Ok(result);
        }
    }

    if payload.get("definition").is_some() {
        return create_pipeline(context, payload);
    }

    Err(MaenifoldError::Message(
        "Invalid parameter combination.\n\n\
         VALID OPERATIONS:\n  \
          Create pipeline: definition\n  \
          List pipelines: list=true\n  \
          Run pipeline: sessionId + run=true\n  \
          Cancel pipeline: sessionId + cancel=true\n  \
          Rewind pipeline: sessionId + rewind\n  \
           Delete pipeline: sessionId + delete=true\n  \
           Restore pipeline: sessionId + restore=true + fromTrashDate\n  \
          Join pipeline: sessionId + join\n  \
          Fork pipeline: sessionId + fork\n  \
          Graft pipeline: sessionId + graft\n  \
         View pipeline: sessionId + view=true\n  \
         Dispatch ready nodes: sessionId + dispatch=true\n  \
          Restart node: sessionId + nodeId + restart=true\n  \
          Update node status: sessionId + nodeId + status\n  \
          Learn tool usage: learn=true\n\n\
          TIP: lifecycle, topology, and view operations all require a sessionId to identify which pipeline session to inspect."
            .to_string(),
    ))
}

fn restore_requested(payload: &Value) -> bool {
    payload
        .get("restore")
        .or_else(|| payload.get("restoreFromQuarantine"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn normalize_pipeline_session_id(identifier: &str) -> Result<String> {
    let trimmed = identifier.trim();
    if trimmed.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "sessionId must not be empty".to_string(),
        ));
    }
    if let Some(path) = trimmed.strip_prefix("memory://pipelines/") {
        let Some(name) = path.rsplit('/').next().filter(|name| !name.is_empty()) else {
            return Err(MaenifoldError::InvalidArgument(format!(
                "pipeline URI has no session id: {identifier}"
            )));
        };
        return Ok(name.strip_suffix(".md").unwrap_or(name).to_string());
    }
    if trimmed.starts_with("memory://") {
        return Err(MaenifoldError::InvalidArgument(format!(
            "sessionId memory URI must point to memory://pipelines/: {identifier}"
        )));
    }
    Ok(trimmed.to_string())
}
