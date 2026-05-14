use serde_json::Value;
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq)]
pub struct SamplingResult {
    pub text: String,
    pub acp_session_id: String,
    pub stop_reason: SamplingStopReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SamplingStopReason {
    EndTurn,
    ToolUse(Vec<ToolUseBlock>),
    MaxTokens,
    StopSequence(String),
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
}

pub async fn execute_runtime_sampling(
    runtime_kind: &str,
    role_name: &str,
    app_session_id: uuid::Uuid,
    prompt: String,
    selected_model: Option<String>,
    session_id_tx: oneshot::Sender<String>,
    event_tx: crate::EventSender,
    resume_session_id: Option<String>,
    tools: Option<Vec<rmcp::model::Tool>>,
) -> Result<SamplingResult, String> {
    let cwd = std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".to_string());

    let role_config_options: Vec<(String, String)> = if let Some(m) = selected_model {
        vec![("model".to_string(), m)]
    } else {
        vec![]
    };

    let prompt = sampling_prompt_with_tools(prompt, tools.as_ref())?;
    let result = super::execute::execute_runtime(
        runtime_kind,
        role_name,
        &prompt,
        &[],
        &cwd,
        &event_tx,
        true,
        vec![],
        None,
        role_config_options,
        None,
        &app_session_id.to_string(),
        resume_session_id,
        Some(session_id_tx),
    )
    .await;

    if result.ok {
        let acp_session_id = result
            .meta
            .get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok(SamplingResult {
            stop_reason: sampling_stop_reason(&result.meta, &result.output),
            text: result.output,
            acp_session_id,
        })
    } else {
        Err(result.error_code.unwrap_or(result.output))
    }
}

fn sampling_prompt_with_tools(
    prompt: String,
    tools: Option<&Vec<rmcp::model::Tool>>,
) -> Result<String, String> {
    let Some(tools) = tools.filter(|items| !items.is_empty()) else {
        return Ok(prompt);
    };
    let tools_json = serde_json::to_string_pretty(tools)
        .map_err(|error| format!("failed to serialize sampling tools: {error}"))?;
    // agent-client-protocol 0.10.4 PromptRequest has prompt blocks but no native
    // tool-definition field, so MCP sampling tools are exposed as a textual
    // preamble that the selected ACP agent can use to emit tool-use JSON.
    Ok(format!(
        "Available MCP sampling tools:\n```json\n{tools_json}\n```\n\nWhen a tool call is needed, respond with JSON content blocks shaped as {{\"type\":\"tool_use\",\"id\":\"...\",\"name\":\"...\",\"input\":{{...}}}}.\n\n{prompt}"
    ))
}

fn sampling_stop_reason(meta: &Value, output: &str) -> SamplingStopReason {
    if let Some(blocks) = parse_tool_use_blocks(output) {
        return SamplingStopReason::ToolUse(blocks);
    }
    match meta.get("stopReason").and_then(Value::as_str) {
        Some("EndTurn") | None => SamplingStopReason::EndTurn,
        Some("MaxTokens") => SamplingStopReason::MaxTokens,
        Some(raw) => SamplingStopReason::Unknown(raw.to_string()),
    }
}

fn parse_tool_use_blocks(output: &str) -> Option<Vec<ToolUseBlock>> {
    let value: Value = serde_json::from_str(output.trim()).ok()?;
    let values = match value {
        Value::Array(items) => items,
        single => vec![single],
    };
    let blocks: Vec<_> = values
        .into_iter()
        .filter_map(|value| {
            let object = value.as_object()?;
            if object.get("type").and_then(Value::as_str) != Some("tool_use") {
                return None;
            }
            Some(ToolUseBlock {
                id: object.get("id")?.as_str()?.to_string(),
                name: object.get("name")?.as_str()?.to_string(),
                input: object.get("input").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    (!blocks.is_empty()).then_some(blocks)
}
