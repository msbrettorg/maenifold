use super::{MCP_PROTOCOL_VERSION, build_headers};
use reqwest::header::CONTENT_TYPE;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::timeout;

fn parse_sse_payload(body: &str, expected_id: u64) -> Result<Value, String> {
    body.split("\n\n")
        .flat_map(|chunk| chunk.split("\r\n\r\n"))
        .filter_map(|block| {
            let data = block
                .lines()
                .map(str::trim)
                .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>();
            if data.is_empty() {
                None
            } else {
                Some(data.join("\n"))
            }
        })
        .filter_map(|candidate| serde_json::from_str::<Value>(&candidate).ok())
        .find(|parsed| parsed.get("id").and_then(Value::as_u64) == Some(expected_id))
        .ok_or_else(|| {
            format!("no valid JSON-RPC response found in SSE stream for request id {expected_id}")
        })
}

async fn parse_http_payload(
    response: reqwest::Response,
    expected_id: u64,
) -> Result<Value, String> {
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let payload = if content_type.contains("text/event-stream") {
        let body = response
            .text()
            .await
            .map_err(|error| format!("failed to read SSE body: {error}"))?;
        parse_sse_payload(&body, expected_id)?
    } else {
        response
            .json::<Value>()
            .await
            .map_err(|error| format!("failed to parse JSON body: {error}"))?
    };

    if payload.get("id").and_then(Value::as_u64) != Some(expected_id) {
        return Err(format!(
            "unexpected JSON-RPC response id for request {expected_id}: {payload}"
        ));
    }
    if let Some(error) = payload.get("error").filter(|error| !error.is_null()) {
        return Err(format!("JSON-RPC error: {error}"));
    }
    Ok(payload)
}

pub(super) async fn discover_http_tools(
    url: &str,
    timeout_duration: Duration,
) -> Result<Vec<Value>, String> {
    timeout(timeout_duration, discover_http_tools_inner(url, timeout_duration))
        .await
        .map_err(|_| "timed out waiting for HTTP MCP server (10 s total budget exceeded)".to_string())?
}

async fn discover_http_tools_inner(
    url: &str,
    timeout_duration: Duration,
) -> Result<Vec<Value>, String> {
    let client = reqwest::Client::builder()
        .timeout(timeout_duration)
        .build()
        .map_err(|error| format!("failed to build HTTP client: {error}"))?;

    let initialize = client
        .post(url)
        .headers(build_headers(None)?)
        .json(&json!({"jsonrpc":"2.0","id":1u64,"method":"initialize","params":{"protocolVersion":MCP_PROTOCOL_VERSION,"capabilities":{},"clientInfo":{"name":"maenifold-core","version":"0.1.0"}}}))
        .send()
        .await
        .map_err(|error| format!("initialize request failed: {error}"))?;
    let session_id = initialize
        .headers()
        .get("Mcp-Session-Id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    if !initialize.status().is_success() {
        return Err(format!(
            "initialize failed with HTTP {}",
            initialize.status()
        ));
    }
    parse_http_payload(initialize, 1).await?;

    let initialized = client
        .post(url)
        .headers(build_headers(session_id.as_deref())?)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .map_err(|error| format!("initialized notification failed: {error}"))?;
    if !initialized.status().is_success() {
        return Err(format!(
            "initialized notification failed with HTTP {}",
            initialized.status()
        ));
    }

    let tools_response = client
        .post(url)
        .headers(build_headers(session_id.as_deref())?)
        .json(&json!({"jsonrpc":"2.0","id":2u64,"method":"tools/list"}))
        .send()
        .await
        .map_err(|error| format!("tools/list request failed: {error}"))?;
    if !tools_response.status().is_success() {
        return Err(format!(
            "tools/list failed with HTTP {}",
            tools_response.status()
        ));
    }

    let payload = parse_http_payload(tools_response, 2).await?;
    let tools = payload
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "tools/list response missing result.tools".to_string())?;

    if let Some(session_id) = session_id {
        let response = client
            .delete(url)
            .header("Accept", "application/json, text/event-stream")
            .header("Mcp-Session-Id", session_id)
            .send()
            .await
            .map_err(|error| format!("session delete failed: {error}"))?;
        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(format!("session delete failed with HTTP {status}"));
        }
    }

    Ok(tools)
}
