use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, CallToolResult, Content},
    transport::TokioChildProcess,
};
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
    acp_config::{AgentsConfig, McpServerEntry},
    error::{MaenifoldError, Result},
    tools::ToolContext,
};

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

async fn invoke_via_stdio(
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

async fn invoke_via_http(
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

fn require_tool_allowed(tool_name: &str, allowed_tools: Option<&[String]>) -> Result<()> {
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

fn resolve_server_entry(
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

fn finalize_external_tool_result(
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

fn render_tool_result(result: CallToolResult) -> Result<String> {
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

fn build_child_path_env(
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, ffi::OsStr};

    use super::*;

    fn config_with_global_and_agents(
        global_servers: Vec<McpServerEntry>,
        agents: impl IntoIterator<Item = (&'static str, Vec<McpServerEntry>)>,
    ) -> AgentsConfig {
        AgentsConfig {
            mcp_servers: global_servers,
            agents: agents
                .into_iter()
                .map(|(agent_name, mcp_servers)| {
                    (
                        agent_name.to_string(),
                        crate::acp_config::AgentEntry {
                            command: "/usr/bin/env true".to_string(),
                            env: HashMap::new(),
                            mcp_servers,
                        },
                    )
                })
                .collect(),
            settings: None,
        }
    }

    #[test]
    fn split_namespaced_tool_name_requires_non_empty_segments() {
        assert_eq!(
            split_namespaced_tool_name("weather.forecast"),
            Some(("weather", "forecast"))
        );
        assert_eq!(split_namespaced_tool_name("weather."), None);
        assert_eq!(split_namespaced_tool_name(".forecast"), None);
        assert_eq!(split_namespaced_tool_name("forecast"), None);
    }

    #[test]
    fn resolve_server_entry_finds_global_server_without_agent() {
        let server = McpServerEntry::Http {
            name: "gate".to_string(),
            url: "http://localhost:8081".to_string(),
        };
        let config = config_with_global_and_agents(vec![server.clone()], []);

        let resolved = resolve_server_entry(&config, "gate", None).expect("global server resolves");

        assert_eq!(resolved, server);
    }

    #[test]
    fn resolve_server_entry_prefers_global_over_agent() {
        let global = McpServerEntry::Http {
            name: "gate".to_string(),
            url: "http://localhost:8081".to_string(),
        };
        let agent_local = McpServerEntry::Stdio {
            name: "gate".to_string(),
            command: "/usr/bin/gate".to_string(),
            args: vec![],
            timeout: None,
            env: HashMap::new(),
        };
        let config =
            config_with_global_and_agents(vec![global.clone()], [("agent-a", vec![agent_local])]);

        let resolved = resolve_server_entry(&config, "gate", Some("agent-a"))
            .expect("global server takes precedence");

        assert_eq!(resolved, global);
    }

    #[test]
    fn resolve_server_entry_falls_back_to_agent_server() {
        let agent_server = McpServerEntry::Stdio {
            name: "weather".to_string(),
            command: "/usr/bin/weather".to_string(),
            args: vec![],
            timeout: None,
            env: HashMap::new(),
        };
        let config =
            config_with_global_and_agents(vec![], [("agent-a", vec![agent_server.clone()])]);

        let resolved = resolve_server_entry(&config, "weather", Some("agent-a"))
            .expect("agent server resolves as fallback");

        assert_eq!(resolved, agent_server);
    }

    #[test]
    fn resolve_server_entry_errors_when_not_found() {
        let config = config_with_global_and_agents(vec![], []);
        let error =
            resolve_server_entry(&config, "missing", None).expect_err("unknown server should fail");
        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn require_tool_allowed_defaults_to_all_when_none() {
        require_tool_allowed("gate.status", None).expect("None means all tools allowed");
    }

    #[test]
    fn require_tool_allowed_permits_listed_tool() {
        let allowed = vec!["gate.status".to_string()];
        require_tool_allowed("gate.status", Some(&allowed))
            .expect("explicitly listed tool is allowed");
    }

    #[test]
    fn require_tool_allowed_rejects_unlisted_tool() {
        let allowed = vec!["gate.sync".to_string()];
        let error = require_tool_allowed("gate.status", Some(&allowed))
            .expect_err("unlisted tool should be rejected");
        assert!(
            error
                .to_string()
                .contains("not in the explicit allowed_tools")
        );
    }

    #[test]
    fn render_tool_result_prefers_structured_content() {
        let rendered = render_tool_result(CallToolResult::structured(serde_json::json!({
            "ok": true
        })))
        .expect("structured result renders");

        assert_eq!(rendered, r#"{"ok":true}"#);
    }

    #[test]
    fn render_tool_result_uses_text_content_when_present() {
        let rendered = render_tool_result(CallToolResult::success(vec![Content::text("done")]))
            .expect("text result renders");

        assert_eq!(rendered, "done");
    }

    #[test]
    fn render_tool_result_surfaces_external_errors() {
        let error = render_tool_result(CallToolResult::error(vec![Content::text("boom")]))
            .expect_err("error result should propagate");

        assert_eq!(error.to_string(), "boom");
    }

    #[test]
    fn finalize_external_tool_result_preserves_success_when_close_fails() {
        let result = finalize_external_tool_result(
            "weather",
            Ok(CallToolResult::success(vec![Content::text("done")])),
            Err(MaenifoldError::Message("close failed".to_string())),
        )
        .expect("successful tool output should survive close failure");

        assert_eq!(result, "done");
    }

    #[test]
    fn build_child_path_env_adds_common_user_tool_dirs_and_command_parent() {
        let path = build_child_path_env(
            Some(OsString::from("/usr/bin:/bin")),
            Some(OsString::from("/Users/tester")),
            "/Users/tester/.npm-global/bin/workiq",
        )
        .expect("path should be constructed");
        let entries: Vec<_> = std::env::split_paths(OsStr::new(&path))
            .map(|entry| entry.to_string_lossy().to_string())
            .collect();

        assert!(entries.iter().any(|entry| entry == "/usr/bin"));
        assert!(entries.iter().any(|entry| entry == "/bin"));
        assert!(
            entries
                .iter()
                .any(|entry| entry == "/Users/tester/.npm-global/bin")
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry == "/Users/tester/.dotnet/tools")
        );
        assert!(entries.iter().any(|entry| entry == "/opt/homebrew/bin"));
    }

    #[test]
    fn build_child_path_env_deduplicates_existing_entries() {
        let path = build_child_path_env(
            Some(OsString::from("/usr/bin:/bin:/usr/bin")),
            Some(OsString::from("/Users/tester")),
            "/usr/bin/tool",
        )
        .expect("path should be constructed");
        let entries: Vec<_> = std::env::split_paths(OsStr::new(&path))
            .map(|entry| entry.to_string_lossy().to_string())
            .collect();

        assert_eq!(
            entries.iter().filter(|entry| *entry == "/usr/bin").count(),
            1
        );
    }
}
