use crate::error::Result;
use crate::tools::ToolContext;
use serde_json::Value;

use super::dispatch::dispatch_pipeline;
use super::update::{restart_pipeline_node, update_pipeline_node_status};

pub(super) fn handle_node_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<Option<String>> {
    if payload
        .get("dispatch")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return dispatch_pipeline(context, session_id).map(Some);
    }

    let node_id = payload.get("nodeId").and_then(Value::as_str);
    if payload
        .get("restart")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && let Some(node_id) = node_id
    {
        return restart_pipeline_node(context, session_id, node_id).map(Some);
    }

    let status = payload.get("status").and_then(Value::as_str);
    if let (Some(node_id), Some(status)) = (node_id, status)
        && matches!(status, "completed" | "abandoned")
    {
        let conclusion = payload.get("conclusion").and_then(Value::as_str);
        return update_pipeline_node_status(
            context, session_id, node_id, status, conclusion, None, None, None, None, None, None,
            None,
        )
        .map(Some);
    }

    Ok(None)
}
