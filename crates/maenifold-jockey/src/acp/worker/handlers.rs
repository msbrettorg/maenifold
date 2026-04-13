use agent_client_protocol::{self as acp, Agent as _};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, resolve_cwd};
use super::super::runtime_state::{
    list_discovered_config_options, list_discovered_modes, remember_runtime_available_commands,
    remember_runtime_modes,
};
use super::super::session::cold_start;
use super::notify::{notify_connection_death, notify_prewarm};
use super::pool::{
    child_pids, pool_key, CANCEL_HANDLES, CONN_MAP, DELTA_CHANNEL_CAPACITY, PROMPT_LOCKS,
};
use super::types::{AcpEvent, ConnectionDeathEvent, PrewarmStatus};

const PROMPT_LIVENESS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

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

    use super::super::runtime_state::clear_all as clear_runtime_state;
    clear_runtime_state();

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
    runtime_key: &'static str,
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

    CONN_MAP.with(|m| m.borrow_mut().remove(&key));
    use super::super::runtime_state::clear_session as clear_session_runtime_state;
    clear_session_runtime_state(app_session_id, runtime_key, role_name);
    Ok(())
}

pub(crate) async fn reconnect_worker_session(
    runtime_key: &'static str,
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

    CONN_MAP.with(|m| m.borrow_mut().remove(&key));
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
pub(crate) async fn ensure_connection(
    key: &str,
    runtime_key: &'static str,
    binary: &str,
    args: &[String],
    env: &[(String, String)],
    resolved: &str,
    role_name: &str,
    app_session_id: &str,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    resume_session_id: Option<String>,
) -> Result<bool, String> {
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
        resolved,
        auto_approve,
        mcp_servers,
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

pub(crate) async fn apply_cold_start_config(
    key: &str,
    session_id: &acp::SessionId,
    delta_tx: &mpsc::Sender<AcpEvent>,
    role_mode: &Option<String>,
    role_config_options: &[(String, String)],
    runtime_key: &str,
) {
    // Borrow conn fields we need
    let (available_modes, current_mode) = CONN_MAP.with(|m| {
        m.borrow()
            .get(key)
            .map(|c| (c.available_modes.clone(), c.current_mode.clone()))
            .unwrap_or_default()
    });

    if !available_modes.is_empty() {
        let _ = delta_tx.try_send(AcpEvent::AvailableModes {
            modes: available_modes,
            current: current_mode,
        });
    }

    let conn_rc = CONN_MAP.with(|m| m.borrow().get(key).map(|c| c.conn.clone()));
    let Some(conn_rc) = conn_rc else { return };

    if let Some(mode) = role_mode {
        if let Err(e) = conn_rc
            .set_session_mode(acp::SetSessionModeRequest::new(
                session_id.clone(),
                acp::SessionModeId::from(mode.clone()),
            ))
            .await
        {
            acp_log(
                "config.set_mode.error",
                json!({ "mode": mode, "error": e.to_string() }),
            );
        }
    }

    let discovered = list_discovered_config_options(runtime_key);
    let supported_keys: std::collections::HashSet<String> = discovered
        .iter()
        .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(|s| s.to_string()))
        .collect();

    for (k, value) in role_config_options {
        if !supported_keys.is_empty() && !supported_keys.contains(k.as_str()) {
            acp_log(
                "config.set_option.skipped",
                json!({ "key": k, "runtime": runtime_key, "reason": "not supported by runtime" }),
            );
            continue;
        }
        if let Err(e) = conn_rc
            .set_session_config_option(acp::SetSessionConfigOptionRequest::new(
                session_id.clone(),
                acp::SessionConfigId::from(k.clone()),
                acp::SessionConfigValueId::from(value.clone()),
            ))
            .await
        {
            acp_log(
                "config.set_option.error",
                json!({ "key": k, "error": e.to_string() }),
            );
        }
    }
}

pub(crate) fn build_prompt_blocks(
    prompt: String,
    context: &[(String, String)],
) -> Vec<acp::ContentBlock> {
    let mut blocks: Vec<acp::ContentBlock> =
        vec![acp::ContentBlock::Text(acp::TextContent::new(prompt))];
    if !context.is_empty() {
        let ctx = context
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("\n");
        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "[context]\n{ctx}"
        ))));
    }
    blocks
}

