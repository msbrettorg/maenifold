use serde_json::Value;

use super::{AcpxError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallRecord {
    pub id: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub arguments: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcpxResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub error: Option<String>,
}

pub(super) fn parse_ndjson_response(stdout: &str) -> Result<AcpxResponse> {
    let mut response = AcpxResponse::default();

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let event: Value = serde_json::from_str(line)
            .map_err(|error| AcpxError::InvalidEvent(format!("{error}; line={line}")))?;

        // JSON-RPC session/update envelopes from acpx
        if event.get("jsonrpc").is_some() {
            if let Some("session/update") = event.get("method").and_then(Value::as_str) {
                handle_session_update(&event, &mut response);
                continue;
            }
            if let Some(error_obj) = event.get("error") {
                response.error = Some(
                    error_obj
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Unknown JSON-RPC error")
                        .to_string(),
                );
                continue;
            }
            continue;
        }

        // Flat events: {"type":"text"}, {"type":"tool_call"}, {"type":"error"}
        match event.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(chunk) = extract_text_chunk(&event) {
                    response.text.push_str(&chunk);
                }
            }
            Some("tool_call") => response.tool_calls.push(parse_tool_call_record(&event)),
            Some("error") => {
                response.error = extract_error_text(&event).or_else(|| Some(line.to_string()));
            }
            _ => {}
        }
    }

    Ok(response)
}

fn handle_session_update(event: &Value, response: &mut AcpxResponse) {
    let Some(update) = event.pointer("/params/update") else {
        return;
    };
    match update.get("sessionUpdate").and_then(Value::as_str) {
        Some("agent_message" | "agent_message_chunk" | "agent_thought_chunk") => {
            if let Some(text) = update.pointer("/content/text").and_then(Value::as_str) {
                response.text.push_str(text);
            }
        }
        Some("agent_error") => {
            response.error =
                update.pointer("/content/text").and_then(Value::as_str).map(String::from);
        }
        _ => {}
    }
}

fn parse_tool_call_record(event: &Value) -> ToolCallRecord {
    ToolCallRecord {
        id: extract_optional_string(event, &["id"]),
        name: extract_optional_string(event, &["name", "tool_name"]),
        title: extract_optional_string(event, &["title"]),
        status: extract_optional_string(event, &["status"]),
        arguments: event
            .get("arguments")
            .cloned()
            .or_else(|| event.get("input").cloned())
            .or_else(|| event.get("data").cloned()),
    }
}

fn extract_text_chunk(event: &Value) -> Option<String> {
    extract_optional_string(event, &["text"]).or_else(|| {
        event.get("data").and_then(|data| match data {
            Value::String(text) => Some(text.clone()),
            Value::Object(_) => extract_optional_string(data, &["text", "content", "message"]),
            _ => None,
        })
    })
}

fn extract_error_text(event: &Value) -> Option<String> {
    extract_optional_string(event, &["error", "message", "text"]).or_else(|| {
        event.get("data").and_then(|data| match data {
            Value::String(text) => Some(text.clone()),
            Value::Object(_) => extract_optional_string(data, &["error", "message", "text"]),
            _ => None,
        })
    })
}

fn extract_optional_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|candidate| match candidate {
            Value::String(text) => Some(text.clone()),
            _ => None,
        })
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ToolCallRecord, parse_ndjson_response};
    use crate::acpx_driver::AcpxError;

    #[test]
    fn acpx_driver_parses_ndjson_text_tool_calls_and_errors() {
        let stdout = concat!(
            "{\"type\":\"text\",\"text\":\"Hello\"}\n",
            "{\"type\":\"tool_call\",\"id\":\"call-1\",\"name\":\"ReadMemory\",\"title\":\"Read memory\",\"status\":\"completed\",\"arguments\":{\"uri\":\"memory://foo\"}}\n",
            "{\"type\":\"text\",\"data\":{\"text\":\" world\"}}\n",
            "{\"type\":\"error\",\"message\":\"agent reported a warning\"}\n"
        );

        let response = parse_ndjson_response(stdout).expect("NDJSON should parse");

        assert_eq!(response.text, "Hello world");
        assert_eq!(response.error.as_deref(), Some("agent reported a warning"));
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(
            response.tool_calls[0],
            ToolCallRecord {
                id: Some("call-1".to_string()),
                name: Some("ReadMemory".to_string()),
                title: Some("Read memory".to_string()),
                status: Some("completed".to_string()),
                arguments: Some(json!({"uri": "memory://foo"})),
            }
        );
    }

    #[test]
    fn acpx_driver_rejects_invalid_ndjson_lines() {
        let error = parse_ndjson_response("not-json\n").expect_err("invalid NDJSON must fail");
        assert!(matches!(error, AcpxError::InvalidEvent(_)));
    }

    #[test]
    fn acpx_driver_parses_jsonrpc_agent_message() {
        let stdout = r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1","update":{"sessionUpdate":"agent_message","content":{"type":"text","text":"Hello from RPC"}}}}"#;
        let response = parse_ndjson_response(stdout).expect("should parse");
        assert_eq!(response.text, "Hello from RPC");
        assert!(response.error.is_none());
    }

    #[test]
    fn acpx_driver_parses_jsonrpc_agent_thought_chunk() {
        let stdout = concat!(
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1","update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"Thinking..."}}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1","update":{"sessionUpdate":"agent_message","content":{"type":"text","text":" Done."}}}}"#,
        );
        let response = parse_ndjson_response(stdout).expect("should parse");
        assert_eq!(response.text, "Thinking... Done.");
    }

    #[test]
    fn acpx_driver_parses_jsonrpc_error() {
        let stdout =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid request"},"id":1}"#;
        let response = parse_ndjson_response(stdout).expect("should parse");
        assert_eq!(response.error.as_deref(), Some("Invalid request"));
        assert!(response.text.is_empty());
    }

    #[test]
    fn acpx_driver_parses_mixed_flat_and_jsonrpc_events() {
        let stdout = concat!(
            "{\"type\":\"text\",\"text\":\"Flat hello\"}\n",
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1","update":{"sessionUpdate":"agent_message","content":{"type":"text","text":" RPC hello"}}}}"#,
            "\n",
            "{\"type\":\"text\",\"text\":\" again\"}\n",
        );
        let response = parse_ndjson_response(stdout).expect("should parse");
        assert_eq!(response.text, "Flat hello RPC hello again");
        assert!(response.error.is_none());
    }
}
