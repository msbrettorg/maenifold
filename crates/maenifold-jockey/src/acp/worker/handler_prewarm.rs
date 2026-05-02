use agent_client_protocol::{self as acp};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, resolve_cwd};
use super::super::runtime_state::{
    list_discovered_config_options, list_discovered_modes, remember_runtime_available_commands,
    remember_runtime_modes,
};
use super::super::session::cold_start;
use super::handler_config::{
    apply_cold_start_config, spawn_connection_health_watch, structured_config_option_pairs,
};
use super::notify::notify_prewarm;
use super::pool::{
    CONN_MAP, DELTA_CHANNEL_CAPACITY, find_session_key, register_session_index,
    remove_session_index,
};
use super::types::{AcpEvent, ConnectionDeathEvent, PrewarmStatus};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_prewarm(
    key: String,
    runtime_key: &str,
    binary: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    client_capabilities: acp::ClientCapabilities,
    role_name: String,
    app_session_id: String,
    cwd: String,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    role_mode: Option<String>,
    role_config_options: Vec<Value>,
    delta_tx: Option<mpsc::Sender<AcpEvent>>,
    result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
    resume_session_id: Option<String>,
) {
    let mut result_tx = result_tx;
    fn remember_prewarm_event(
        runtime_key: &str,
        role_name: &str,
        app_session_id: &str,
        event: &AcpEvent,
    ) {
        match event {
            AcpEvent::AvailableCommands { commands } => {
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
                    app_session_id,
                    runtime_key,
                    role_name,
                    commands.clone(),
                );
            }
            AcpEvent::AvailableModes { modes, .. } => {
                remember_runtime_modes(
                    runtime_key,
                    modes
                        .iter()
                        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
                        .collect(),
                );
            }
            _ => {}
        }
    }

    let is_resume_replay = resume_session_id.is_some();

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

    let forwarded_delta_tx = if let Some(external_delta_tx) = delta_tx.clone() {
        let (forward_tx, mut forward_rx) = mpsc::channel::<AcpEvent>(DELTA_CHANNEL_CAPACITY);
        let runtime_key_owned = runtime_key.to_string();
        let role_name_owned = role_name.clone();
        let app_session_id_owned = app_session_id.clone();
        tokio::task::spawn_local(async move {
            while let Some(event) = forward_rx.recv().await {
                remember_prewarm_event(
                    &runtime_key_owned,
                    &role_name_owned,
                    &app_session_id_owned,
                    &event,
                );
                if external_delta_tx.send(event).await.is_err() {
                    break;
                }
            }
        });
        Some(forward_tx)
    } else {
        None
    };

    if let Some(session_id) = resume_session_id.clone()
        && find_session_key(runtime_key, &role_name, &session_id).is_some()
        && complete_live_resume_prewarm(
            &key,
            runtime_key,
            &role_name,
            &app_session_id,
            &cwd,
            mcp_servers.clone(),
            forwarded_delta_tx.clone(),
            result_tx.take(),
            &session_id,
        )
        .await
    {
        return;
    }

    let already_live = CONN_MAP.with(|m| m.borrow().contains_key(&key));
    if already_live {
        complete_already_live_prewarm(&key, runtime_key, forwarded_delta_tx, result_tx, false)
            .await;
        return;
    }

    let flat_role_config_options =
        structured_config_option_pairs(&role_config_options, runtime_key);
    match cold_start(
        runtime_key,
        &role_name,
        &app_session_id,
        &binary,
        &args,
        &env,
        client_capabilities,
        &resolved,
        auto_approve,
        mcp_servers,
        forwarded_delta_tx.clone(),
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

            CONN_MAP.with(|m| m.borrow_mut().insert(key.clone(), conn));
            register_session_index(runtime_key, &role_name, &session_id_str, &key);

            if let Some(forward_tx) = forwarded_delta_tx.as_ref() {
                if is_resume_replay {
                    let _ = forward_tx.send(AcpEvent::ReplayComplete).await;
                }
                // Prewarm path: log critical config errors but don't block.
                // The actual prompt handler will re-apply and catch failures.
                if let Err(e) = apply_cold_start_config(
                    &key,
                    &session_id,
                    forward_tx,
                    &role_mode,
                    &flat_role_config_options,
                    runtime_key,
                )
                .await
                {
                    acp_log(
                        "prewarm.config_error",
                        json!({ "runtime": runtime_key, "error": e }),
                    );
                }
            } else {
                let dummy_tx = {
                    let (tx, _rx) = mpsc::channel::<AcpEvent>(DELTA_CHANNEL_CAPACITY);
                    tx
                };

                // Prewarm path: log critical config errors but don't block.
                if let Err(e) = apply_cold_start_config(
                    &key,
                    &session_id,
                    &dummy_tx,
                    &role_mode,
                    &flat_role_config_options,
                    runtime_key,
                )
                .await
                {
                    acp_log(
                        "prewarm.config_error",
                        json!({ "runtime": runtime_key, "error": e }),
                    );
                }
            }

            // Clear any temporary delta forwarding slot once prewarm setup is complete.
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
            complete_failed_prewarm(delta_tx, result_tx, is_resume_replay).await;
        }
    }
}

