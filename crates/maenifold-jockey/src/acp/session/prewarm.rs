use serde_json::Value;
use tokio::sync::oneshot;

use super::super::adapter::build_stdio_adapter;
use super::super::worker::RuntimeKind;
use super::super::worker::{worker_tx, WorkerMsg};
use super::mcp::load_role_mcp_servers;
use crate::db::app_session_role::{load_app_session_role_cli_id, save_app_session_role_cli_id};
use crate::db::role::update_role_config_option_defs_if_changed;
use crate::types::AppState;

fn normalize_runtime_key(runtime_kind: &str) -> Option<&'static str> {
    RuntimeKind::from_str(runtime_kind).map(|k| k.runtime_key())
}

// ── Internal shared implementation ───────────────────────────────────────────

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
        Ok(Some(a)) => a,
        _ => return None,
    };
    let resolved_session_id = opts
        .app_session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| id.to_string())
        .unwrap_or_else(|| format!("role-refresh:{}:{}", adapter.runtime_key, opts.role_name));
    let (tx, rx) = oneshot::channel();
    let _ = worker_tx().send(WorkerMsg::Prewarm {
        runtime_key: adapter.runtime_key,
        binary: adapter.binary,
        args: adapter.args,
        env: adapter.env,
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

fn persist_prewarm_result(
    state: &AppState,
    app_sid: &str,
    runtime_key: &str,
    role_name: &str,
    opts: &[Value],
    sid: &str,
) {
    if !opts.is_empty() {
        match serde_json::to_string(opts) {
            Ok(serialized) => {
                if let Err(e) =
                    update_role_config_option_defs_if_changed(state, role_name, &serialized)
                {
                    eprintln!(
                        "[prewarm] failed to persist config option defs for {role_name}: {e}"
                    );
                }
            }
            Err(e) => {
                eprintln!("[prewarm] failed to serialize config option defs for {role_name}: {e}");
            }
        }
    }
    if !sid.is_empty() {
        let _ = save_app_session_role_cli_id(state, app_sid, runtime_key, role_name, sid);
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Fire-and-forget prewarm; persists session ID and config option defs.
pub async fn prewarm_role(
    runtime_kind: &str,
    role_name: &str,
    cwd: &str,
    state: Option<(&AppState, &str)>,
) {
    let runtime_key = normalize_runtime_key(runtime_kind).unwrap_or(runtime_kind);
    let resume_session_id = state
        .as_ref()
        .and_then(|(s, sid)| load_app_session_role_cli_id(s, sid, runtime_key, role_name));
    let app_session_id = state.as_ref().map(|(_, sid)| *sid);
    let mcp_servers = state
        .as_ref()
        .map(|(s, _)| load_role_mcp_servers(s, role_name))
        .unwrap_or_default();

    let Some(rx) = send_prewarm(PrewarmOpts {
        runtime_kind,
        role_name,
        cwd,
        resume_session_id,
        app_session_id,
        mcp_servers,
    })
    .await
    else {
        return;
    };

    if let (Some((s, app_sid)), Ok((opts, _modes, sid))) = (state, rx.await) {
        persist_prewarm_result(s, app_sid, runtime_key, role_name, &opts, &sid);
    }
}

/// Prewarm and return discovered config options + modes (used by role config UI).
pub async fn prewarm_role_for_config(
    runtime_kind: &str,
    role_name: &str,
    cwd: &str,
    state: Option<(&AppState, &str)>,
) -> (Vec<Value>, Vec<String>) {
    let runtime_key = normalize_runtime_key(runtime_kind).unwrap_or(runtime_kind);
    let resume_session_id = state
        .as_ref()
        .and_then(|(s, sid)| load_app_session_role_cli_id(s, sid, runtime_key, role_name));
    let app_session_id = state.as_ref().map(|(_, sid)| *sid);
    let mcp_servers = state
        .as_ref()
        .map(|(s, _)| load_role_mcp_servers(s, role_name))
        .unwrap_or_default();

    let Some(rx) = send_prewarm(PrewarmOpts {
        runtime_kind,
        role_name,
        cwd,
        resume_session_id,
        app_session_id,
        mcp_servers,
    })
    .await
    else {
        return (vec![], vec![]);
    };

    let (opts, modes, sid) = rx.await.unwrap_or_default();
    if let Some((s, app_sid)) = state {
        persist_prewarm_result(s, app_sid, runtime_key, role_name, &opts, &sid);
    }
    (opts, modes)
}

/// Prewarm to refresh config option definitions only (no session ID involved).
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
    if !opts.is_empty() {
        if let Ok(serialized) = serde_json::to_string(&opts) {
            let _ = update_role_config_option_defs_if_changed(state, role_name, &serialized);
        }
    }
    (opts, modes)
}

/// Prewarm with an explicit session ID (used when resuming a known session).
pub async fn prewarm_role_with_session_id(
    runtime_kind: &str,
    role_name: &str,
    cwd: &str,
    resume_session_id: Option<String>,
    state: &AppState,
    app_session_id: &str,
) {
    let runtime_key = normalize_runtime_key(runtime_kind).unwrap_or(runtime_kind);
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
    if let Ok((opts, _modes, sid)) = rx.await {
        persist_prewarm_result(state, app_session_id, runtime_key, role_name, &opts, &sid);
    }
}
