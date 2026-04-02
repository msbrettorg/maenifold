use super::{
    apply_session_config,
    types::{AcpxSessionInfo, AgentError, InitInfo, Result, SessionInfo, SessionOpts},
};
use agent_client_protocol::{self as acp, Agent as _};
use std::path::PathBuf;
use std::sync::Mutex as StdMutex;
use tokio::time::Duration;

pub(super) async fn list_sessions(
    conn: &acp::ClientSideConnection,
    init_info: &StdMutex<Option<InitInfo>>,
    cwd: Option<PathBuf>,
) -> Result<Vec<AcpxSessionInfo>> {
    let has_list = init_info
        .lock()
        .ok()
        .and_then(|info| info.as_ref().map(|info| info.capabilities.clone()))
        .and_then(|caps| caps.session_capabilities.list)
        .is_some();

    if !has_list {
        return Err(AgentError::Acp(
            "agent does not support session/list".to_string(),
        ));
    }

    Ok(conn
        .list_sessions(acp::ListSessionsRequest::new().cwd(cwd))
        .await
        .map(|response| {
            response
                .sessions
                .into_iter()
                .map(|session| AcpxSessionInfo {
                    session_id: session.session_id.to_string(),
                    title: session.title,
                    updated_at: session.updated_at,
                })
                .collect()
        })
        .map_err(AgentError::from)?)
}

pub(super) async fn load_session(
    conn: &acp::ClientSideConnection,
    init_info: &StdMutex<Option<InitInfo>>,
    session_id: String,
    opts: SessionOpts,
) -> Result<SessionInfo> {
    let has_load = init_info
        .lock()
        .ok()
        .and_then(|info| info.as_ref().map(|info| info.capabilities.clone()))
        .is_some_and(|caps| caps.load_session);

    if !has_load {
        return Err(AgentError::Acp(
            "agent does not support load_session".to_string(),
        ));
    }

    let timeout_seconds = opts.timeout_seconds.unwrap_or(3600);
    let response = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        conn.load_session(
            acp::LoadSessionRequest::new(acp::SessionId::new(session_id.clone()), opts.cwd.clone())
                .mcp_servers(opts.mcp_servers.clone()),
        ),
    )
    .await
    .map_err(|_| AgentError::Timeout {
        message: format!("load_session timed out after {timeout_seconds}s"),
    })?
    .map_err(AgentError::from)?;

    let has_config_options = response.config_options.is_some();
    let config_options = response.config_options.unwrap_or_default();

    apply_session_config(
        conn,
        acp::SessionId::new(session_id.clone()),
        &opts,
        has_config_options,
        &config_options,
    )
    .await;

    Ok(SessionInfo {
        session_id,
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
        config_options,
    })
}
