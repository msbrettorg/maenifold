use super::acp_relay::{RelayConfig, RelaySession, SessionLoadParams, SessionSetupParams};
use super::acp_relay_dispatch::forward_event;
use super::acp_relay_helpers::{build_modes, resolve_session_cwd, rpc_error};
use super::acp_relay_mcp::{merge_mcp_servers, prewarm_session, reset_session_connection};
use agent_client_protocol as acp;
use axum::extract::ws::{Message, WebSocket};
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

pub(super) async fn handle_initialize(
    relay: &RelayConfig,
    session: &mut Option<RelaySession>,
    params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let request: acp::InitializeRequest = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    if session.is_none() {
        let app_session_id = format!("acp-relay:{}", uuid::Uuid::new_v4());
        let client_capabilities = request.client_capabilities.clone();
        let mcp_servers = relay.configured_mcp_servers.clone();
        let (config_options, modes, session_id) = prewarm_session(
            relay,
            &app_session_id,
            relay.default_cwd.clone(),
            mcp_servers.clone(),
            client_capabilities.clone(),
            None,
            HashMap::new(),
            request_id.clone(),
        )
        .await?;
        if session_id.is_empty() {
            return Err(rpc_error(
                request_id,
                -32603,
                "Agent initialization failed — check agent configuration and binary path",
            ));
        }
        *session = Some(RelaySession {
            app_session_id,
            session_id,
            cwd: relay.default_cwd.clone(),
            mcp_servers,
            client_capabilities,
            config_options,
            modes,
            current_mode_id: None,
        });
    } else if let Some(active) = session.as_mut() {
        active.client_capabilities = request.client_capabilities;
    }

    Ok(json!({
        "protocolVersion": 1,
        "agentCapabilities": {
            "loadSession": true,
            "promptCapabilities": { "image": false, "audio": false, "embeddedContext": true },
            "sessionCapabilities": {},
            "_meta": { "maenifold/extensions": { "supportedAgentMethods": ["session/set_config_option"] } }
        },
        "authMethods": [],
        "agentInfo": { "name": relay.agent_name, "title": relay.agent_name, "version": "0.1.0" }
    }))
}

pub(super) async fn handle_session_new(
    relay: &RelayConfig,
    session: &mut Option<RelaySession>,
    params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let Some(active) = session.as_ref() else {
        return Err(rpc_error(
            request_id,
            -32600,
            "initialize must be called before session/new",
        ));
    };
    let app_session_id = active.app_session_id.clone();
    let client_capabilities = active.client_capabilities.clone();
    let request: SessionSetupParams = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    let cwd = resolve_session_cwd(request.cwd, &relay.default_cwd)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    reset_session_connection(relay, &app_session_id, request_id.clone()).await?;
    let mcp_servers = merge_mcp_servers(&relay.configured_mcp_servers, request.mcp_servers);
    let session_config_options = request.config_options;
    let (config_options, modes, session_id) = prewarm_session(
        relay,
        &app_session_id,
        cwd.clone(),
        mcp_servers.clone(),
        client_capabilities,
        None,
        session_config_options,
        request_id.clone(),
    )
    .await?;
    if session_id.is_empty() {
        return Err(rpc_error(
            request_id,
            -32603,
            "session/new failed — agent did not return a session",
        ));
    }
    let active = session
        .as_mut()
        .ok_or_else(|| rpc_error(request_id.clone(), -32600, "relay session disappeared"))?;
    active.session_id = session_id.clone();
    active.cwd = cwd;
    active.mcp_servers = mcp_servers;
    active.config_options = config_options.clone();
    active.modes = modes.clone();
    active.current_mode_id = None;
    let modes = build_modes(&active.modes, &active.current_mode_id);
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
    let request: SessionLoadParams = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
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
    let mcp_servers = merge_mcp_servers(&relay.configured_mcp_servers, request.mcp_servers);

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
            auto_approve: true,
            mcp_servers: mcp_servers.clone(),
            role_mode: None,
            role_config_options: vec![],
            delta_tx: Some(event_tx),
            result_tx: Some(tx),
            resume_session_id: Some(requested_session_id.clone()),
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;

    let mut replay_complete = false;
    let mut replay_result = None;
    let (config_options, modes, session_id) = loop {
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else {
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
                    return Err(rpc_error(request_id.clone(), -32000, "websocket closed during session/load replay"));
                }
            }
            result = &mut rx => {
                let result = result
                    .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker disconnected"))?;
                if replay_complete {
                    break result;
                }
                replay_result = Some(result);
            }
        }
    };
    if session_id.is_empty() {
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
