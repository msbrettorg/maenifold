use crate::error::{
    GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, JSONRPC_INVALID_PARAMS, MaenifoldError,
};
use crate::mcp::tool_utils::format_tool_error;
use crate::registry::ToolRegistry;
use crate::tools::{RequestCancelOnDrop, with_cancellation_scope};
use crate::util::json::OutputFormat;
use rmcp::model::{CallToolResult, Content, ErrorCode, Tool};
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(super) async fn invoke_native(
    tool_name: String,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let display_name = tool_name.clone();
    let cancellation_token = CancellationToken::new();
    let _cancel_on_drop = RequestCancelOnDrop(cancellation_token.clone());
    let result = tokio::task::spawn_blocking(move || {
        with_cancellation_scope(cancellation_token, || {
            ToolRegistry::global().try_invoke(&tool_name, &parameters, OutputFormat::Json)
        })
    })
    .await
    .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;
    match result {
        Ok(output) if envelope_indicates_failure(&output) => {
            Ok(CallToolResult::error(vec![Content::text(output)]))
        }
        Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
        Err(err) => Ok(CallToolResult::error(vec![Content::text(
            format_tool_error(&display_name, &err),
        )])),
    }
}

pub(super) fn hidden_tool_message(tool_name: &str, domain: &str) -> String {
    if tool_name.starts_with("gateway_") {
        return format!(
            "Gateway facade '{tool_name}' is retired. Use GatewayIndex with domain='{domain}' to inspect commands, then GatewayExecute."
        );
    }
    format!(
        "Tool '{tool_name}' is routed under domain '{domain}'. Use GatewayExecute with domain='{domain}', command='{tool_name}'."
    )
}

pub(super) fn map_gateway_error(tool: &str, err: MaenifoldError) -> rmcp::ErrorData {
    match err {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => rmcp::ErrorData::new(
            ErrorCode(i32::try_from(code).unwrap_or(JSONRPC_INTERNAL_ERROR as i32)),
            message,
            Some(data),
        ),
        MaenifoldError::GatewayInvalidParams(message) => rmcp::ErrorData::new(
            ErrorCode(JSONRPC_INVALID_PARAMS as i32),
            message,
            Some(Value::Null),
        ),
        _ => rmcp::ErrorData::internal_error(format_tool_error(tool, &err), None),
    }
}

pub(super) fn log_gateway_error(server: Option<&str>, tool: &str, err: &MaenifoldError) {
    let server = server.unwrap_or("");
    match err {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } if *code == JSONRPC_INTERNAL_ERROR || *code == GATEWAY_TIMEOUT_ERROR => {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) {
                tracing::error!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed");
            } else {
                tracing::error!(server, tool, err = %message, code, "gateway tool call failed");
            }
        }
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) {
                tracing::warn!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed");
            } else {
                tracing::warn!(server, tool, err = %message, code, "gateway tool call failed");
            }
        }
        _ => tracing::warn!(server, tool, err = %err, "gateway tool call failed"),
    }
}

pub(super) fn render_json(value: Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}

pub(super) fn tool_from_definition(definition: crate::mcp::McpToolDefinition) -> Tool {
    let schema = match definition.input_schema {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    let mut tool = Tool::new(definition.name, definition.description, Arc::new(schema));
    tool.execution = definition.execution;
    tool
}

fn envelope_indicates_failure(text: &str) -> bool {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.get("success").and_then(Value::as_bool).map(|b| !b))
        .unwrap_or(false)
}
