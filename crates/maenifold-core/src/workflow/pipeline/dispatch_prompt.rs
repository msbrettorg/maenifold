use crate::error::{MaenifoldError, Result};
use serde_json::json;

use super::PipelineNode;

pub(super) fn build_prompt_dispatch_response(
    node: &PipelineNode,
    session_id: &str,
) -> Result<serde_json::Value> {
    let agent = node.acp_client().ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline node '{}' of type '{}' does not have an agent",
            node.id(),
            node.node_type_name()
        ))
    })?;
    Ok(json!({
        "nodeId": node.id(),
        "sessionId": session_id,
        "type": node.node_type_name(),
        "agent": agent,
    }))
}
