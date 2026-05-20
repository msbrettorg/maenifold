//! Direct inferencing resolver for `GatewayIntent`.
//!
//! Calls a configured LLM backend (Azure OpenAI / OpenAI) over HTTP via
//! `reqwest` and returns a validated `ResolverOutput`.
//!
//! This is the sole resolver path for `GatewayIntent` after T-331.1.
//! **ACP/Jockey is never reachable from this file.**

use std::time::Duration;

use serde_json::{Value, json};

use crate::acp_config::{IntentResolverSettings, ResolvedInference};
use crate::mcp::gateway_catalog::GatewayCatalog;
use crate::mcp::gateway_resolver::{fallback, parse_and_validate_output, resolver_prompt};
use crate::mcp::gateway_resolver_types::{ResolverError, ResolverOutput, ResolverResult};

const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

pub(crate) async fn resolve_intent_with_direct_inference(
    catalog: &GatewayCatalog,
    settings: &IntentResolverSettings,
    user_intent: &str,
) -> ResolverResult<ResolverOutput> {
    if !settings.enabled {
        return Err(ResolverError::Disabled {
            fallback: fallback(),
        });
    }
    let resolved = settings.resolved_direct_inference().map_err(|message| {
        ResolverError::DirectInferenceSetup {
            fallback: format!("{} ({message})", fallback()),
        }
    })?;
    let prompt = resolver_prompt(catalog, settings, user_intent);
    let body = build_request_body(&resolved, &prompt);
    let timeout_seconds = settings.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
        .map_err(|err| ResolverError::DirectInferenceFailed {
            message: format!("failed to build http client: {err}"),
            fallback: fallback(),
        })?;
    let (_url, request) = build_request(&client, &resolved, &body);
    let response = request.send().await.map_err(|err| {
        if err.is_timeout() {
            ResolverError::Timeout {
                seconds: timeout_seconds,
                fallback: fallback(),
            }
        } else {
            ResolverError::DirectInferenceFailed {
                message: format!("direct inference HTTP request failed: {err}"),
                fallback: fallback(),
            }
        }
    })?;
    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|err| ResolverError::DirectInferenceFailed {
            message: format!("failed to read response body: {err}"),
            fallback: fallback(),
        })?;
    if !status.is_success() {
        return Err(ResolverError::DirectInferenceFailed {
            message: format!(
                "direct inference returned HTTP {}: {body_text}",
                status.as_u16()
            ),
            fallback: fallback(),
        });
    }
    let raw: Value = serde_json::from_str(&body_text).map_err(|err| ResolverError::Malformed {
        message: format!("response was not JSON: {err}"),
        fallback: fallback(),
    })?;
    let assistant_text = extract_assistant_text(&raw)?;
    parse_and_validate_output(catalog, &assistant_text, settings.max_candidates)
}

fn build_request_body(resolved: &ResolvedInference, prompt: &str) -> Value {
    let mut body = json!({
        "messages": [{ "role": "user", "content": prompt }],
        "temperature": 0.0,
        "max_completion_tokens": 2048,
    });
    if resolved.provider != "azureOpenAI" {
        // OpenAI requires `model`; Azure routes by deployment in the URL.
        body["model"] = json!(resolved.deployment);
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

fn extract_assistant_text(raw: &Value) -> ResolverResult<String> {
    let content = raw
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| ResolverError::Malformed {
            message: "response missing /choices/0/message/content".to_string(),
            fallback: fallback(),
        })?;
    if content.trim().is_empty() {
        return Err(ResolverError::Malformed {
            message: "response /choices/0/message/content was empty".to_string(),
            fallback: fallback(),
        });
    }
    Ok(content.to_string())
}
