use crate::error::{JSONRPC_METHOD_NOT_FOUND, MaenifoldError};
use crate::mcp::gateway_runtime::GatewayState;
use crate::registry::ToolRegistry;
use crate::tools::{RequestCancelOnDrop, with_cancellation_scope};
use crate::util::json::OutputFormat;
use crate::util::json::success;
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
    if tool_name == "ReadMcpResource"
        && let Some(uri) = arguments.get("uri").and_then(Value::as_str)
        && super::resources::is_gateway_uri(uri)
    {
        return handle_gateway_public_resource_read(uri, gateway, true).await;
    }
    if tool_name == "ReadMcpResource"
        && let Some(uri) = arguments.get("uri").and_then(Value::as_str)
        && gateway
            .as_ref()
            .is_some_and(|gateway| gateway.owns_resource_uri(uri))
    {
        return handle_gateway_public_resource_read(uri, gateway, false).await;
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

async fn handle_gateway_public_resource_read(
    uri: &str,
    gateway: Option<Arc<GatewayState>>,
    static_gateway_uri: bool,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let Some(gateway) = gateway else {
        return Ok(CallToolResult::error(vec![Content::text(format!(
            "Error invoking ReadMcpResource: gateway resource '{uri}' is unavailable because gateway state is not initialized"
        ))]));
    };
    let result = if static_gateway_uri {
        super::resources::handle_gateway_read_resource(uri, Some(gateway.pool()))
            .await
            .map_err(super::resources::map_resource_error)
    } else {
        gateway.read_resource(uri).await
    };
    match result {
        Ok(result) => Ok(CallToolResult::success(vec![Content::text(success(
            first_text_content(&result)?,
        ))])),
        Err(err) => Ok(CallToolResult::error(vec![Content::text(format!(
            "Error invoking ReadMcpResource: {}",
            err
        ))])),
    }
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

fn first_text_content(
    result: &rmcp::model::ReadResourceResult,
) -> std::result::Result<String, rmcp::ErrorData> {
    result
        .contents
        .first()
        .and_then(|content| match content {
            rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            rmcp::ErrorData::internal_error("Gateway resource read returned non-text content", None)
        })
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
