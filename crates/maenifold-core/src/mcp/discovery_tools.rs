use crate::api::mcp_discovery;
use crate::mcp::{config_tools::ConfigMutationState, gateway_handler::GatewayState};
use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

const DISCOVERY_TOOLS: &[&str] = &["DiscoverMcpTools"];

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct DiscoverMcpToolsParams {}

pub(crate) fn is_discovery_tool(tool_name: &str) -> bool {
    DISCOVERY_TOOLS.contains(&tool_name)
}

pub(crate) async fn handle_tool_call(
    state: ConfigMutationState,
    gateway: Option<Arc<GatewayState>>,
    tool_name: &str,
    arguments: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    handle_gateway_tool_call(state, gateway.as_deref(), tool_name, arguments).await
}

pub(crate) async fn handle_gateway_tool_call(
    state: ConfigMutationState,
    gateway: Option<&GatewayState>,
    tool_name: &str,
    arguments: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    match tool_name {
        "DiscoverMcpTools" => {
            let _params: DiscoverMcpToolsParams =
                serde_json::from_value(arguments).map_err(invalid_params)?;
            let results = discover_mcp_tools(state, gateway).await;
            let output = serde_json::to_string_pretty(&results).map_err(internal_error)?;
            Ok(CallToolResult::success(vec![Content::text(output)]))
        }
        _ => Err(rmcp::ErrorData::invalid_params(
            format!("Unknown discovery tool: {tool_name}"),
            Some(json!(null)),
        )),
    }
}

async fn discover_mcp_tools(
    state: ConfigMutationState,
    gateway: Option<&GatewayState>,
) -> Vec<mcp_discovery::McpServerDiscoveryResult> {
    let config = state.config_manager().get_config();
    let mut results = Vec::new();

    if let Some(gateway) = gateway {
        for (server_name, tools) in gateway.pool().all_gateway_tools() {
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

    let non_gateway: Vec<_> = config
        .mcp_servers
        .iter()
        .filter(|server| !server.is_gateway())
        .cloned()
        .collect();
    if !non_gateway.is_empty() {
        results.extend(
            mcp_discovery::discover_all_mcp_tools(
                &non_gateway,
                mcp_discovery::DEFAULT_DISCOVERY_TIMEOUT,
            )
            .await,
        );
    }

    results
}

fn invalid_params(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::invalid_params(error.to_string(), Some(json!(null)))
}

fn internal_error(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(error.to_string(), Some(json!(null)))
}
