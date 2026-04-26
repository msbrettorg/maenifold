use agent_client_protocol_schema::{EnvVariable, McpServerStdio};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info};

use crate::mcp::stdio_session::StdioSession;

pub(super) async fn discover_stdio_tools(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    timeout_duration: Duration,
) -> Result<Vec<Value>, String> {
    info!(
        command,
        args = ?args,
        timeout_seconds = timeout_duration.as_secs_f64(),
        "attempting MCP stdio server discovery via rmcp"
    );

    let entry = McpServerStdio::new("discovery".to_string(), command.to_string())
        .args(args.to_vec())
        .env(
            env.iter()
                .map(|(name, value)| EnvVariable::new(name.clone(), value.clone()))
                .collect(),
        );
    let session =
        StdioSession::start_with_startup_timeout(&entry, timeout_duration, timeout_duration)
            .await
            .map_err(|error| {
                let stderr = error
                    .stderr
                    .filter(|stderr| !stderr.trim().is_empty())
                    .map(|stderr| format!("\nstderr:\n{}", stderr.trim()))
                    .unwrap_or_default();
                let message = format!("{}{stderr}", error.error);
                error!(command, args = ?args, error = %message, "MCP stdio discovery failed");
                message
            })?;

    let tools = session
        .tools()
        .iter()
        .map(|tool| serde_json::to_value(tool).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>();
    session.shutdown().await;
    tools
}
