use super::acp_relay::{PendingPrompt, RelayConfig, RelaySession};
use super::acp_relay_helpers::{rpc_error, update_config_options};
use agent_client_protocol as acp;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

pub(super) fn handle_prompt(
    relay: &RelayConfig,
    session: &Option<RelaySession>,
    pending_prompt: &mut Option<PendingPrompt>,
    delta_rx: &mut Option<mpsc::Receiver<jockey::acp::AcpEvent>>,
    result_rx: &mut Option<oneshot::Receiver<Result<(String, String), String>>>,
    params: Value,
    request_id: Value,
) -> Result<(), Value> {
    if pending_prompt.is_some() {
        return Err(rpc_error(
            request_id,
            -32000,
            "session/prompt already in progress",
        ));
    }
    let Some(active) = session.as_ref() else {
        return Err(rpc_error(
            request_id,
            -32600,
            "initialize must be called before session/prompt",
        ));
    };
    let request: acp::PromptRequest = serde_json::from_value(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error.to_string()))?;
    if request.session_id.to_string() != active.session_id {
        return Err(rpc_error(
            request_id,
            -32602,
            "sessionId does not match active session",
        ));
    }
    let (prompt, context) =
        prompt_parts(&request).map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    let (event_tx, event_rx) = mpsc::channel(512);
    let (tx, rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Execute {
            runtime_key: relay.adapter.runtime_key.clone(),
            binary: relay.adapter.binary.clone(),
            args: relay.adapter.args.clone(),
            env: relay.adapter.env.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: active.app_session_id.clone(),
            prompt,
            context,
            cwd: active.cwd.clone(),
            delta_tx: event_tx,
            result_tx: tx,
            // Design decision: auto_approve is always true for the single-user desktop app.
            // The user is the operator — click-through permissions on every fs/write would
            // make pipelines unusable. When multi-user support is needed, this should be
            // configurable via relay settings.
            auto_approve: true,
            mcp_servers: active.mcp_servers.clone(),
            client_capabilities: active.client_capabilities.clone(),
            role_mode: None,
            role_config_options: vec![],
            resume_session_id: None,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    *pending_prompt = Some(PendingPrompt {
        request_id: request_id.clone(),
        cancelled: false,
    });
    *delta_rx = Some(event_rx);
    *result_rx = Some(rx);
    Ok(())
}

pub(super) async fn handle_set_config_option(
    relay: &RelayConfig,
    session: &mut Option<RelaySession>,
    params: Value,
    request_id: Value,
) -> Result<Value, Value> {
    let Some(active) = session.as_mut() else {
        return Err(rpc_error(
            request_id,
            -32600,
            "initialize must be called before session/set_config_option",
        ));
    };
    let request = parse_set_config_request(params)
        .map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
    if request.0 != active.session_id {
        return Err(rpc_error(
            request_id.clone(),
            -32602,
            "sessionId does not match active session",
        ));
    }
    let (tx, rx) = oneshot::channel();
    jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::SetConfigOption {
            runtime_key: relay.adapter.runtime_key.clone(),
            role_name: relay.agent_name.clone(),
            app_session_id: active.app_session_id.clone(),
            config_id: request.1.clone(),
            value: request.2.clone(),
            result_tx: tx,
        })
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker unavailable"))?;
    rx.await
        .map_err(|_| rpc_error(request_id.clone(), -32000, "ACP worker disconnected"))?
        .map_err(|error| rpc_error(request_id.clone(), -32000, error))?;
    let category = update_config_options(&mut active.config_options, &request.1, &request.2);
    Ok(json!({
        "configOptions": active.config_options,
        "updatedCategory": category,
    }))
}
fn prompt_parts(request: &acp::PromptRequest) -> Result<(String, Vec<(String, String)>), String> {
    let mut prompt = Vec::new();
    let mut context = Vec::new();
    for (index, block) in request.prompt.iter().enumerate() {
        match block {
            acp::ContentBlock::Text(text) => prompt.push(text.text.clone()),
            acp::ContentBlock::ResourceLink(_) | acp::ContentBlock::Resource(_) => {
                context.push((
                    format!("attachment-{}", index + 1),
                    serde_json::to_string(block).map_err(|error| error.to_string())?,
                ));
            }
            acp::ContentBlock::Image(_) => {
                return Err("image prompt blocks are not supported by the relay".to_string());
            }
            acp::ContentBlock::Audio(_) => {
                return Err("audio prompt blocks are not supported by the relay".to_string());
            }
            _ => return Err("unsupported prompt block type".to_string()),
        }
    }
    let prompt = prompt.join("\n").trim().to_string();
    if prompt.is_empty() {
        return Err("prompt must contain at least one text block".to_string());
    }
    Ok((prompt, context))
}

fn parse_set_config_request(params: Value) -> Result<(String, String, String), String> {
    let session_id = params
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or("missing sessionId")?;
    let config_id = params
        .get("configId")
        .and_then(Value::as_str)
        .ok_or("missing configId")?;
    let value = match params.get("value") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => return Err("value must be a string or boolean".to_string()),
    };
    Ok((session_id.to_string(), config_id.to_string(), value))
}
