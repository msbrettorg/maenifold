use super::acp_relay::{PendingPrompt, RelayConfig, RelaySession};
use super::acp_relay_caps::{AgentCaps, caps_for_agent};
use super::acp_relay_helpers::{rpc_error, update_config_options};
use agent_client_protocol as acp;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

pub(super) fn handle_prompt(
    relay: &RelayConfig,
    session: &Option<RelaySession>,
    pending_prompt: &mut Option<PendingPrompt>,
    delta_rx: &mut Option<mpsc::Receiver<jockey::acp::AcpEvent>>,
    result_rx: &mut Option<oneshot::Receiver<jockey::acp::AcpExecuteResult>>,
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
    let blocks = prompt_parts(&request, caps_for_agent(&relay.agent_name, &relay.adapter))
        .map_err(|error| rpc_error(request_id.clone(), -32602, error))?;
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
            prompt: prompt_text(&blocks),
            context: Vec::new(),
            prompt_blocks: Some(blocks),
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
fn prompt_parts(
    request: &acp::PromptRequest,
    caps: AgentCaps,
) -> Result<Vec<acp::ContentBlock>, String> {
    let mut blocks = Vec::new();
    let mut has_prompt_content = false;
    for block in &request.prompt {
        match block {
            acp::ContentBlock::Text(text) => {
                if !text.text.trim().is_empty() {
                    has_prompt_content = true;
                }
                blocks.push(block.clone());
            }
            acp::ContentBlock::ResourceLink(_) | acp::ContentBlock::Resource(_) => {
                has_prompt_content = true;
                blocks.push(block.clone());
            }
            acp::ContentBlock::Image(_) => {
                if !caps.image {
                    return Err("image prompt blocks are not supported by the relay".to_string());
                }
                has_prompt_content = true;
                blocks.push(block.clone());
            }
            acp::ContentBlock::Audio(_) => {
                if !caps.audio {
                    return Err("audio prompt blocks are not supported by the relay".to_string());
                }
                has_prompt_content = true;
                blocks.push(block.clone());
            }
            _ => return Err("unsupported prompt block type".to_string()),
        }
    }
    if !has_prompt_content {
        return Err("prompt must contain at least one content block".to_string());
    }
    Ok(blocks)
}

fn prompt_text(blocks: &[acp::ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            acp::ContentBlock::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(image: bool, audio: bool) -> AgentCaps {
        AgentCaps {
            image,
            audio,
            embedded_context: true,
        }
    }

    fn prompt_with_block(block: Value) -> acp::PromptRequest {
        serde_json::from_value(json!({
            "sessionId": "session-1",
            "prompt": [
                { "type": "text", "text": "describe this" },
                block
            ]
        }))
        .expect("valid prompt request")
    }

    #[test]
    fn image_block_is_forwarded_when_cap_enabled() {
        let request = prompt_with_block(json!({
            "type": "image",
            "data": "iVBORw0KGgo=",
            "mimeType": "image/png"
        }));

        let blocks = prompt_parts(&request, caps(true, false)).expect("image accepted");

        assert!(matches!(blocks.get(1), Some(acp::ContentBlock::Image(_))));
    }

    #[test]
    fn image_block_is_rejected_when_cap_disabled() {
        let request = prompt_with_block(json!({
            "type": "image",
            "data": "iVBORw0KGgo=",
            "mimeType": "image/png"
        }));

        let error = prompt_parts(&request, caps(false, false)).expect_err("image rejected");

        assert_eq!(error, "image prompt blocks are not supported by the relay");
    }

    #[test]
    fn audio_block_is_forwarded_when_cap_enabled() {
        let request = prompt_with_block(json!({
            "type": "audio",
            "data": "UklGRg==",
            "mimeType": "audio/wav"
        }));

        let blocks = prompt_parts(&request, caps(false, true)).expect("audio accepted");

        assert!(matches!(blocks.get(1), Some(acp::ContentBlock::Audio(_))));
    }
}
