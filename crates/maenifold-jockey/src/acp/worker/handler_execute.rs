use agent_client_protocol::schema::{self as acp};
use serde_json::{Map, Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use super::super::adapter::{acp_log, resolve_cwd};
use super::handler_config::{
    apply_cold_start_config, build_prompt_blocks, spawn_connection_health_watch,
};
use super::handlers::{ensure_connection, evict_if_cwd_changed};
use super::pool::{CANCEL_HANDLES, CONN_MAP, CancelHandle, PROMPT_LOCKS};
use super::resume_execute::{
    generic_error, normalize_prompt_result_after_cancel, prompt_error, resume_start_error,
};
use super::types::{AcpEvent, AcpExecuteError, AcpExecuteSuccess, ConnectionDeathEvent};

const PROMPT_LIVENESS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

fn build_prompt_request(
    session_id: &acp::SessionId,
    blocks: Vec<acp::ContentBlock>,
    allowed_tools: &Option<Vec<String>>,
    output_schema: &Option<Value>,
) -> acp::PromptRequest {
    let Some(schema) = output_schema else {
        return acp::PromptRequest::new(session_id.clone(), blocks);
    };
    let complete_node_allowed = allowed_tools
        .as_ref()
        .is_some_and(|tools| tools.iter().any(|tool| tool == "complete_node"));
    if !complete_node_allowed {
        return acp::PromptRequest::new(session_id.clone(), blocks);
    }

    let description = "Submit the final structured output for this pipeline node. The pipeline will not advance until this tool is called with valid arguments matching the declared output_schema.";
    let mut complete_node = Map::new();
    complete_node.insert("name".to_string(), json!("complete_node"));
    complete_node.insert("description".to_string(), json!(description));
    complete_node.insert("input_schema".to_string(), schema.clone());
    complete_node.insert("inputSchema".to_string(), schema.clone());

    let mut meta = Map::new();
    meta.insert(
        "tools".to_string(),
        Value::Array(vec![Value::Object(complete_node)]),
    );
    meta.insert(
        "allowedTools".to_string(),
        json!(allowed_tools.as_ref().cloned().unwrap_or_default()),
    );
    acp::PromptRequest::new(session_id.clone(), blocks).meta(meta)
}

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
    prompt_blocks: Option<Vec<acp::ContentBlock>>,
    cwd: String,
    delta_tx: mpsc::Sender<AcpEvent>,
    result_tx: oneshot::Sender<Result<AcpExecuteSuccess, AcpExecuteError>>,
    auto_approve: bool,
    mcp_servers: Vec<acp::McpServer>,
    allowed_tools: Option<Vec<String>>,
    output_schema: Option<serde_json::Value>,
    role_config_options: Vec<(String, String)>,
    resume_session_id: Option<String>,
    session_id_tx: Option<oneshot::Sender<String>>,
) {
    let prompt_lock = PROMPT_LOCKS.with(|m| {
        let mut map = m.borrow_mut();
        map.entry(key.clone())
            .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    });
    let prompt_guard = prompt_lock.lock().await;
    let requested_resume = resume_session_id.is_some();
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
            let _ = result_tx.send(Err(resume_start_error(e, requested_resume)));
            return;
        }
    };

    let (session_id, resumed) = CONN_MAP.with(|m| {
        m.borrow()
            .get(&key)
            .map(|c| (c.session_id.clone(), c.resumed))
            .unwrap_or_else(|| (acp::SessionId::from(String::new()), false))
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
        let message = "connection disappeared after cold start".to_string();
        let _ = result_tx.send(Err(prompt_error(message, requested_resume, resumed)));
        return;
    };
    let session_id_text = session_id.to_string();
    if let (false, Some(tx)) = (session_id_text.is_empty(), session_id_tx) {
        let _ = tx.send(session_id_text);
    }

    if is_cold {
        let health_rx_clone = CONN_MAP.with(|m| m.borrow().get(&key).map(|c| c.health_rx.clone()));
        if let Err(e) = apply_cold_start_config(
            &key,
            &session_id,
            &delta_tx,
            &role_config_options,
            runtime_key,
        )
        .await
        {
            let _ = result_tx.send(Err(generic_error(e, requested_resume, resumed)));
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
        // Previously config options were only applied during cold start, silently
        // dropping per-node model and thoughtLevel overrides on warm reuse.
        if let Err(e) = apply_cold_start_config(
            &key,
            &session_id,
            &delta_tx,
            &role_config_options,
            runtime_key,
        )
        .await
        {
            let _ = result_tx.send(Err(generic_error(e, requested_resume, resumed)));
            return;
        }
    }

    let blocks = prompt_blocks.unwrap_or_else(|| build_prompt_blocks(prompt, &context));
    let prompt_request = build_prompt_request(&session_id, blocks, &allowed_tools, &output_schema);

    // Install cancel handle so the Cancel message handler can send ACP cancel
    // without touching CONN_MAP.
    let cancelled = Arc::new(AtomicBool::new(false));
    CANCEL_HANDLES.with(|m| {
        m.borrow_mut().insert(
            key.clone(),
            CancelHandle {
                conn: conn_rc.clone(),
                session_id: session_id.clone(),
                cancelled: cancelled.clone(),
            },
        );
    });

    let prompt_started = Instant::now();
    let prompt_fut = conn_rc.send_request(prompt_request).block_task();
    tokio::pin!(prompt_fut);

    let prompt_result: Result<Result<acp::PromptResponse, acp::Error>, String> = loop {
        tokio::select! {
            res = &mut prompt_fut => {
                break Ok(res);
            }
            changed = health_rx.changed() => {
                if changed.is_err() || !*health_rx.borrow() {
                    break Err(format!(
                        "Agent '{runtime_key}' exited: prompt was in progress when the process stopped"
                    ));
                }
            }
            _ = tokio::time::sleep(PROMPT_LIVENESS_INTERVAL) => {
                if !*health_rx.borrow() {
                    break Err(format!("Agent '{runtime_key}' exited: process is no longer alive"));
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

    let was_cancelled = cancelled.load(Ordering::SeqCst);
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
            let _ = result_tx.send(normalize_prompt_result_after_cancel(
                Ok(AcpExecuteSuccess {
                    output: String::new(),
                    session_id: session_id.to_string(),
                    resumed,
                    stop_reason: resp.stop_reason,
                }),
                was_cancelled,
                &session_id,
                resumed,
            ));
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
            let _ = result_tx.send(normalize_prompt_result_after_cancel(
                Err(prompt_error(e.to_string(), requested_resume, resumed)),
                was_cancelled,
                &session_id,
                resumed,
            ));
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
            let _ = conn_rc.send_notification(acp::CancelNotification::new(session_id.clone()));
            let is_same = CONN_MAP.with(|m| {
                m.borrow()
                    .get(&key)
                    .map(|c| c.instance_id == instance_id)
                    .unwrap_or(false)
            });
            if is_same {
                CONN_MAP.with(|m| m.borrow_mut().remove(&key));
            }
            let _ = result_tx.send(normalize_prompt_result_after_cancel(
                Err(prompt_error(reason, requested_resume, resumed)),
                was_cancelled,
                &session_id,
                resumed,
            ));
        }
    }
}

#[cfg(test)]
#[path = "handler_execute_tests.rs"]
mod handler_execute_tests;

#[cfg(test)]
#[path = "handler_execute_consecutive_tests.rs"]
mod handler_execute_consecutive_tests;
