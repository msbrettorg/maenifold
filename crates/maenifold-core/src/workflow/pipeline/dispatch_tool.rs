use crate::error::Result;
use serde_json::json;

use super::PipelineNode;

pub(super) fn build_tool_dispatch_response(
    node: &PipelineNode,
    session_id: &str,
) -> Result<serde_json::Value> {
    let PipelineNode::Tool {
        tool_name,
        tool_args,
        ..
    } = node
    else {
        unreachable!("tool dispatch response requires tool node")
    };

    let mut response = json!({
        "nodeId": node.id(),
        "sessionId": session_id,
        "type": node.node_type_name(),
        "toolName": tool_name,
        "toolArgs": tool_args,
    });
    if let Some(agent) = node.acp_client() {
        response["agent"] = serde_json::Value::String(agent.to_string());
    }
    Ok(response)
}
