use std::collections::BTreeSet;

use rmcp::model::{
    CreateMessageResult, JsonObject, Role, SamplingContent, SamplingMessage, SamplingMessageContent,
};
use serde_json::Value;

pub(crate) fn incoming_tool_result_ids(messages: &[SamplingMessage]) -> BTreeSet<String> {
    messages
        .iter()
        .flat_map(|message| message.content.iter())
        .filter_map(|content| {
            content
                .as_tool_result()
                .map(|tool_result| tool_result.tool_use_id.clone())
        })
        .collect()
}

pub(crate) fn sampling_messages_to_prompt(
    messages: &[SamplingMessage],
) -> Result<String, rmcp::ErrorData> {
    let mut prompt = String::new();
    for message in messages {
        if !prompt.is_empty() {
            prompt.push('\n');
        }
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        prompt.push_str(role);
        prompt.push_str(": ");
        match &message.content {
            SamplingContent::Single(content) => {
                prompt.push_str(&sampling_content_text(content)?);
            }
            SamplingContent::Multiple(contents) => {
                let mut parts = Vec::new();
                for content in contents {
                    parts.push(sampling_content_text(content)?);
                }
                prompt.push_str(&parts.join("\n"));
            }
        }
    }
    Ok(prompt)
}

pub(crate) fn text_result(text: String, model: String, stop_reason: &str) -> CreateMessageResult {
    CreateMessageResult::new(SamplingMessage::new(Role::Assistant, text), model)
        .with_stop_reason(stop_reason)
}

pub(crate) fn tool_use_result(
    blocks: Vec<jockey::acp::ToolUseBlock>,
    model: String,
) -> CreateMessageResult {
    let contents = blocks
        .into_iter()
        .map(|block| {
            SamplingMessageContent::tool_use(block.id, block.name, json_object(block.input))
        })
        .collect();
    CreateMessageResult::new(
        SamplingMessage::new_multiple(Role::Assistant, contents),
        model,
    )
    .with_stop_reason(CreateMessageResult::STOP_REASON_TOOL_USE)
}

pub(crate) fn tool_use_ids(blocks: &[jockey::acp::ToolUseBlock]) -> BTreeSet<String> {
    blocks.iter().map(|block| block.id.clone()).collect()
}

fn sampling_content_text(content: &SamplingMessageContent) -> Result<String, rmcp::ErrorData> {
    match content {
        SamplingMessageContent::Text(text) => Ok(text.text.clone()),
        SamplingMessageContent::ToolResult(tool_result) => serde_json::to_string(tool_result)
            .map_err(|error| rmcp::ErrorData::invalid_params(error.to_string(), None)),
        SamplingMessageContent::ToolUse(tool_use) => serde_json::to_string(tool_use)
            .map_err(|error| rmcp::ErrorData::invalid_params(error.to_string(), None)),
        _ => Err(rmcp::ErrorData::invalid_params(
            "Only text and tool sampling message content is supported.",
            None,
        )),
    }
}

fn json_object(value: Value) -> JsonObject {
    match value {
        Value::Object(map) => map,
        other => {
            let mut map = JsonObject::new();
            map.insert("value".to_string(), other);
            map
        }
    }
}
