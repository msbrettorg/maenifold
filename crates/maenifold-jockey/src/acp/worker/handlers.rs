use agent_client_protocol::{self as acp, Agent as _};
use serde_json::json;
use tokio::sync::oneshot;

use super::super::adapter::acp_log;
use super::super::runtime_state::clear_all as clear_runtime_state_fn;
use super::super::session::cold_start;
use super::pool::{
    CANCEL_HANDLES, CONN_MAP, PROMPT_LOCKS, TrackedSessionReap, child_pids, pool_key,
    remove_session_index, remove_session_index_for_pool_key,
};

// Re-exports from extracted modules
#[allow(unused_imports)]
pub(crate) use super::handler_config::{
    apply_cold_start_config, build_prompt_blocks, spawn_connection_health_watch,
};
pub(crate) use super::handler_execute::handle_execute;
pub(crate) use super::handler_prewarm::handle_prewarm;

// ── Connection lifecycle ──────────────────────────────────────────────────────

pub(crate) async fn shutdown_worker_state() {
    // Cancel all in-progress prompts
    let cancel_items = CANCEL_HANDLES.with(|m| {
        let mut map = m.borrow_mut();
        let items = map
            .iter()
            .map(|(k, h)| (k.clone(), h.conn.clone(), h.session_id.clone()))
            .collect::<Vec<_>>();
        map.clear();
        items
    });
    for (_key, conn, session_id) in cancel_items {
        let _ = conn.cancel(acp::CancelNotification::new(session_id)).await;
    }

    // Drop all live connections
    CONN_MAP.with(|m| {
        m.borrow_mut().clear();
    });
    PROMPT_LOCKS.with(|m| {
        m.borrow_mut().clear();
    });
    super::pool::SESSION_INDEX.with(|m| {
        m.borrow_mut().clear();
    });

    clear_runtime_state_fn();

    use super::permission::permission_requests;
    let request_ids: Vec<String> = permission_requests()
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    for request_id in request_ids {
        if let Some((_, tx)) = permission_requests().remove(&request_id) {
            let _ = tx.send(acp::RequestPermissionOutcome::Cancelled);
        }
    }

    use super::super::client::shutdown_terminals;
    shutdown_terminals().await;

    let remaining_pids: Vec<u32> = child_pids().iter().map(|r| *r).collect();
    for pid in &remaining_pids {
        unsafe {
            libc::kill(-(*pid as i32), libc::SIGKILL);
        }
        child_pids().remove(pid);
    }

    acp_log(
        "shutdown.complete",
        json!({ "forceKilled": remaining_pids.len() }),
    );
}

pub(crate) async fn reset_worker_session(
    runtime_key: &str,
    role_name: &str,
    app_session_id: &str,
) -> Result<(), String> {
    let key = pool_key(app_session_id, runtime_key, role_name);

    let cancel = CANCEL_HANDLES.with(|m| m.borrow_mut().remove(&key));
    if let Some(h) = cancel {
        let _ = h
            .conn
            .cancel(acp::CancelNotification::new(h.session_id))
            .await;
    }

    if let Some(live) = CONN_MAP.with(|m| m.borrow_mut().remove(&key)) {
        remove_session_index(runtime_key, role_name, &live.session_id.to_string());
    }
    use super::super::runtime_state::clear_session as clear_session_runtime_state;
    clear_session_runtime_state(app_session_id, runtime_key, role_name);
    Ok(())
}

pub(crate) async fn cancel_tracked_session(session_id: &str) -> Result<TrackedSessionReap, String> {
    let target = CONN_MAP.with(|m| {
        m.borrow()
            .iter()
            .find(|(_, live)| live.session_id.to_string() == session_id)
            .map(|(key, live)| (key.clone(), live.child_pid.is_some()))
    });
    let Some((key, had_process_group)) = target else {
        return Ok(TrackedSessionReap {
            acp_session: false,
            killed_process_group: false,
        });
    };

    let cancel = CANCEL_HANDLES.with(|m| m.borrow_mut().remove(&key));
    if let Some(h) = cancel {
        let _ = h
            .conn
            .cancel(acp::CancelNotification::new(h.session_id))
            .await;
    }

    if let Some(live) = CONN_MAP.with(|m| m.borrow_mut().remove(&key)) {
        remove_session_index_for_pool_key(&key);
        drop(live);
    }
    Ok(TrackedSessionReap {
        acp_session: true,
        killed_process_group: had_process_group,
    })
}

pub(crate) fn handle_cancel_tracked_session(
    session_id: String,
    result_tx: oneshot::Sender<Result<TrackedSessionReap, String>>,
) {
    tokio::task::spawn_local(async move {
        let result = cancel_tracked_session(&session_id).await;
        let _ = result_tx.send(result);
    });
}