/// Spawn a task that watches the health channel and evicts the connection when
/// the agent process dies.
pub(crate) fn spawn_connection_health_watch(
    key: String,
    instance_id: u64,
    death: ConnectionDeathEvent,
    mut hrx: tokio::sync::watch::Receiver<bool>,
) {
    tokio::task::spawn_local(async move {
        loop {
            if hrx.changed().await.is_err() {
                break;
            }
            if !*hrx.borrow() {
                break;
            }
        }
        acp_log("health.process_died", json!({ "key": &key }));

        // Only evict if the connection in the map is still the same instance.
        let is_same = CONN_MAP.with(|m| {
            m.borrow()
                .get(&key)
                .map(|c| c.instance_id == instance_id)
                .unwrap_or(false)
        });

        if is_same {
            CONN_MAP.with(|m| m.borrow_mut().remove(&key));
            CANCEL_HANDLES.with(|m| m.borrow_mut().remove(&key));
            notify_connection_death(death);
        }
    });
}

// ── handle_execute ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_execute(
    key: String,
    runtime_key: &'static str,
    binary: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    role_name: String,
    app_session_id: String,
    prompt: String,
    context: Vec<(String, String)>,
    cwd: String,
    delta_tx: mpsc::Sender<AcpEvent>,
    result_tx: oneshot::Sender<Result<(String, String), String>>,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    role_mode: Option<String>,
    role_config_options: Vec<(String, String)>,
    resume_session_id: Option<String>,
) {
    let resolved = resolve_cwd(&cwd);

    evict_if_cwd_changed(&key, &resolved, runtime_key, &role_name);

    let conn_exists = CONN_MAP.with(|m| m.borrow().contains_key(&key));
    if !conn_exists {
        let _ = delta_tx.try_send(AcpEvent::StatusUpdate {
            text: "Connecting to agent runtime...".to_string(),
        });
    }

    let is_cold = match ensure_connection(
        &key,
        runtime_key,
        &binary,
        &args,
        &env,
        &resolved,
        &role_name,
        &app_session_id,
        auto_approve,
        mcp_servers,
        resume_session_id,
    )
    .await
    {
        Ok(cold) => cold,
        Err(e) => {
            let _ = result_tx.send(Err(e));
            return;
        }
    };

    let session_id = CONN_MAP.with(|m| {
        m.borrow()
            .get(&key)
            .map(|c| c.session_id.clone())
            .unwrap_or_else(|| acp::SessionId::from(String::new()))
    });
    let delta_slot = CONN_MAP.with(|m| {
        m.borrow()
            .get(&key)
            .map(|c| c.delta_slot.clone())
            .unwrap_or_else(|| Arc::new(Mutex::new(None)))
    });

    if let Ok(mut slot_guard) = delta_slot.lock() {
        *slot_guard = Some(delta_tx.clone());
    }

    let conn_rc = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.conn.clone()));
    let health_rx = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.health_rx.clone()));
    let instance_id = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.instance_id));

    let (Some(conn_rc), Some(mut health_rx), Some(instance_id)) = (conn_rc, health_rx, instance_id)
    else {
        let _ = result_tx.send(Err("connection disappeared after cold start".to_string()));
        return;
    };

    if is_cold {
        let health_rx_clone = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.health_rx.clone()));
        apply_cold_start_config(
            &key,
            &session_id,
            &delta_tx,
            &role_mode,
            &role_config_options,
            runtime_key,
        )
        .await;

        let death = ConnectionDeathEvent {
            runtime_key: runtime_key.to_string(),
            role_name: role_name.clone(),
            app_session_id: app_session_id.clone(),
        };
        if let Some(hrx) = health_rx_clone {
            spawn_connection_health_watch(key.clone(), instance_id, death, hrx);
        }
    } else {
        let (available_modes, current_mode) = CONN_MAP.with(|m| {
            m.borrow()
                .get(&key)
                .map(|c| (c.available_modes.clone(), c.current_mode.clone()))
                .unwrap_or_default()
        });
        if !available_modes.is_empty() {
            let _ = delta_tx.try_send(AcpEvent::AvailableModes {
                modes: available_modes,
                current: current_mode,
            });
        }
    }

    let blocks = build_prompt_blocks(prompt, &context);

    // Install cancel handle so the Cancel message handler can send ACP cancel
    // without touching CONN_MAP.
    CANCEL_HANDLES.with(|m| {
        m.borrow_mut().insert(
            key.clone(),
            super::pool::CancelHandle {
                conn: conn_rc.clone(),
                session_id: session_id.clone(),
            },
        );
    });

    let prompt_lock = PROMPT_LOCKS.with(|m| {
        let mut map = m.borrow_mut();
        map.entry(key.clone())
            .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    });
    let prompt_guard = prompt_lock.lock().await;

    let prompt_started = Instant::now();
    let prompt_fut = conn_rc.prompt(acp::PromptRequest::new(session_id.clone(), blocks));
    tokio::pin!(prompt_fut);

    let prompt_result: Result<Result<acp::PromptResponse, acp::Error>, &str> = loop {
        tokio::select! {
            res = &mut prompt_fut => {
                break Ok(res);
            }
            changed = health_rx.changed() => {
                if changed.is_err() || !*health_rx.borrow() {
                    break Err("agent process exited while prompt was in progress");
                }
            }
            _ = tokio::time::sleep(PROMPT_LIVENESS_INTERVAL) => {
                if !*health_rx.borrow() {
                    break Err("agent process is no longer alive");
                }
            }
        }
    };

    CANCEL_HANDLES.with(|m| {
        m.borrow_mut().remove(&key);
    });
    drop(prompt_guard);

    if let Ok(mut slot_guard) = delta_slot.lock() {
        *slot_guard = None;
    }

    match prompt_result {
        Ok(Ok(resp)) => {
            acp_log(
                "stage.ok",
                json!({
                    "runtime": runtime_key,
                    "stage": "session/prompt",
                    "latencyMs": prompt_started.elapsed().as_millis(),
                    "stopReason": format!("{:?}", resp.stop_reason)
                }),
            );
            let _ = result_tx.send(Ok((String::new(), session_id.to_string())));
        }
        Ok(Err(e)) => {
            acp_log(
                "pool.invalidate",
                json!({ "runtime": runtime_key, "error": e.to_string() }),
            );
            // Invalidate connection only if still the same instance.
            let is_same = CONN_MAP.with(|m| {
                m.borrow()
                    .get(&key)
                    .map(|c| c.instance_id == instance_id)
                    .unwrap_or(false)
            });
            if is_same {
                CONN_MAP.with(|m| m.borrow_mut().remove(&key));
            }
            let _ = result_tx.send(Err(e.to_string()));
        }
        Err(reason) => {
            acp_log(
                "pool.prompt.process_died",
                json!({
                    "runtime": runtime_key,
                    "role": role_name,
                    "elapsedSec": prompt_started.elapsed().as_secs(),
                    "reason": reason
                }),
            );
            let _ = conn_rc
                .cancel(acp::CancelNotification::new(session_id.clone()))
                .await;
            let is_same = CONN_MAP.with(|m| {
                m.borrow()
                    .get(&key)
                    .map(|c| c.instance_id == instance_id)
                    .unwrap_or(false)
            });
            if is_same {
                CONN_MAP.with(|m| m.borrow_mut().remove(&key));
            }
            let _ = result_tx.send(Err(reason.to_string()));
        }
    }
}

