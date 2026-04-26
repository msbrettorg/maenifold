use super::config_tools::ConfigMutationState;
use crate::error::{JSONRPC_METHOD_NOT_FOUND, MaenifoldError};
use crate::mcp::{config_tools, discovery_tools, gateway_handler::GatewayState};
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use rmcp::model::{CallToolResult, Content};
use serde_json::Value;
use std::sync::Arc;

pub(crate) async fn handle_tool_call(
    tool_name: String,
    arguments: Value,
    config_mutation: Option<ConfigMutationState>,
    gateway: Option<Arc<GatewayState>>,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    if config_tools::is_config_mutation_tool(&tool_name) {
        let Some(state) = config_mutation else {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Config mutation tool '{tool_name}' requires HTTP daemon config state"
            ))]));
        };
        return config_tools::handle_tool_call(state, &tool_name, arguments).await;
    }
    if discovery_tools::is_discovery_tool(&tool_name) {
        let Some(state) = config_mutation else {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Discovery tool '{tool_name}' requires HTTP daemon config state"
            ))]));
        };
        return discovery_tools::handle_tool_call(state, gateway.clone(), &tool_name, arguments)
            .await;
    }
    if let Some(gateway) = gateway
        && tool_name != "GetHelp"
        && tool_name != "ReadMcpResource"
    {
        if let Some(gateway_result) = gateway.handle_tool_call(&tool_name, arguments).await? {
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
    let result = tokio::task::spawn_blocking(move || {
        ToolRegistry::global().try_invoke(&tool_name_for_invoke, &arguments, OutputFormat::Json)
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
