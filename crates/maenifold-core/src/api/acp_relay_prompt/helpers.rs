use super::super::acp_relay_caps::AgentCaps;
use agent_client_protocol::schema::{self as acp};
use serde_json::Value;

pub(super) fn prompt_parts(
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
            acp::ContentBlock::ResourceLink(_) => {
                has_prompt_content = true;
                blocks.push(block.clone());
            }
            acp::ContentBlock::Resource(_) => {
                if !caps.embedded_context {
                    return Err(
                        "embedded context prompt blocks are not supported by the relay".to_string(),
                    );
                }
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

pub(super) fn prompt_text(blocks: &[acp::ContentBlock]) -> String {
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

pub(super) fn parse_set_config_request(params: Value) -> Result<(String, String, String), String> {
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
    use serde_json::json;

    fn caps(image: bool, audio: bool, embedded_context: bool) -> AgentCaps {
        AgentCaps {
            image,
            audio,
            embedded_context,
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

        let blocks = prompt_parts(&request, caps(true, false, true)).expect("image accepted");

        assert!(matches!(blocks.get(1), Some(acp::ContentBlock::Image(_))));
    }

    #[test]
    fn image_block_is_rejected_when_cap_disabled() {
        let request = prompt_with_block(json!({
            "type": "image",
            "data": "iVBORw0KGgo=",
            "mimeType": "image/png"
        }));

        let error = prompt_parts(&request, caps(false, false, true)).expect_err("image rejected");

        assert_eq!(error, "image prompt blocks are not supported by the relay");
    }

    #[test]
    fn audio_block_is_forwarded_when_cap_enabled() {
        let request = prompt_with_block(json!({
            "type": "audio",
            "data": "UklGRg==",
            "mimeType": "audio/wav"
        }));

        let blocks = prompt_parts(&request, caps(false, true, true)).expect("audio accepted");

        assert!(matches!(blocks.get(1), Some(acp::ContentBlock::Audio(_))));
    }

    #[test]
    fn resource_link_block_is_forwarded_without_embedded_context_cap() {
        let request = prompt_with_block(json!({
            "type": "resource_link",
            "name": "repo-readme",
            "uri": "file:///workspace/README.md"
        }));

        let blocks =
            prompt_parts(&request, caps(false, false, false)).expect("resource link accepted");

        assert!(matches!(
            blocks.get(1),
            Some(acp::ContentBlock::ResourceLink(_))
        ));
    }

    #[test]
    fn resource_block_is_forwarded_when_embedded_context_cap_enabled() {
        let request = prompt_with_block(json!({
            "type": "resource",
            "resource": {
                "uri": "file:///workspace/README.md",
                "text": "# README",
                "mimeType": "text/markdown"
            }
        }));

        let blocks =
            prompt_parts(&request, caps(false, false, true)).expect("embedded resource accepted");

        assert!(matches!(
            blocks.get(1),
            Some(acp::ContentBlock::Resource(_))
        ));
    }

    #[test]
    fn resource_block_is_rejected_when_embedded_context_cap_disabled() {
        let request = prompt_with_block(json!({
            "type": "resource",
            "resource": {
                "uri": "file:///workspace/README.md",
                "text": "# README",
                "mimeType": "text/markdown"
            }
        }));

        let error = prompt_parts(&request, caps(false, false, false))
            .expect_err("embedded resource rejected");

        assert_eq!(
            error,
            "embedded context prompt blocks are not supported by the relay"
        );
    }
}
