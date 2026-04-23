use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::{
    collections::HashSet,
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};
use tokio::process::Command;
use tracing::{error, info};

use crate::{
    error::{MaenifoldError, Result},
    tools::ToolContext,
};

use super::external_mcp::finalize_external_tool_result;

pub(super) async fn invoke_via_stdio(
    _context: &ToolContext,
    server_name: &str,
    remote_tool_name: &str,
    command: &str,
    args: &[String],
    arguments: Option<JsonMap<String, JsonValue>>,
    cwd: &Path,
) -> Result<String> {
    let mut child = Command::new(command);
    child.args(args);
    child.current_dir(cwd);
    if let Some(path) = build_child_path_env(env::var_os("PATH"), env::var_os("HOME"), command) {
        child.env("PATH", path);
    }

    let transport = TokioChildProcess::new(child).map_err(|error| {
        error!(
            server_name,
            error = %error,
            "failed to spawn external MCP server"
        );
        MaenifoldError::Message(format!(
            "Failed to spawn external MCP server '{server_name}': {error}"
        ))
    })?;
    let mut client = ().serve(transport).await.map_err(|error| {
        error!(
            server_name,
            error = %error,
            "failed to connect to external MCP server"
        );
        MaenifoldError::Message(format!(
            "Failed to connect to external MCP server '{server_name}': {error}"
        ))
    })?;

    let request = build_call_request(remote_tool_name, arguments);
    info!(server_name, remote_tool_name, "calling external MCP tool");
    let call_result = client.call_tool(request).await.map_err(|error| {
        error!(
            server_name,
            remote_tool_name,
            error = %error,
            "external MCP tool call failed"
        );
        MaenifoldError::Message(format!(
            "Failed to invoke external MCP tool '{server_name}.{remote_tool_name}': {error}"
        ))
    });
    if call_result.is_ok() {
        info!(
            server_name,
            remote_tool_name, "external MCP tool call succeeded"
        );
    }
    let close_result = client.close().await.map_err(|error| {
        error!(
            server_name,
            error = %error,
            "failed to close external MCP server"
        );
        MaenifoldError::Message(format!(
            "Failed to close external MCP server '{server_name}': {error}"
        ))
    });

    finalize_external_tool_result(server_name, call_result, close_result)
}

pub(super) async fn invoke_via_http(
    server_name: &str,
    remote_tool_name: &str,
    url: &str,
    arguments: Option<JsonMap<String, JsonValue>>,
) -> Result<String> {
    use rmcp::transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    };

    // Build an explicit reqwest client with a 30-second timeout.
    // rustls is activated transitively via rmcp's `reqwest` feature flag
    // (`reqwest?/rustls`), so this client is rustls-capable for HTTPS endpoints.
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|error| {
            MaenifoldError::Message(format!(
                "Failed to build HTTP client for MCP server '{server_name}': {error}"
            ))
        })?;
    let config = StreamableHttpClientTransportConfig::with_uri(url);
    let transport = StreamableHttpClientTransport::with_client(http_client, config);

    let mut client = ().serve(transport).await.map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to connect to external MCP server '{server_name}' at {url}: {error}"
        ))
    })?;

    let request = build_call_request(remote_tool_name, arguments);
    info!(
        server_name,
        remote_tool_name, url, "calling external MCP tool via HTTP"
    );
    let call_result = client.call_tool(request).await.map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to invoke external MCP tool '{server_name}.{remote_tool_name}': {error}"
        ))
    });
    if call_result.is_ok() {
        info!(
            server_name,
            remote_tool_name, "external MCP HTTP tool call succeeded"
        );
    }
    let close_result = client.close().await.map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to close external MCP server '{server_name}': {error}"
        ))
    });

    finalize_external_tool_result(server_name, call_result, close_result)
}

fn build_call_request(
    remote_tool_name: &str,
    arguments: Option<JsonMap<String, JsonValue>>,
) -> CallToolRequestParams {
    match arguments {
        Some(arguments) => {
            CallToolRequestParams::new(remote_tool_name.to_string()).with_arguments(arguments)
        }
        None => CallToolRequestParams::new(remote_tool_name.to_string()),
    }
}

pub(super) fn build_child_path_env(
    existing_path: Option<OsString>,
    home: Option<OsString>,
    command: &str,
) -> Option<OsString> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let mut push_entry = |entry: PathBuf| {
        if !entry.as_os_str().is_empty() && seen.insert(entry.clone()) {
            entries.push(entry);
        }
    };

    if let Some(existing_path) = existing_path {
        for entry in env::split_paths(&existing_path) {
            push_entry(entry);
        }
    }

    if let Some(home) = home.map(PathBuf::from) {
        for relative in [".npm-global/bin", ".dotnet/tools", ".local/bin"] {
            push_entry(home.join(relative));
        }
    }

    for entry in [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        push_entry(PathBuf::from(entry));
    }

    if let Some(parent) = Path::new(command).parent() {
        push_entry(parent.to_path_buf());
    }

    env::join_paths(entries).ok()
}