// ── handle_prewarm ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_prewarm(
    key: String,
    runtime_key: &'static str,
    binary: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    role_name: String,
    app_session_id: String,
    cwd: String,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    role_mode: Option<String>,
    role_config_options: Vec<(String, String)>,
    result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
    resume_session_id: Option<String>,
) {
    let already_live = CONN_MAP.with(|m| m.borrow().contains_key(&key));
    if already_live {
        if let Some(tx) = result_tx {
            let session_id = CONN_MAP.with(|m| {
                m.borrow()
                    .get(&key)
                    .map(|c| c.session_id.to_string())
                    .unwrap_or_default()
            });
            let _ = tx.send((
                list_discovered_config_options(runtime_key),
                list_discovered_modes(runtime_key),
                session_id,
            ));
        }
        return;
    }

    let resolved = resolve_cwd(&cwd);
    acp_log(
        "prewarm.start",
        json!({ "runtime": runtime_key, "role": role_name }),
    );
    notify_prewarm(
        runtime_key,
        &role_name,
        &app_session_id,
        PrewarmStatus::Started,
    );

    match cold_start(
        runtime_key,
        &role_name,
        &app_session_id,
        &binary,
        &args,
        &env,
        &resolved,
        auto_approve,
        mcp_servers,
        resume_session_id,
    )
    .await
    {
        Ok(conn) => {
            acp_log(
                "prewarm.ok",
                json!({ "runtime": runtime_key, "role": role_name, "sessionId": conn.session_id.to_string() }),
            );
            notify_prewarm(
                runtime_key,
                &role_name,
                &app_session_id,
                PrewarmStatus::Ready,
            );

            let session_id = conn.session_id.clone();
            let session_id_str = session_id.to_string();
            let instance_id = conn.instance_id;
            let health_rx_clone = conn.health_rx.clone();

            let dummy_tx = {
                let (tx, _rx) = mpsc::channel::<AcpEvent>(DELTA_CHANNEL_CAPACITY);
                tx
            };

            CONN_MAP.with(|m| m.borrow_mut().insert(key.clone(), conn));

            apply_cold_start_config(
                &key,
                &session_id,
                &dummy_tx,
                &role_mode,
                &role_config_options,
                runtime_key,
            )
            .await;

            // Install a temporary drain channel so notifications sent after session/new
            // (e.g. AvailableCommandsUpdate via setTimeout) are captured rather than dropped.
            let (drain_tx, mut drain_rx) = mpsc::channel::<AcpEvent>(DELTA_CHANNEL_CAPACITY);
            let delta_slot = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.delta_slot.clone()));
            if let Some(ds) = delta_slot {
                if let Ok(mut sg) = ds.lock() {
                    *sg = Some(drain_tx);
                }
            }

            // Drain for up to 300 ms to collect any immediate notifications.
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(300);
            loop {
                match tokio::time::timeout_at(deadline, drain_rx.recv()).await {
                    Ok(Some(AcpEvent::AvailableCommands { commands })) => {
                        acp_log(
                            "commands.discovered",
                            json!({
                                "runtime": runtime_key,
                                "role": role_name,
                                "count": commands.len(),
                                "names": commands.iter().filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())).collect::<Vec<_>>()
                            }),
                        );
                        remember_runtime_available_commands(
                            &app_session_id,
                            runtime_key,
                            &role_name,
                            commands,
                        );
                    }
                    Ok(Some(AcpEvent::AvailableModes { modes, .. })) => {
                        remember_runtime_modes(
                            runtime_key,
                            modes
                                .iter()
                                .filter_map(|m| {
                                    m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                                })
                                .collect(),
                        );
                    }
                    Ok(Some(_)) => {}
                    _ => break,
                }
            }
            drop(drain_rx);

            // Clear the drain channel from the delta_slot
            let delta_slot = CONN_MAP.with(|m| {
                m.borrow()
                    .get(&key)
                    .filter(|c| c.instance_id == instance_id)
                    .map(|c| c.delta_slot.clone())
            });
            if let Some(ds) = delta_slot {
                if let Ok(mut sg) = ds.lock() {
                    *sg = None;
                }
            }

            let death = ConnectionDeathEvent {
                runtime_key: runtime_key.to_string(),
                role_name: role_name.clone(),
                app_session_id: app_session_id.clone(),
            };
            spawn_connection_health_watch(key, instance_id, death, health_rx_clone);

            if let Some(tx) = result_tx {
                let _ = tx.send((
                    list_discovered_config_options(runtime_key),
                    list_discovered_modes(runtime_key),
                    session_id_str,
                ));
            }
        }
        Err(e) => {
            acp_log(
                "prewarm.error",
                json!({ "runtime": runtime_key, "role": role_name, "error": e }),
            );
            notify_prewarm(
                runtime_key,
                &role_name,
                &app_session_id,
                PrewarmStatus::Failed { error: e.clone() },
            );
            if let Some(tx) = result_tx {
                let _ = tx.send((vec![], vec![], String::new()));
            }
        }
    }
}