pub(crate) fn handle_cancel(runtime_key: String, role_name: String, app_session_id: String) {
    let key = pool_key(&app_session_id, &runtime_key, &role_name);
    let handle = CANCEL_HANDLES.with(|m| {
        m.borrow().get(&key).map(|h| {
            h.mark_cancelled();
            (h.conn.clone(), h.session_id.clone())
        })
    });
    if let Some((conn, session_id)) = handle {
        acp_log(
            "cancel.sending",
            json!({ "runtime": &runtime_key, "role": role_name }),
        );
        tokio::task::spawn_local(async move {
            let _ = conn.cancel(acp::CancelNotification::new(session_id)).await;
        });
    } else {
        acp_log(
            "cancel.no_active_session",
            json!({ "runtime": &runtime_key, "role": role_name }),
        );
    }
}

pub(crate) fn live_session_ids() -> Vec<String> {
    CONN_MAP.with(|m| {
        m.borrow()
            .values()
            .map(|live| live.session_id.to_string())
            .collect()
    })
}

pub(crate) async fn reconnect_worker_session(
    runtime_key: &str,
    role_name: &str,
    app_session_id: &str,
) -> Result<(), String> {
    let key = pool_key(app_session_id, runtime_key, role_name);

    let cancel = CANCEL_HANDLES.with(|m| m.borrow_mut().remove(&key));
    if let Some(h) = cancel {
        let _ = h
            .conn
            .cancel(acp::CancelNotification::new(h.session_id))
            .await;
    }

    if let Some(live) = CONN_MAP.with(|m| m.borrow_mut().remove(&key)) {
        remove_session_index(runtime_key, role_name, &live.session_id.to_string());
    }
    use super::super::runtime_state::clear_session as clear_session_runtime_state;
    clear_session_runtime_state(app_session_id, runtime_key, role_name);
    acp_log(
        "reconnect.ok",
        json!({ "runtime": runtime_key, "role": role_name, "appSession": app_session_id }),
    );
    Ok(())
}

// ── Connection helpers ────────────────────────────────────────────────────────

/// Evict the connection if the working directory has changed.
pub(crate) fn evict_if_cwd_changed(key: &str, resolved: &str, runtime_key: &str, role_name: &str) {
    let should_evict = CONN_MAP.with(|m| {
        m.borrow()
            .get(key)
            .map(|c| c.cwd != resolved)
            .unwrap_or(false)
    });
    if should_evict {
        acp_log(
            "pool.evict",
            json!({ "runtime": runtime_key, "role": role_name, "reason": "cwd_change" }),
        );
        CONN_MAP.with(|m| m.borrow_mut().remove(key));
    }
}

/// Ensure a live connection exists for the given key; cold-start if missing.
/// Returns `true` if a cold start was performed.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn ensure_connection(
    key: &str,
    runtime_key: &str,
    binary: &str,
    args: &[String],
    env: &[(String, String)],
    client_capabilities: acp::ClientCapabilities,
    resolved: &str,
    role_name: &str,
    app_session_id: &str,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    resume_session_id: Option<String>,
) -> Result<bool, String> {
    // When the caller explicitly requests session resume, the cached connection
    // (keyed only by pool key, not by ACP session_id) cannot be assumed to point
    // at the requested session. Evict it so cold_start can run session/load.
    if resume_session_id.is_some() {
        let evicted = CONN_MAP.with(|m| m.borrow_mut().remove(key).is_some());
        if evicted {
            acp_log(
                "pool.evict",
                json!({
                    "runtime": runtime_key,
                    "role": role_name,
                    "reason": "resume_requested",
                    "sessionId": resume_session_id.as_deref().unwrap_or(""),
                }),
            );
        }
    }
    let already_live = CONN_MAP.with(|m| m.borrow().contains_key(key));
    if already_live {
        acp_log(
            "pool.reuse",
            json!({ "runtime": runtime_key, "role": role_name }),
        );
        return Ok(false);
    }
    let conn = cold_start(
        runtime_key,
        role_name,
        app_session_id,
        binary,
        args,
        env,
        client_capabilities,
        resolved,
        auto_approve,
        mcp_servers,
        None,
        resume_session_id,
    )
    .await?;
    acp_log(
        "pool.cold_start",
        json!({ "runtime": runtime_key, "role": role_name, "sessionId": conn.session_id.to_string() }),
    );
    CONN_MAP.with(|m| m.borrow_mut().insert(key.to_string(), conn));
    Ok(true)
}
