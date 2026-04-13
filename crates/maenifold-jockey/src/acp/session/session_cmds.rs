use tokio::sync::oneshot;

use super::super::worker::{worker_tx, WorkerMsg};

fn normalize_runtime_key(runtime_kind: &str) -> String {
    runtime_kind.trim().to_ascii_lowercase()
}

fn resolve_session_id(app_session_id: Option<&str>) -> Option<String> {
    app_session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| id.to_string())
}

pub async fn cancel_session(runtime_kind: &str, role_name: &str, app_session_id: Option<&str>) {
    let runtime_key = normalize_runtime_key(runtime_kind);
    if runtime_key.is_empty() {
        return;
    };
    let Some(resolved_session_id) = resolve_session_id(app_session_id) else {
        return;
    };
    let _ = worker_tx().send(WorkerMsg::Cancel {
        runtime_key,
        role_name: role_name.to_string(),
        app_session_id: resolved_session_id,
    });
}

pub async fn reset_session(
    runtime_kind: &str,
    role_name: &str,
    app_session_id: Option<&str>,
) -> Result<(), String> {
    let runtime_key =
        { let k = normalize_runtime_key(runtime_kind); if k.is_empty() { return Err("empty runtime".to_string()); }; k };
    let resolved_session_id =
        resolve_session_id(app_session_id).ok_or_else(|| "app session id required".to_string())?;
    let (tx, rx) = oneshot::channel();
    let _ = worker_tx().send(WorkerMsg::Reset {
        runtime_key,
        role_name: role_name.to_string(),
        app_session_id: resolved_session_id,
        result_tx: tx,
    });
    rx.await.map_err(|_| "worker disconnected".to_string())?
}

pub async fn reconnect_session(
    runtime_kind: &str,
    role_name: &str,
    app_session_id: Option<&str>,
) -> Result<(), String> {
    let runtime_key =
        { let k = normalize_runtime_key(runtime_kind); if k.is_empty() { return Err("empty runtime".to_string()); }; k };
    let resolved_session_id =
        resolve_session_id(app_session_id).ok_or_else(|| "app session id required".to_string())?;
    let (tx, rx) = oneshot::channel();
    let _ = worker_tx().send(WorkerMsg::Reconnect {
        runtime_key,
        role_name: role_name.to_string(),
        app_session_id: resolved_session_id,
        result_tx: tx,
    });
    rx.await.map_err(|_| "worker disconnected".to_string())?
}

pub async fn set_mode(
    runtime_kind: &str,
    role_name: &str,
    mode_id: &str,
    app_session_id: Option<&str>,
) -> Result<(), String> {
    let runtime_key =
        { let k = normalize_runtime_key(runtime_kind); if k.is_empty() { return Err("empty runtime".to_string()); }; k };
    let resolved_session_id =
        resolve_session_id(app_session_id).ok_or_else(|| "app session id required".to_string())?;
    let (tx, rx) = oneshot::channel();
    let _ = worker_tx().send(WorkerMsg::SetMode {
        runtime_key,
        role_name: role_name.to_string(),
        app_session_id: resolved_session_id,
        mode_id: mode_id.to_string(),
        result_tx: tx,
    });
    rx.await.map_err(|_| "worker disconnected".to_string())?
}

pub async fn set_config_option(
    runtime_kind: &str,
    role_name: &str,
    key: &str,
    value: &str,
    app_session_id: Option<&str>,
) -> Result<(), String> {
    let runtime_key =
        { let k = normalize_runtime_key(runtime_kind); if k.is_empty() { return Err("empty runtime".to_string()); }; k };
    let resolved_session_id =
        resolve_session_id(app_session_id).ok_or_else(|| "app session id required".to_string())?;
    let (tx, rx) = oneshot::channel();
    let _ = worker_tx().send(WorkerMsg::SetConfigOption {
        runtime_key,
        role_name: role_name.to_string(),
        app_session_id: resolved_session_id,
        config_id: key.to_string(),
        value: value.to_string(),
        result_tx: tx,
    });
    rx.await.map_err(|_| "worker disconnected".to_string())?
}
