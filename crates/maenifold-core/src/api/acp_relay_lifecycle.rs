use super::acp_relay::{PendingPrompt, RELAY_IDLE_CLEANUP_TTL, RelayConfig, RelaySession};
use super::acp_relay_dispatch::{apply_event_state, forward_event, handle_incoming};
use super::acp_relay_helpers::{cancel_permissions, rpc_error, rpc_ok, send_json};
use axum::extract::ws::WebSocket;
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

pub(super) async fn relay_socket(socket: WebSocket, relay: RelayConfig) {
    let (mut sender, mut receiver) = socket.split();
    let mut session: Option<RelaySession> = None;
    let mut pending_prompt: Option<PendingPrompt> = None;
    let mut delta_rx: Option<mpsc::Receiver<jockey::acp::AcpEvent>> = None;
    let mut result_rx: Option<oneshot::Receiver<jockey::acp::AcpExecuteResult>> = None;
    // NOTE: Permission forwarding infrastructure is dormant while auto_approve is true.
    // The code is retained for future multi-user support but is currently unreachable.
    let mut permission_ids: HashMap<u64, String> = HashMap::new();
    let mut next_permission_id = 1_u64;

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                let Some(Ok(message)) = incoming else { break; };
                if !handle_incoming(
                    message,
                    &relay,
                    &mut sender,
                    &mut session,
                    &mut pending_prompt,
                    &mut delta_rx,
                    &mut result_rx,
                    &mut permission_ids,
                    &mut next_permission_id,
                ).await {
                    break;
                }
            }
            event = async { delta_rx.as_mut().expect("delta_rx checked by select! guard").recv().await }, if delta_rx.is_some() => {
                let Some(event) = event else { delta_rx = None; continue; };
                if let Some(active) = session.as_mut() {
                    apply_event_state(active, &event);
                    if !forward_event(
                        &mut sender,
                        &active.session_id,
                        &event,
                        &mut permission_ids,
                        &mut next_permission_id,
                    ).await {
                        break;
                    }
                }
            }
            result = async { result_rx.as_mut().expect("result_rx checked by select! guard").await }, if result_rx.is_some() => {
                let result = result.unwrap_or_else(|_| Err(jockey::acp::AcpExecuteError {
                    message: "worker disconnected".to_string(),
                    requested_resume: false,
                    resumed: false,
                    resume_failure_kind: None,
                }));
                if let Some(session_id) = session.as_ref().map(|active| active.session_id.clone()) {
                    while let Some(event) = delta_rx.as_mut().and_then(|rx| rx.try_recv().ok()) {
                        if let Some(active_mut) = session.as_mut() {
                            apply_event_state(active_mut, &event);
                        }
                        if !forward_event(
                            &mut sender,
                            &session_id,
                            &event,
                            &mut permission_ids,
                            &mut next_permission_id,
                        ).await {
                            break;
                        }
                    }
                }
                let cancelled = pending_prompt.as_ref().is_some_and(|prompt| prompt.cancelled);
                let request_id = pending_prompt.take().map(|prompt| prompt.request_id);
                delta_rx = None;
                result_rx = None;
                if let Some(id) = request_id {
                    let payload = prompt_result_payload(id, result, cancelled);
                    if !send_json(&mut sender, payload).await {
                        break;
                    }
                }
            }
        }
    }

    if let Some(active) = session.as_ref()
        && relay.keep_session_alive
    {
        schedule_session_idle_cleanup(relay.clone(), active.app_session_id.clone());
    } else if let Some(active) = session.as_ref() {
        reset_session_on_socket_close(&relay, active);
    }
    cancel_permissions(permission_ids);
}

fn schedule_session_idle_cleanup(relay: RelayConfig, app_session_id: String) {
    tokio::spawn(async move {
        tokio::time::sleep(relay_idle_cleanup_ttl()).await;
        let (result_tx, _result_rx) = oneshot::channel();
        let _ =
            jockey::acp::worker_tx().send(relay_reset_message(&relay, &app_session_id, result_tx));
    });
}

pub(super) fn relay_idle_cleanup_ttl() -> Duration {
    RELAY_IDLE_CLEANUP_TTL
}

fn reset_session_on_socket_close(relay: &RelayConfig, active: &RelaySession) {
    let (result_tx, _result_rx) = oneshot::channel();
    let _ = jockey::acp::worker_tx().send(relay_reset_message(
        relay,
        &active.app_session_id,
        result_tx,
    ));
}

pub(super) fn relay_reset_message(
    relay: &RelayConfig,
    app_session_id: &str,
    result_tx: oneshot::Sender<Result<(), String>>,
) -> jockey::acp::WorkerMsg {
    jockey::acp::WorkerMsg::Reset {
        runtime_key: relay.adapter.runtime_key.clone(),
        role_name: relay.agent_name.clone(),
        app_session_id: app_session_id.to_string(),
        result_tx,
    }
}

pub(super) fn relay_reset_by_session_message(
    runtime_key: String,
    role_name: String,
    session_id: String,
    result_tx: oneshot::Sender<Result<bool, String>>,
) -> jockey::acp::WorkerMsg {
    jockey::acp::WorkerMsg::ResetBySession {
        runtime_key,
        role_name,
        session_id,
        result_tx,
    }
}

pub(super) fn prompt_result_payload(
    request_id: Value,
    result: jockey::acp::AcpExecuteResult,
    cancelled: bool,
) -> Value {
    if cancelled {
        return rpc_ok(request_id, json!({"stopReason": "cancelled"}));
    }

    match result {
        Ok(_) => rpc_ok(request_id, json!({"stopReason": "end_turn"})),
        Err(error) => rpc_error(request_id, -32000, error.message),
    }
}
