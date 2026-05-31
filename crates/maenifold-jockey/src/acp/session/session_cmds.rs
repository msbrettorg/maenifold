use tokio::sync::oneshot;

use super::super::worker::{TrackedSessionReap, WorkerMsg, kill_pool_child_direct, worker_tx};

fn recv_blocking<T: Send + 'static>(rx: oneshot::Receiver<T>) -> Result<T, String> {
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(move || rx.blocking_recv())
            .join()
            .map_err(|_| "worker wait thread panicked".to_string())?
            .map_err(|_| "worker disconnected".to_string())
    } else {
        rx.blocking_recv()
            .map_err(|_| "worker disconnected".to_string())
    }
}

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

/// T-328.16 — Fire-and-forget request to SIGKILL the child ACP process for
/// the given pool slot. Used as the wall-clock-budget kill switch when a
/// hung `session.prompt()` future doesn't honour `tokio::time::timeout`'s
/// drop-cancel. The T-328.14 wait watcher propagates abandonment after reap.
pub async fn force_kill_session(runtime_kind: &str, role_name: &str, app_session_id: Option<&str>) {
    let runtime_key = normalize_runtime_key(runtime_kind);
    if runtime_key.is_empty() {
        return;
    };
    let Some(resolved_session_id) = resolve_session_id(app_session_id) else {
        return;
    };
    let _ = worker_tx().send(WorkerMsg::ForceKill {
        runtime_key,
        role_name: role_name.to_string(),
        app_session_id: resolved_session_id,
    });
}

/// T-328.16 v2 — Synchronous kill switch that bypasses the worker actor
/// mpsc. The async variant above queues a message that the worker actor
/// only processes between `rx.recv().await` polls — but with a hung
/// `session.prompt()` blocking the LocalSet's executor, the queued kill can
/// sit unread indefinitely. This variant looks up the child pid in the
/// cross-thread POOL_PIDS index and invokes `libc::kill` inline. Returns
/// true if a kill was attempted.
pub fn force_kill_session_sync(
    runtime_kind: &str,
    role_name: &str,
    app_session_id: Option<&str>,
) -> bool {
    let runtime_key = normalize_runtime_key(runtime_kind);
    if runtime_key.is_empty() {
        return false;
    }
    let Some(resolved_session_id) = resolve_session_id(app_session_id) else {
        return false;
    };
    let key = format!("{resolved_session_id}:{runtime_key}:{role_name}");
    kill_pool_child_direct(&key)
}

pub fn cancel_tracked_session_blocking(session_id: &str) -> Result<TrackedSessionReap, String> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Err("session id required".to_string());
    }
    let (tx, rx) = oneshot::channel();
    let _ = worker_tx().send(WorkerMsg::CancelTrackedSession {
        session_id: session_id.to_string(),
        result_tx: tx,
    });
    recv_blocking(rx)?
}

pub fn list_live_sessions_blocking() -> Vec<String> {
    let (tx, rx) = oneshot::channel();
    if worker_tx()
        .send(WorkerMsg::ListLiveSessions { result_tx: tx })
        .is_err()
    {
        return Vec::new();
    }
    recv_blocking(rx).unwrap_or_default()
}

pub async fn reset_session(
    runtime_kind: &str,
    role_name: &str,
    app_session_id: Option<&str>,
) -> Result<(), String> {
    let runtime_key = {
        let k = normalize_runtime_key(runtime_kind);
        if k.is_empty() {
            return Err("empty runtime".to_string());
        };
        k
    };
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
    let runtime_key = {
        let k = normalize_runtime_key(runtime_kind);
        if k.is_empty() {
            return Err("empty runtime".to_string());
        };
        k
    };
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

pub async fn set_config_option(
    runtime_kind: &str,
    role_name: &str,
    key: &str,
    value: &str,
    app_session_id: Option<&str>,
) -> Result<(), String> {
    let runtime_key = {
        let k = normalize_runtime_key(runtime_kind);
        if k.is_empty() {
            return Err("empty runtime".to_string());
        };
        k
    };
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
