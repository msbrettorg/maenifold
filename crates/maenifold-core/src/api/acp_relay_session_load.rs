//! `session/load` handler and replay-completion helpers.
//!
//! Split out of `acp_relay_session.rs` to keep that file under the 300-LOC
//! budget. The replay-completion semantics (T-330.3) live here too because
//! they are only used by `handle_session_load`.

use super::acp_relay::{RelayConfig, RelaySession, SessionLoadParams};
use super::acp_relay_dispatch::forward_event;
use super::acp_relay_helpers::{
    build_modes, reject_client_mcp_servers, resolve_session_cwd, rpc_error,
};
use super::acp_relay_mcp::reset_session_connection;
use axum::extract::ws::{Message, WebSocket};
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

pub(super) async fn handle_session_load(
    relay: &RelayConfig,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    session: &mut Option<RelaySession>,
    permission_ids: &mut HashMap<u64, String>,
    next_permission_id: &mut u64,
    params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let active = session.as_ref().ok_or_else(|| {
        rpc_error(
            request_id.clone(),
            -32600,
            "initialize must be called before session/load",
        )
    })?;
    reject_client_mcp_servers(&params, request_id.clone(), "session/load")?;
    let request: SessionLoadParams = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    let _ = request.mcp_servers.len();
    let requested_session_id = request.session_id;
    let app_session_id = active.app_session_id.clone();
    let client_capabilities = active.client_capabilities.clone();
    let previous_cwd = active.cwd.clone();
    let cwd = request
        .cwd
        .map(|cwd| {
            resolve_session_cwd(Some(cwd), &relay.default_cwd)
                .map_err(|error| rpc_error(request_id.clone(), -32602, error))
        })
        .transpose()?
        .unwrap_or(previous_cwd);
    let mcp_servers = relay.configured_mcp_servers.clone();

    // Reset the current connection so the pool key is free for re-prewarm
    reset_session_connection(relay, &app_session_id, request_id.clone()).await?;

    let (event_tx, mut event_rx) = mpsc::channel(512);
    let (tx, mut rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Prewarm {
            runtime_key: relay.adapter.runtime_key.clone(),
            binary: relay.adapter.binary.clone(),
            args: relay.adapter.args.clone(),
            env: relay.adapter.env.clone(),
            client_capabilities: client_capabilities.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: app_session_id.clone(),
            cwd: cwd.clone(),
            auto_approve: false,
            mcp_servers: mcp_servers.clone(),
            role_mode: None,
            role_config_options: vec![],
            delta_tx: Some(event_tx),
            result_tx: Some(tx),
            resume_session_id: Some(requested_session_id.clone()),
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;

    let mut replay_complete = false;
    let mut replay_result: Option<(Vec<Value>, Vec<String>, String)> = None;
    let (config_options, modes, session_id) = loop {
        // Once `ReplayComplete` has been observed we must NOT poll `event_rx`
        // any longer: the upstream forwarder task drops its sender clone the
        // moment resume finishes, so the channel closes almost immediately
        // after delivering `ReplayComplete`. Polling it after that point
        // produces a spurious `ACP replay channel closed before completion`
        // error and the UI hangs (T-330.3).
        if replay_complete {
            if let Some(result) = replay_result.take() {
                break result;
            }
            match wait_for_worker_result(&mut rx).await {
                Ok(result) => break result,
                Err(error) => {
                    cleanup_failed_session_load(relay, &app_session_id).await;
                    return Err(replay_wait_error_to_rpc(error, request_id.clone()));
                }
            }
        }
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else {
                    cleanup_failed_session_load(relay, &app_session_id).await;
                    return Err(rpc_error(request_id.clone(), -32000, "ACP replay channel closed before completion"));
                };
                if matches!(event, jockey::acp::AcpEvent::ReplayComplete) {
                    replay_complete = true;
                    if let Some(result) = replay_result.take() {
                        break result;
                    }
                    continue;
                }
                if !replay_complete
                    && !forward_event(
                        sender,
                        &requested_session_id,
                        &event,
                        permission_ids,
                        next_permission_id,
                    ).await
                {
                    cleanup_failed_session_load(relay, &app_session_id).await;
                    return Err(rpc_error(request_id.clone(), -32000, "websocket closed during session/load replay"));
                }
            }
            result = &mut rx => {
                let result = result
                    .map_err(|_| {
                        rpc_error(request_id.clone(), -32000, "ACP worker disconnected")
                    });
                let result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        cleanup_failed_session_load(relay, &app_session_id).await;
                        return Err(error);
                    }
                };
                if replay_complete {
                    break result;
                }
                replay_result = Some(result);
            }
        }
    };
    if session_id.is_empty() {
        cleanup_failed_session_load(relay, &app_session_id).await;
        return Err(rpc_error(
            request_id,
            -32603,
            "session/load failed — agent did not return a session",
        ));
    }
    *session = Some(RelaySession {
        app_session_id: app_session_id.clone(),
        session_id: session_id.clone(),
        cwd,
        mcp_servers,
        client_capabilities,
        config_options: config_options.clone(),
        modes: modes.clone(),
        current_mode_id: None,
    });
    let modes = session
        .as_ref()
        .and_then(|active| build_modes(&active.modes, &active.current_mode_id));
    Ok(json!({
        "sessionId": session_id,
        "configOptions": config_options,
        "availableCommands": jockey::acp::list_available_commands(
            &app_session_id,
            &relay.adapter.runtime_key,
            &relay.agent_name,
        ),
        "modes": modes,
    }))
}

async fn cleanup_failed_session_load(relay: &RelayConfig, app_session_id: &str) {
    let _ = reset_session_connection(relay, app_session_id, Value::Null).await;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ReplayWaitError {
    /// Worker dropped the result oneshot before sending a result.
    WorkerDisconnected,
}

fn replay_wait_error_to_rpc(error: ReplayWaitError, request_id: Value) -> Value {
    match error {
        ReplayWaitError::WorkerDisconnected => {
            rpc_error(request_id, -32000, "ACP worker disconnected")
        }
    }
}

/// Wait for the ACP worker's final result oneshot after `ReplayComplete`
/// has already been observed.
///
/// At this point we must NOT poll the upstream event channel: the forwarder
/// task drops its sender clone as soon as the resume finishes, which closes
/// the channel right after `ReplayComplete` is delivered. Polling that closed
/// channel in a `select!` produces a spurious `ACP replay channel closed
/// before completion` error and the UI hangs (T-330.3).
pub(crate) async fn wait_for_worker_result(
    rx: &mut tokio::sync::oneshot::Receiver<(Vec<Value>, Vec<String>, String)>,
) -> Result<(Vec<Value>, Vec<String>, String), ReplayWaitError> {
    match rx.await {
        Ok(result) => Ok(result),
        Err(_) => Err(ReplayWaitError::WorkerDisconnected),
    }
}
