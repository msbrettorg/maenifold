// Discovery tool: enumerate MCP servers + their tools (used by the Pipelines tab).
// Reads on-disk maenifold.json directly; settings are owned by the desktop, never by MCP.
use crate::acp_config::AgentsConfig;
use crate::api::mcp_discovery;
use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp::gateway_handler::GatewayState;
use rmcp::model::{CallToolResult, Content, Prompt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::Arc;

const DISCOVERY_TOOLS: &[&str] = &["DiscoverMcpTools", "DiscoverMcpPrompts", "GetMcpPrompt"];

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct DiscoverMcpToolsParams {}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct DiscoverMcpPromptsParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetMcpPromptParams {
    pub(crate) server: String,
    pub(crate) name: String,
    pub(crate) arguments: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct McpPromptDiscoveryResult {
    server: String,
    prompts: Vec<Prompt>,
}

pub(crate) fn is_discovery_tool(tool_name: &str) -> bool {
    DISCOVERY_TOOLS.contains(&tool_name)
}

pub(crate) async fn handle_tool_call(
    gateway: Option<Arc<GatewayState>>,
    tool_name: &str,
    arguments: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    handle_gateway_tool_call(gateway.as_deref(), tool_name, arguments).await
}

pub(crate) async fn handle_gateway_tool_call(
    gateway: Option<&GatewayState>,
    tool_name: &str,
    arguments: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    match tool_name {
        "DiscoverMcpTools" => {
            let _params: DiscoverMcpToolsParams =
                serde_json::from_value(arguments).map_err(invalid_params)?;
            let results = discover_mcp_tools(gateway).await;
            let output = serde_json::to_string_pretty(&results).map_err(internal_error)?;
            Ok(CallToolResult::success(vec![Content::text(output)]))
        }
        "DiscoverMcpPrompts" => {
            let _params: DiscoverMcpPromptsParams =
                serde_json::from_value(arguments).map_err(invalid_params)?;
            let results = discover_mcp_prompts(gateway).await;
            let output = serde_json::to_string_pretty(&results).map_err(internal_error)?;
            Ok(CallToolResult::success(vec![Content::text(output)]))
        }
        "GetMcpPrompt" => {
            let params: GetMcpPromptParams =
                serde_json::from_value(arguments).map_err(invalid_params)?;
            let result = get_mcp_prompt(gateway, params).await?;
            let output = serde_json::to_string_pretty(&result).map_err(internal_error)?;
            Ok(CallToolResult::success(vec![Content::text(output)]))
        }
        _ => Err(rmcp::ErrorData::invalid_params(
            format!("Unknown discovery tool: {tool_name}"),
            Some(json!(null)),
        )),
    }
}

async fn discover_mcp_tools(
    gateway: Option<&GatewayState>,
) -> Vec<mcp_discovery::McpServerDiscoveryResult> {
    let config =
        AgentsConfig::load_or_default(&Config::current().maenifold_root).unwrap_or_default();
    let mut results = Vec::new();
    let mut pool_server_names = std::collections::HashSet::new();

    if let Some(gateway) = gateway {
        pool_server_names.insert("maenifold".to_string());
        results.push(native_maenifold_discovery_result(gateway));
        for (server_name, tools) in gateway.pool().all_gateway_tools() {
            pool_server_names.insert(server_name.clone());
            if server_name == "maenifold" {
                continue;
            }
            let tools = tools
                .into_iter()
                .map(|tool| mcp_discovery::McpDiscoveredTool {
                    name: format!("{server_name}.{}", tool.name),
                    description: tool.description.as_ref().map(ToString::to_string),
                    input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
                })
                .collect();
            results.push(mcp_discovery::McpServerDiscoveryResult {
                server: server_name,
                server_type: "stdio".to_string(),
                status: mcp_discovery::McpDiscoveryStatus::Ok,
                error: None,
                tools,
            });
        }
    }

    let unresolved_servers: Vec<_> = config
        .mcp_servers
        .iter()
        .filter(|(name, _server)| !pool_server_names.contains(name.as_str()))
        .map(|(name, server)| (name.clone(), server.clone()))
        .collect();
    if !unresolved_servers.is_empty() {
        results.extend(
            mcp_discovery::discover_all_mcp_tools(
                &unresolved_servers,
                mcp_discovery::DEFAULT_DISCOVERY_TIMEOUT,
            )
            .await,
        );
    }

    results
}

fn native_maenifold_discovery_result(
    gateway: &GatewayState,
) -> mcp_discovery::McpServerDiscoveryResult {
    let tools = gateway
        .list_tools()
        .into_iter()
        .map(|tool| mcp_discovery::McpDiscoveredTool {
            name: format!("maenifold.{}", tool.name),
            description: tool.description.as_ref().map(ToString::to_string),
            input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
        })
        .collect();
    mcp_discovery::McpServerDiscoveryResult {
        server: "maenifold".to_string(),
        server_type: "native".to_string(),
        status: mcp_discovery::McpDiscoveryStatus::Ok,
        error: None,
        tools,
    }
}

async fn discover_mcp_prompts(gateway: Option<&GatewayState>) -> Vec<McpPromptDiscoveryResult> {
    gateway
        .map(|gateway| {
            gateway
                .pool()
                .all_gateway_prompts()
                .into_iter()
                .map(|(server, prompts)| McpPromptDiscoveryResult { server, prompts })
                .collect()
        })
        .unwrap_or_default()
}

async fn get_mcp_prompt(
    gateway: Option<&GatewayState>,
    params: GetMcpPromptParams,
) -> std::result::Result<rmcp::model::GetPromptResult, rmcp::ErrorData> {
    let gateway = gateway.ok_or_else(|| {
        rmcp::ErrorData::invalid_params(
            format!("Gateway: unknown server '{}'", params.server),
            Some(json!(null)),
        )
    })?;
    gateway
        .pool()
        .get_prompt(&params.server, &params.name, params.arguments)
        .await
        .map_err(maenifold_error)
}

fn invalid_params(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::invalid_params(error.to_string(), Some(json!(null)))
}

fn internal_error(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(error.to_string(), Some(json!(null)))
}

fn maenifold_error(error: MaenifoldError) -> rmcp::ErrorData {
    match error {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } if code == crate::error::JSONRPC_INVALID_PARAMS => {
            rmcp::ErrorData::invalid_params(message, Some(data))
        }
        other => internal_error(other),
    }
}

#[cfg(test)]
#[path = "discovery_tools_tests.rs"]
mod tests;
