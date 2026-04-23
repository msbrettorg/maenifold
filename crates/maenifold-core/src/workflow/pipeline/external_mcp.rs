use rmcp::model::{CallToolResult, Content};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::path::Path;

use crate::{
    acp_config::{AgentsConfig, McpServerEntry},
    error::{MaenifoldError, Result},
    tools::ToolContext,
};

use super::external_mcp_transport::{invoke_via_http, invoke_via_stdio};

pub(super) fn split_namespaced_tool_name(tool_name: &str) -> Option<(&str, &str)> {
    let (server_name, remote_tool_name) = tool_name.split_once('.')?;
    if server_name.trim().is_empty() || remote_tool_name.trim().is_empty() {
        return None;
    }
    Some((server_name, remote_tool_name))
}

pub(super) async fn invoke_external_tool(
    context: &ToolContext,
    config: &AgentsConfig,
    server_name: &str,
    remote_tool_name: &str,
    tool_args: Option<&JsonValue>,
    agent_name: Option<&str>,
    allowed_tools: Option<&[String]>,
    cwd: &Path,
) -> Result<String> {
    let namespaced_tool_name = format!("{server_name}.{remote_tool_name}");
    require_tool_allowed(&namespaced_tool_name, allowed_tools)?;
    let server = resolve_server_entry(config, server_name, agent_name)?;
    let arguments = tool_arguments(tool_args)?;

    match &server {
        McpServerEntry::Stdio { command, args, .. } => {
            invoke_via_stdio(
                context,
                server_name,
                remote_tool_name,
                command,
                args,
                arguments,
                cwd,
            )
            .await
        }
        McpServerEntry::Http { url, .. } => {
            invoke_via_http(server_name, remote_tool_name, url, arguments).await
        }
    }
}

pub(super) fn require_tool_allowed(
    tool_name: &str,
    allowed_tools: Option<&[String]>,
) -> Result<()> {
    match allowed_tools {
        None => Ok(()),
        Some(tools) if tools.iter().any(|allowed| allowed == tool_name) => Ok(()),
        Some(_) => Err(MaenifoldError::AgentSetup(format!(
            "External MCP tool '{tool_name}' is not in the explicit allowed_tools list"
        ))),
    }
}

fn tool_arguments(tool_args: Option<&JsonValue>) -> Result<Option<JsonMap<String, JsonValue>>> {
    match tool_args {
        None => Ok(None),
        Some(JsonValue::Object(map)) => Ok(Some(map.clone())),
        Some(_) => Err(MaenifoldError::InvalidPayload(
            "External MCP tool arguments must be a JSON object".to_string(),
        )),
    }
}

pub(super) fn resolve_server_entry(
    config: &AgentsConfig,
    server_name: &str,
    agent_name: Option<&str>,
) -> Result<McpServerEntry> {
    if let Some(entry) = config.mcp_servers.iter().find(|s| s.name() == server_name) {
        return Ok(entry.clone());
    }
    if let Some(agent_name) = agent_name {
        if let Some(agent) = config.agents.get(agent_name) {
            if let Some(entry) = agent.mcp_servers.iter().find(|s| s.name() == server_name) {
                return Ok(entry.clone());
            }
        }
    }
    let scope = agent_name.map_or("global config".to_string(), |a| {
        format!("agent '{a}' or global config")
    });
    Err(MaenifoldError::AgentSetup(format!(
        "MCP server '{server_name}' not found in {scope}"
    )))
}

pub(super) fn finalize_external_tool_result(
    server_name: &str,
    call_result: Result<CallToolResult>,
    close_result: Result<rmcp::service::QuitReason>,
) -> Result<String> {
    match (call_result, close_result) {
        (Ok(result), Ok(_)) => render_tool_result(result),
        (Ok(result), Err(close_error)) => {
            tracing::warn!(
                server_name,
                error = %close_error,
                "Failed to close external MCP server connection after successful tool call"
            );
            render_tool_result(result)
        }
        (Err(error), Ok(_)) => Err(error),
        (Err(error), Err(close_error)) => {
            tracing::warn!(
                server_name,
                error = %close_error,
                "Failed to close external MCP server connection after tool call failure"
            );
            Err(error)
        }
    }
}

pub(super) fn render_tool_result(result: CallToolResult) -> Result<String> {
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

#[cfg(test)]
#[path = "external_mcp_tests.rs"]
mod tests;
