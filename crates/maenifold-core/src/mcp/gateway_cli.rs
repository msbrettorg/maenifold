use super::{McpToolDefinition, discovery_tools, gateway_gate, gateway_startup, schema};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use rmcp::model::{CallToolResult, Content};
use serde_json::Value;

pub(crate) async fn invoke_gateway_cli_tool(
    config: &Config,
    tool_name: &str,
    arguments: Value,
) -> Result<String> {
    let gateway = match gateway_startup::build_gateway(config).await? {
        gateway_startup::GatewayStartup::Ready(gateway) => gateway,
        gateway_startup::GatewayStartup::Disabled => {
            return Err(MaenifoldError::Message(
                "Gateway is disabled; no gateway state is available".to_string(),
            ));
        }
        gateway_startup::GatewayStartup::Failed { error, .. } => {
            return Err(MaenifoldError::Message(format!(
                "Gateway startup failed: {error}"
            )));
        }
    };

    let result = if discovery_tools::is_discovery_tool(tool_name) {
        discovery_tools::handle_tool_call(Some(gateway), tool_name, arguments)
            .await
            .map_err(map_cli_gateway_error)?
    } else {
        gateway
            .handle_tool_call(tool_name, arguments)
            .await
            .map_err(map_cli_gateway_error)?
            .ok_or_else(|| MaenifoldError::UnknownTool(tool_name.to_string()))?
    };
    render_call_tool_result(result)
}

pub(crate) fn gateway_cli_help(tool_name: &str) -> String {
    match gateway_tool_definition(tool_name) {
        Some(definition) => {
            let schema = serde_json::to_string_pretty(&definition.input_schema)
                .unwrap_or_else(|_| definition.input_schema.to_string());
            format!(
                "{}\n\n{}\n\nInput schema:\n{}",
                definition.name, definition.description, schema
            )
        }
        None => {
            gate_cli_help(tool_name).unwrap_or_else(|| format!("{tool_name}\n\nNo help available."))
        }
    }
}

fn gateway_tool_definition(tool_name: &str) -> Option<McpToolDefinition> {
    match tool_name {
        "GatewayIndex" => Some(schema::gateway_tools::gateway_index_tool()),
        "GatewayExecute" => Some(schema::gateway_tools::gateway_execute_tool()),
        "GatewayBatch" => Some(schema::gateway_tools::gateway_batch_tool()),
        "GatewayIntent" => Some(schema::gateway_tools::gateway_intent_tool()),
        "DiscoverMcpTools" => Some(schema::discovery_tools::discover_mcp_tools_tool()),
        _ => None,
    }
}

fn gate_cli_help(tool_name: &str) -> Option<String> {
    gateway_gate::commands()
        .into_iter()
        .find(|command| command.name == tool_name)
        .map(|command| {
            let schema = serde_json::to_string_pretty(&command.schema)
                .unwrap_or_else(|_| command.schema.to_string());
            format!(
                "{}\n\n{}\n\nSafety class: {}\n\nInput schema:\n{}",
                command.name, command.description, command.safety_class, schema
            )
        })
}

fn render_call_tool_result(result: CallToolResult) -> Result<String> {
    let CallToolResult {
        content,
        structured_content,
        is_error,
        ..
    } = result;
    let rendered = if let Some(value) = structured_content {
        serde_json::to_string(&value)?
    } else if content.len() == 1 {
        content
            .first()
            .and_then(extract_text_content)
            .unwrap_or_else(|| serde_json::to_string(&content).unwrap_or_else(|_| "[]".to_string()))
    } else {
        serde_json::to_string(&content)?
    };

    if is_error.unwrap_or(false) {
        Err(MaenifoldError::Message(rendered))
    } else {
        Ok(rendered)
    }
}

fn extract_text_content(content: &Content) -> Option<String> {
    content.raw.as_text().map(|text| text.text.clone())
}

fn map_cli_gateway_error(error: rmcp::ErrorData) -> MaenifoldError {
    MaenifoldError::Message(error.to_string())
}