async fn complete_failed_prewarm(
    delta_tx: Option<mpsc::Sender<AcpEvent>>,
    result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
    is_resume_replay: bool,
) {
    if is_resume_replay {
        if let Some(tx) = delta_tx {
            let _ = tx.send(AcpEvent::ReplayComplete).await;
        }
    }
    if let Some(tx) = result_tx {
        let _ = tx.send((vec![], vec![], String::new()));
    }
}

#[allow(clippy::too_many_arguments)]
async fn complete_live_resume_prewarm(
    key: &str,
    runtime_key: &str,
    role_name: &str,
    _app_session_id: &str,
    _cwd: &str,
    _mcp_servers: Vec<acp::McpServer>,
    delta_tx: Option<mpsc::Sender<AcpEvent>>,
    result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
    resume_session_id: &str,
) -> bool {
    let Some(existing_key) = find_session_key(runtime_key, role_name, resume_session_id) else {
        return false;
    };

    if existing_key != key {
        let moved = CONN_MAP.with(|m| {
            let mut map = m.borrow_mut();
            map.remove(&existing_key).map(|live| {
                let old_session_id = live.session_id.to_string();
                map.insert(key.to_string(), live);
                old_session_id
            })
        });
        if let Some(old_session_id) = moved {
            remove_session_index(runtime_key, role_name, &old_session_id);
            register_session_index(runtime_key, role_name, &old_session_id, key);
        }
    }

    let live = CONN_MAP.with(|m| {
        m.borrow().get(key).map(|live| {
            (
                live.delta_slot.clone(),
                live.session_id.to_string(),
            )
        })
    });
    let Some((delta_slot, live_session_id)) = live else {
        return false;
    };
    if live_session_id != resume_session_id {
        return false;
    }

    if let Ok(mut slot) = delta_slot.lock() {
        *slot = delta_tx.clone();
    }

    acp_log(
        "session.live_attach.ok",
        json!({ "runtime": runtime_key, "sessionId": resume_session_id }),
    );

    if let Some(tx) = delta_tx {
        let _ = tx.send(AcpEvent::ReplayComplete).await;
    }

    if let Ok(mut slot) = delta_slot.lock() {
        *slot = None;
    }

    if let Some(tx) = result_tx {
        let _ = tx.send((
            list_discovered_config_options(runtime_key),
            list_discovered_modes(runtime_key),
            resume_session_id.to_string(),
        ));
    }
    true
}

async fn complete_already_live_prewarm(
    key: &str,
    runtime_key: &str,
    delta_tx: Option<mpsc::Sender<AcpEvent>>,
    result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
    is_resume_replay: bool,
) {
    let session_id = CONN_MAP.with(|m| {
        m.borrow()
            .get(key)
            .map(|c| c.session_id.to_string())
            .unwrap_or_default()
    });

    if is_resume_replay {
        if let Some(tx) = delta_tx {
            let _ = tx.send(AcpEvent::ReplayComplete).await;
        }
    }

    if let Some(tx) = result_tx {
        let _ = tx.send((
            list_discovered_config_options(runtime_key),
            list_discovered_modes(runtime_key),
            session_id,
        ));
    }
}

#[cfg(test)]
#[path = "handler_prewarm_tests.rs"]
mod tests;
