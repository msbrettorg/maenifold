use agent_client_protocol::{self as acp, Agent as _};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, resolve_cwd};
use super::handler_config::{
    apply_cold_start_config, build_prompt_blocks, spawn_connection_health_watch,
};
use super::handlers::{ensure_connection, evict_if_cwd_changed};
use super::pool::{CANCEL_HANDLES, CONN_MAP, PROMPT_LOCKS, CancelHandle};
use super::types::{AcpEvent, ConnectionDeathEvent};

const PROMPT_LIVENESS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_execute(
    key: String,
    runtime_key: &str,
    binary: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    client_capabilities: acp::ClientCapabilities,
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
        client_capabilities,
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
        if let Err(e) = apply_cold_start_config(
            &key,
            &session_id,
            &delta_tx,
            &role_mode,
            &role_config_options,
            runtime_key,
        )
        .await
        {
            let _ = result_tx.send(Err(e));
            return;
        }

        let death = ConnectionDeathEvent {
            runtime_key: runtime_key.to_string(),
            role_name: role_name.clone(),
            app_session_id: app_session_id.clone(),
        };
        if let Some(hrx) = health_rx_clone {
            spawn_connection_health_watch(key.clone(), instance_id, death, hrx);
        }
    } else {
        // T-170.8: Re-apply config options on warm connections so that
        // model/thoughtLevel overrides from each pipeline node reach the agent.
        // Previously only modes were re-emitted here, silently dropping
        // per-node model and thoughtLevel overrides.
        if let Err(e) = apply_cold_start_config(
            &key,
            &session_id,
            &delta_tx,
            &role_mode,
            &role_config_options,
            runtime_key,
        )
        .await
        {
            let _ = result_tx.send(Err(e));
            return;
        }
    }

    let blocks = build_prompt_blocks(prompt, &context);

    // Install cancel handle so the Cancel message handler can send ACP cancel
    // without touching CONN_MAP.
    CANCEL_HANDLES.with(|m| {
        m.borrow_mut().insert(
            key.clone(),
            CancelHandle {
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
