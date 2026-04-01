use crate::error::{MaenifoldError, Result};
use crate::sessions::load_help_doc;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::Value;

use super::handler_crud::{create_pipeline, delete_pipeline, list_pipelines, view_pipeline};
use super::handler_lifecycle::handle_lifecycle_operation;
use super::handler_nodes::handle_node_operation;
use super::handler_surgery::handle_surgery_operation;

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
        if let Some(result) = handle_surgery_operation(context, session_id, payload)? {
            return Ok(result);
        }

        if payload
            .get("delete")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return delete_pipeline(context, session_id);
        }

        if let Some(result) = handle_lifecycle_operation(context, session_id, payload)? {
            return Ok(result);
        }

        if payload
            .get("view")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return view_pipeline(context, session_id);
        }

        if let Some(result) = handle_node_operation(context, session_id, payload)? {
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
          Pause pipeline: sessionId + pause=true\n  \
          Resume pipeline: sessionId + resume=true\n  \
          Cancel pipeline: sessionId + cancel=true\n  \
          Rewind pipeline: sessionId + rewind\n  \
          Delete pipeline: sessionId + delete=true\n  \
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
