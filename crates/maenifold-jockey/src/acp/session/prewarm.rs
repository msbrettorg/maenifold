use serde_json::Value;
use tokio::sync::oneshot;

use super::super::adapter::build_stdio_adapter;
use super::super::worker::{WorkerMsg, worker_tx};
use super::cold_start::default_client_capabilities;
use super::mcp::load_role_mcp_servers;
use crate::types::AppState;

struct PrewarmOpts<'a> {
    runtime_kind: &'a str,
    role_name: &'a str,
    cwd: &'a str,
    resume_session_id: Option<String>,
    app_session_id: Option<&'a str>,
    mcp_servers: Vec<agent_client_protocol::McpServer>,
}

async fn send_prewarm(
    opts: PrewarmOpts<'_>,
) -> Option<oneshot::Receiver<(Vec<Value>, Vec<String>, String)>> {
    let adapter = match build_stdio_adapter(opts.runtime_kind) {
        Ok(Some(adapter)) => adapter,
        _ => return None,
    };
    let resolved_session_id = opts
        .app_session_id
        .filter(|id| !id.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("role-refresh:{}:{}", adapter.runtime_key, opts.role_name));
    let (tx, rx) = oneshot::channel();
    let _ = worker_tx().send(WorkerMsg::Prewarm {
        runtime_key: adapter.runtime_key,
        binary: adapter.binary,
        args: adapter.args,
        env: adapter.env,
        client_capabilities: default_client_capabilities(),
        role_name: opts.role_name.to_string(),
        app_session_id: resolved_session_id,
        cwd: opts.cwd.to_string(),
        auto_approve: true,
        mcp_servers: opts.mcp_servers,
        role_mode: None,
        role_config_options: vec![],
        result_tx: Some(tx),
        resume_session_id: opts.resume_session_id,
    });
    Some(rx)
}

/// Fire-and-forget prewarm.
pub async fn prewarm_role(
    runtime_kind: &str,
    role_name: &str,
    cwd: &str,
    state: Option<(&AppState, &str)>,
) {
    let app_session_id = state.as_ref().map(|(_, sid)| *sid);
    let mcp_servers = state
        .as_ref()
        .map(|(state, _)| load_role_mcp_servers(state, role_name))
        .unwrap_or_default();

    let Some(rx) = send_prewarm(PrewarmOpts {
        runtime_kind,
        role_name,
        cwd,
        resume_session_id: None,
        app_session_id,
        mcp_servers,
    })
    .await
    else {
        return;
    };

    let _ = rx.await;
}

/// Prewarm and return discovered config options + modes.
pub async fn prewarm_role_for_config(
    runtime_kind: &str,
    role_name: &str,
    cwd: &str,
    state: Option<(&AppState, &str)>,
) -> (Vec<Value>, Vec<String>) {
    let app_session_id = state.as_ref().map(|(_, sid)| *sid);
    let mcp_servers = state
        .as_ref()
        .map(|(state, _)| load_role_mcp_servers(state, role_name))
        .unwrap_or_default();

    let Some(rx) = send_prewarm(PrewarmOpts {
        runtime_kind,
        role_name,
        cwd,
        resume_session_id: None,
        app_session_id,
        mcp_servers,
    })
    .await
    else {
        return (vec![], vec![]);
    };

    let (opts, modes, _sid) = rx.await.unwrap_or_default();
    (opts, modes)
}

/// Prewarm to refresh config option definitions only.
pub async fn refresh_role_config_defs(
    runtime_kind: &str,
    role_name: &str,
    cwd: &str,
    state: &AppState,
) -> (Vec<Value>, Vec<String>) {
    let mcp_servers = load_role_mcp_servers(state, role_name);
    let Some(rx) = send_prewarm(PrewarmOpts {
        runtime_kind,
        role_name,
        cwd,
        resume_session_id: None,
        app_session_id: None,
        mcp_servers,
    })
    .await
    else {
        return (vec![], vec![]);
    };

    let (opts, modes, _sid) = rx.await.unwrap_or_default();
    (opts, modes)
}

/// Prewarm with an explicit session ID.
pub async fn prewarm_role_with_session_id(
    runtime_kind: &str,
    role_name: &str,
    cwd: &str,
    resume_session_id: Option<String>,
    state: &AppState,
    app_session_id: &str,
) {
    let mcp_servers = load_role_mcp_servers(state, role_name);
    let Some(rx) = send_prewarm(PrewarmOpts {
        runtime_kind,
        role_name,
        cwd,
        resume_session_id,
        app_session_id: Some(app_session_id),
        mcp_servers,
    })
    .await
    else {
        return;
    };

    let _ = rx.await;
}
