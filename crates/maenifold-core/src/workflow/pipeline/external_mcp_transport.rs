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

use super::external_mcp::finalize_external_tool_result;
use crate::{
    error::{MaenifoldError, Result},
    mcp::gateway_session::HttpServerConfig,
    mcp::http_oauth::prepare_http_client,
    mcp::stdio_pool::StdioPool,
};

pub(super) async fn invoke_via_stdio(
    server_name: &str,
    remote_tool_name: &str,
    arguments: Option<JsonMap<String, JsonValue>>,
    pool: &StdioPool,
) -> Result<String> {
    let arguments = arguments
        .map(JsonValue::Object)
        .unwrap_or_else(|| JsonValue::Object(JsonMap::new()));
    info!(server_name, remote_tool_name, "calling external MCP tool");
    let call_result = pool
        .call_tool(server_name, remote_tool_name, arguments)
        .await
        .map_err(|error| {
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

    finalize_external_tool_result(
        server_name,
        call_result,
        Ok(rmcp::service::QuitReason::Closed),
    )
}

pub(super) async fn invoke_via_stdio_fallback(
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

impl HttpServerConfig {
    fn auth_discovery_config(&self) -> Result<Self> {
        let mut discovery_url = reqwest::Url::parse(&self.url).map_err(|error| {
            MaenifoldError::Message(format!(
                "invalid HTTP MCP URL for gateway server '{}': {error}",
                self.name
            ))
        })?;
        discovery_url.set_path("");
        discovery_url.set_query(None);
        discovery_url.set_fragment(None);
        Ok(Self {
            url: discovery_url.to_string(),
            ..self.clone()
        })
    }
}

pub(super) async fn invoke_via_http(
    config: &HttpServerConfig,
    remote_tool_name: &str,
    arguments: Option<JsonMap<String, JsonValue>>,
) -> Result<String> {
    use rmcp::transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    };

    // Build an explicit reqwest client with a 30-second timeout.
    // rustls is activated transitively via rmcp's `reqwest` feature flag
    // (`reqwest?/rustls`), so this client is rustls-capable for HTTPS endpoints.
    let http_client = reqwest::Client::builder()
        .timeout(config.timeout)
        .build()
        .map_err(|error| {
            MaenifoldError::Message(format!(
                "Failed to build HTTP client for MCP server '{}': {error}",
                config.name
            ))
        })?;
    let auth_config = config.auth_discovery_config()?;
    let prepared_client = prepare_http_client(&auth_config, http_client).await?;
    let mut transport_config = StreamableHttpClientTransportConfig::with_uri(config.url.clone())
        .custom_headers(config.custom_headers()?);
    if let Some(auth_header) = prepared_client.auth_header {
        transport_config = transport_config.auth_header(auth_header);
    }
    let transport =
        StreamableHttpClientTransport::with_client(prepared_client.client, transport_config);

    let mut client = ().serve(transport).await.map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to connect to external MCP server '{}' at {}: {error}",
            config.name, config.url
        ))
    })?;

    let request = build_call_request(remote_tool_name, arguments);
    info!(
        server_name = config.name,
        remote_tool_name,
        url = config.url,
        "calling external MCP tool via HTTP"
    );
    let call_result = client.call_tool(request).await.map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to invoke external MCP tool '{}.{remote_tool_name}': {error}",
            config.name
        ))
    });
    if call_result.is_ok() {
        info!(
            server_name = config.name,
            remote_tool_name, "external MCP HTTP tool call succeeded"
        );
    }
    let close_result = client.close().await.map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to close external MCP server '{}': {error}",
            config.name
        ))
    });

    finalize_external_tool_result(&config.name, call_result, close_result)
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
