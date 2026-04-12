use sacp::{ConnectionTo, schema as sacps};
use tokio::time::Duration;

use super::{
    client::HeadlessClient,
    convert::bridge,
    set_config_option::apply_session_config,
    types::{AgentError, Result, SessionInfo, SessionOpts},
};

const CONNECTION_TIMEOUT_SECS: u64 = 60;

fn trace_enabled() -> bool {
    std::env::var("MAENIFOLD_TRACE")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        })
        .unwrap_or(true)
}

pub(super) async fn session_new(
    conn: &ConnectionTo<sacp::Agent>,
    client: &HeadlessClient,
    opts: SessionOpts,
) -> Result<SessionInfo> {
    let timeout_seconds = opts
        .acp_connection_timeout_seconds
        .unwrap_or(CONNECTION_TIMEOUT_SECS);
    let cwd = opts.cwd.clone();
    let mcp_servers = opts
        .mcp_servers
        .clone()
        .into_iter()
        .map(bridge)
        .collect::<Result<Vec<sacps::McpServer>>>()?;

    let response = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        conn.send_request(sacps::NewSessionRequest::new(cwd).mcp_servers(mcp_servers))
            .block_task(),
    )
    .await
    .map_err(|_| AgentError::Timeout {
        message: format!("new_session timed out after {timeout_seconds}s"),
    })?
    .map_err(|error| AgentError::Acp(error.to_string()))?;

    let session_id = response.session_id.clone();
    if trace_enabled() {
        eprintln!("[acp] new session: {}", session_id);
    }
    let has_config_options = response.config_options.is_some();
    let config_options = response.config_options.clone().unwrap_or_default();
    let session_info = SessionInfo {
        session_id: session_id.to_string(),
        modes: response
            .modes
            .as_ref()
            .map(|state| {
                state
                    .available_modes
                    .clone()
                    .into_iter()
                    .map(bridge)
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default(),
        models: response
            .models
            .as_ref()
            .map(|state| {
                state
                    .available_models
                    .clone()
                    .into_iter()
                    .map(bridge)
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default(),
        config_options: config_options
            .clone()
            .into_iter()
            .map(bridge)
            .collect::<Result<Vec<_>>>()?,
    };

    client.register_session(session_id.to_string()).await;
    apply_session_config(
        conn,
        session_id.to_string(),
        &opts,
        has_config_options,
        &session_info.config_options,
    )
    .await;
    Ok(session_info)
}
