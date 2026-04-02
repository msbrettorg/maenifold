use agent_client_protocol::{self as acp, Agent as _};
use tokio::time::Duration;

use super::{
    apply_session_config,
    types::{AgentError, Result, SessionInfo, SessionOpts},
};

pub(super) async fn session_new(
    conn: &acp::ClientSideConnection,
    opts: SessionOpts,
) -> Result<SessionInfo> {
    let timeout_seconds = opts.resolved_timeout_seconds();
    let cwd = opts.cwd.clone();
    let mcp_servers = opts.mcp_servers.clone();
    let response = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        conn.new_session(acp::NewSessionRequest::new(cwd).mcp_servers(mcp_servers)),
    )
    .await
    .map_err(|_| AgentError::Timeout {
        message: format!("new_session timed out after {timeout_seconds}s"),
    })?
    .map_err(AgentError::from)?;

    let session_id = response.session_id.clone();
    let has_config_options = response.config_options.is_some();
    let config_options = response.config_options.clone().unwrap_or_default();
    let session_info = SessionInfo {
        session_id: session_id.to_string(),
        modes: response
            .modes
            .as_ref()
            .map(|state| state.available_modes.clone())
            .unwrap_or_default(),
        models: response
            .models
            .as_ref()
            .map(|state| state.available_models.clone())
            .unwrap_or_default(),
        config_options: config_options.clone(),
    };

    apply_session_config(conn, session_id, &opts, has_config_options, &config_options).await;
    Ok(session_info)
}
