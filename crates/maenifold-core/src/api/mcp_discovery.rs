mod http;
mod stdio;

use crate::acp_config::McpServerEntry;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tokio::task::JoinSet;

pub(crate) const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
pub(crate) const DEFAULT_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpDiscoveredTool {
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) input_schema: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum McpDiscoveryStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServerDiscoveryResult {
    pub(crate) server: String,
    #[serde(rename = "type")]
    pub(crate) server_type: String,
    pub(crate) status: McpDiscoveryStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
    pub(crate) tools: Vec<McpDiscoveredTool>,
}

pub(crate) async fn discover_all_mcp_tools(
    servers: &[McpServerEntry],
    _timeout_duration: Duration,
) -> Vec<McpServerDiscoveryResult> {
    let mut tasks = JoinSet::new();
    for (index, server) in servers.iter().cloned().enumerate() {
        tasks.spawn(async move {
            let timeout_duration = server.discovery_timeout();
            (index, discover_server(server, timeout_duration).await)
        });
    }

    let mut results = vec![None; servers.len()];
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok((index, result)) => results[index] = Some(result),
            Err(error) => {
                let index = results.iter().position(Option::is_none).unwrap_or(0);
                results[index] = Some(McpServerDiscoveryResult {
                    server: format!("unknown-{index}"),
                    server_type: "unknown".to_string(),
                    status: McpDiscoveryStatus::Error,
                    error: Some(format!("discovery task failed: {error}")),
                    tools: Vec::new(),
                });
            }
        }
    }

    results.into_iter().flatten().collect()
}

pub(super) fn build_headers(session_id: Option<&str>) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/event-stream"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if let Some(session_id) = session_id {
        headers.insert(
            "Mcp-Session-Id",
            HeaderValue::from_str(session_id)
                .map_err(|error| format!("invalid session id header: {error}"))?,
        );
    } else {
        headers.insert(
            "Mcp-Protocol-Version",
            HeaderValue::from_static(MCP_PROTOCOL_VERSION),
        );
    }
    Ok(headers)
}

pub(super) fn normalize_tools(
    server_name: &str,
    tools: &[Value],
) -> Result<Vec<McpDiscoveredTool>, String> {
    tools
        .iter()
        .map(|tool| {
            let tool_name = tool
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("tool missing string name for server '{server_name}'"))?;
            let input_schema = tool.get("inputSchema").cloned().ok_or_else(|| {
                format!("tool '{tool_name}' missing inputSchema for server '{server_name}'")
            })?;
            if !input_schema.is_object() {
                return Err(format!(
                    "tool '{tool_name}' returned non-object inputSchema for server '{server_name}'"
                ));
            }

            Ok(McpDiscoveredTool {
                name: format!("{server_name}.{tool_name}"),
                description: tool
                    .get("description")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                input_schema,
            })
        })
        .collect()
}

async fn discover_server(
    server: McpServerEntry,
    timeout_duration: Duration,
) -> McpServerDiscoveryResult {
    let (name, server_type) = match &server {
        McpServerEntry::Http { name, .. } => (name.clone(), "http".to_string()),
        McpServerEntry::Stdio { name, .. } => (name.clone(), "stdio".to_string()),
    };

    if name.contains('.') {
        return McpServerDiscoveryResult {
            server: name,
            server_type,
            status: McpDiscoveryStatus::Error,
            error: Some(
                "server name must not contain dots: tool routing uses 'server.toolName' \
                 notation and dots in the server name create ambiguous namespaces"
                    .to_string(),
            ),
            tools: Vec::new(),
        };
    }

    let (name, server_type, result) = match server {
        McpServerEntry::Http { name, url } => (
            name,
            "http".to_string(),
            http::discover_http_tools(&url, timeout_duration).await,
        ),
        McpServerEntry::Stdio {
            name,
            command,
            args,
            ..
        } => (
            name,
            "stdio".to_string(),
            stdio::discover_stdio_tools(&command, &args, timeout_duration).await,
        ),
    };

    match result.and_then(|tools| normalize_tools(&name, &tools)) {
        Ok(tools) => McpServerDiscoveryResult {
            server: name,
            server_type,
            status: McpDiscoveryStatus::Ok,
            error: None,
            tools,
        },
        Err(error) => McpServerDiscoveryResult {
            server: name,
            server_type,
            status: McpDiscoveryStatus::Error,
            error: Some(error),
            tools: Vec::new(),
        },
    }
}
