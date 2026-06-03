use super::acp_relay::{RelayConfig, RelaySession, SessionSetupParams};
use super::acp_relay_caps::{AgentCaps, caps_for_agent};
use super::acp_relay_helpers::{reject_client_mcp_servers, resolve_session_cwd, rpc_error};
use super::acp_relay_mcp::{prewarm_session, reset_session_connection};
use agent_client_protocol::schema::{self as acp};
use serde_json::{Value, json};
use std::collections::HashMap;

// Re-export `handle_session_load` and its replay-wait helpers from the
// sibling `acp_relay_session_load` module so callers can keep importing
// from `acp_relay_session` (and the test module can reach them through
// `use super::*;`). The split keeps this file under the 300-LOC budget.
pub(super) use super::acp_relay_session_load::handle_session_load;
#[cfg(test)]
pub(super) use super::acp_relay_session_load::{ReplayWaitError, wait_for_worker_result};

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
        let (config_options, session_id) = prewarm_session(
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
        });
    } else if let Some(active) = session.as_mut() {
        active.client_capabilities = request.client_capabilities;
    }

    Ok(initialize_response(
        relay,
        caps_for_agent(&relay.agent_name, &relay.adapter),
    ))
}

fn initialize_response(relay: &RelayConfig, caps: AgentCaps) -> Value {
    json!({
        "protocolVersion": 1,
        "agentCapabilities": {
            "loadSession": true,
            "promptCapabilities": {
                "image": caps.image,
                "audio": caps.audio,
                "embeddedContext": caps.embedded_context
            },
            "sessionCapabilities": { "close": true },
            "_meta": { "maenifold/extensions": { "supportedAgentMethods": ["session/set_config_option", "session/close"] } }
        },
        "authMethods": [],
        "agentInfo": { "name": relay.agent_name, "title": relay.agent_name, "version": "0.1.0" }
    })
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
    reject_client_mcp_servers(&params, request_id.clone(), "session/new")?;
    let request: SessionSetupParams = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    let _ = request.mcp_servers.len();
    let cwd = resolve_session_cwd(request.cwd, &relay.default_cwd)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    reset_session_connection(relay, &app_session_id, request_id.clone()).await?;
    let mcp_servers = relay.configured_mcp_servers.clone();
    let (config_options, session_id) = prewarm_session(
        relay,
        &app_session_id,
        cwd.clone(),
        mcp_servers.clone(),
        client_capabilities,
        None,
        request.config_options,
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
    Ok(json!({
        "sessionId": session_id,
        "configOptions": config_options,
    }))
}

#[cfg(test)]
#[path = "acp_relay_session_tests.rs"]
mod tests;
