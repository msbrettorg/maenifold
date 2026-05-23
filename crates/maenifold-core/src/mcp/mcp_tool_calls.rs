use crate::error::{JSONRPC_METHOD_NOT_FOUND, MaenifoldError};
use crate::mcp::gateway_handler::GatewayState;
use crate::registry::ToolRegistry;
use crate::tools::{RequestCancelOnDrop, with_cancellation_scope};
use crate::util::json::OutputFormat;
use rmcp::RoleServer;
use rmcp::model::{CallToolResult, Content, Tool};
use rmcp::service::Peer;
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(crate) async fn handle_tool_call(
    tool_name: String,
    arguments: Value,
    gateway: Option<Arc<GatewayState>>,
    client_peer: Option<Peer<RoleServer>>,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    if tool_name == "complete_node" {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "complete_node accepted structured output: {arguments}"
        ))]));
    }
    if let Some(gateway) = gateway
        && tool_name != "GetHelp"
        && tool_name != "ReadMcpResource"
    {
        if let Some(gateway_result) = gateway
            .handle_tool_call(&tool_name, arguments, client_peer)
            .await?
        {
            return Ok(gateway_result);
        }
        return Err(unknown_tool_error(&tool_name));
    }
    invoke_native(tool_name, arguments).await
}

async fn invoke_native(
    tool_name: String,
    arguments: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let tool_name_for_invoke = tool_name.clone();
    let cancellation_token = CancellationToken::new();
    let _cancel_on_drop = RequestCancelOnDrop(cancellation_token.clone());
    let result = tokio::task::spawn_blocking(move || {
        with_cancellation_scope(cancellation_token, || {
            ToolRegistry::global().try_invoke(&tool_name_for_invoke, &arguments, OutputFormat::Json)
        })
    })
    .await
    .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

    match result {
        Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
        Err(MaenifoldError::UnknownTool(_)) => Err(unknown_tool_error(&tool_name)),
        Err(err) => Ok(CallToolResult::error(vec![Content::text(
            super::tool_utils::format_tool_error(&tool_name, &err),
        )])),
    }
}

fn unknown_tool_error(tool_name: &str) -> rmcp::ErrorData {
    rmcp::ErrorData::new(
        rmcp::model::ErrorCode(JSONRPC_METHOD_NOT_FOUND as i32),
        format!("Unknown tool: '{tool_name}'"),
        Some(Value::Null),
    )
}

pub(crate) fn complete_node_tool() -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "additionalProperties": true
    });
    let schema = match schema {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    Tool::new(
        "complete_node",
        "Submit the final structured output for this pipeline node. For nodes with output_schema, the prompt request advertises the node-specific input_schema; the pipeline will not advance until this tool is called with valid arguments.",
        Arc::new(schema),
    )
}
