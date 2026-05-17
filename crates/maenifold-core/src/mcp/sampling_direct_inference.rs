//! Direct-inference dispatcher for `sampling/createMessage`.
//!
//! When a downstream MCP server (an upstream stdio server gateway-mounted into
//! maenifold) calls `sampling/createMessage`, the daemon fulfills the request
//! by calling the configured inference backend over HTTP. ACP / Jockey is
//! NEVER used here — same boundary as `GatewayIntent` (`AGENTS.md` P0 #7).
//!
//! Returns a result shape compatible with the existing `SamplingStopReason`
//! enum so the rest of the sampling pipeline (tool-loop correlation, registry
//! response payload, MCP `CreateMessageResult` construction) is unchanged.

use rmcp::model::Tool;
use serde_json::{Value, json};

use crate::acp_config::ResolvedInference;
use jockey::acp::{SamplingResult, SamplingStopReason, ToolUseBlock};

const DEFAULT_TIMEOUT_SECONDS: u64 = 60;
const DEFAULT_MAX_TOKENS: u64 = 4096;

pub(crate) async fn execute_direct_inference_sampling(
    resolved: &ResolvedInference,
    prompt: String,
    selected_model: Option<String>,
    tools: Option<Vec<Tool>>,
    max_tokens: Option<u64>,
) -> Result<SamplingResult, String> {
    let effective_deployment = selected_model.unwrap_or_else(|| resolved.deployment.clone());
    let effective_resolved = ResolvedInference {
        deployment: effective_deployment.clone(),
        ..resolved.clone()
    };
    let request_body = build_chat_completions_body(
        &effective_resolved,
        prompt,
        tools.as_ref(),
        max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
        .build()
        .map_err(|err| format!("failed to build http client: {err}"))?;
    let (url, request) = build_request(&client, &effective_resolved, &request_body);
    let response = request
        .send()
        .await
        .map_err(|err| format!("direct inference HTTP request failed ({url}): {err}"))?;
    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|err| format!("failed to read direct inference response body: {err}"))?;
    if !status.is_success() {
        return Err(format!(
            "direct inference returned HTTP {} ({url}): {body_text}",
            status.as_u16()
        ));
    }
    let raw: Value = serde_json::from_str(&body_text)
        .map_err(|err| format!("direct inference response not JSON: {err}: {body_text}"))?;
    parse_chat_completion(&raw)
}

fn build_chat_completions_body(
    resolved: &ResolvedInference,
    prompt: String,
    tools: Option<&Vec<Tool>>,
    max_tokens: u64,
) -> Value {
    let mut body = json!({
        "messages": [{ "role": "user", "content": prompt }],
        "max_tokens": max_tokens,
    });
    if resolved.provider != "azureOpenAI" {
        body["model"] = json!(resolved.deployment);
    }
    if let Some(tool_list) = tools.filter(|items| !items.is_empty()) {
        body["tools"] = json!(
            tool_list
                .iter()
                .map(|tool| {
                    let description = tool.description.as_deref().unwrap_or("");
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name.as_ref(),
                            "description": description,
                            "parameters": *tool.input_schema,
                        }
                    })
                })
                .collect::<Vec<_>>()
        );
    }
    body
}

fn build_request(
    client: &reqwest::Client,
    resolved: &ResolvedInference,
    body: &Value,
) -> (String, reqwest::RequestBuilder) {
    if resolved.provider == "azureOpenAI" {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            resolved.endpoint, resolved.deployment, resolved.api_version
        );
        let builder = client
            .post(&url)
            .header("api-key", resolved.api_key.as_str())
            .header("content-type", "application/json")
            .json(body);
        (url, builder)
    } else {
        let url = format!("{}/chat/completions", resolved.endpoint);
        let builder = client
            .post(&url)
            .bearer_auth(resolved.api_key.as_str())
            .header("content-type", "application/json")
            .json(body);
        (url, builder)
    }
}

fn parse_chat_completion(raw: &Value) -> Result<SamplingResult, String> {
    let choice = raw
        .pointer("/choices/0")
        .ok_or_else(|| "response missing /choices/0".to_string())?;
    let message = choice
        .pointer("/message")
        .ok_or_else(|| "response missing /choices/0/message".to_string())?;
    let finish_reason = choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .unwrap_or("stop")
        .to_string();
    let text = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let tool_calls = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let stop_reason = if !tool_calls.is_empty() {
        let blocks: Vec<ToolUseBlock> = tool_calls
            .into_iter()
            .filter_map(|call| {
                let id = call.get("id").and_then(Value::as_str)?.to_string();
                let function = call.get("function")?;
                let name = function.get("name").and_then(Value::as_str)?.to_string();
                let raw_args = function
                    .get("arguments")
                    .and_then(Value::as_str)
                    .unwrap_or("{}");
                let input: Value = serde_json::from_str(raw_args).unwrap_or(json!({}));
                Some(ToolUseBlock { id, name, input })
            })
            .collect();
        SamplingStopReason::ToolUse(blocks)
    } else {
        match finish_reason.as_str() {
            "length" => SamplingStopReason::MaxTokens,
            "stop" => SamplingStopReason::EndTurn,
            other => SamplingStopReason::Unknown(other.to_string()),
        }
    };
    Ok(SamplingResult {
        text,
        // No ACP session backs direct inference; expose synthetic id for
        // observability registries that still want a non-empty string.
        acp_session_id: format!("direct-inference-{}", uuid::Uuid::new_v4()),
        stop_reason,
    })
}
