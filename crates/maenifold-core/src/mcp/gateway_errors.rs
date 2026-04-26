use crate::error::{
    GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, JSONRPC_INVALID_PARAMS, MaenifoldError,
};
use crate::mcp::tool_utils::format_tool_error;
use serde_json::Value;
use tracing::{error, warn};

pub(crate) fn invalid_params(error: serde_json::Error) -> MaenifoldError {
    MaenifoldError::GatewayInvalidParams(error.to_string())
}

pub(crate) fn hidden_tool_message(tool_name: &str, domain: &str) -> String {
    if tool_name.starts_with("gateway_") {
        return format!(
            "Gateway facade '{tool_name}' is retired. Use GatewayIndex with domain='{domain}' to inspect commands, then GatewayExecute."
        );
    }
    format!(
        "Tool '{tool_name}' is routed under domain '{domain}'. Use GatewayExecute with domain='{domain}', command='{tool_name}'."
    )
}

pub(crate) fn legacy_facade_name(server_name: &str) -> String {
    let sanitized: String = server_name
        .chars()
        .flat_map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_lowercase().collect::<Vec<_>>()
            } else {
                vec!['_']
            }
        })
        .take(60)
        .collect();
    format!("gateway_{sanitized}")
}

pub(crate) fn map_gateway_error(tool: &str, err: MaenifoldError) -> rmcp::ErrorData {
    match err {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => rmcp::ErrorData::new(
            rmcp::model::ErrorCode(i32::try_from(code).unwrap_or(JSONRPC_INTERNAL_ERROR as i32)),
            message,
            Some(data),
        ),
        MaenifoldError::GatewayInvalidParams(message) => rmcp::ErrorData::new(
            rmcp::model::ErrorCode(JSONRPC_INVALID_PARAMS as i32),
            message,
            Some(Value::Null),
        ),
        _ => rmcp::ErrorData::internal_error(format_tool_error(tool, &err), None),
    }
}

pub(crate) fn log_gateway_error(server: Option<&str>, tool: &str, err: &MaenifoldError) {
    let server = server.unwrap_or("");
    match err {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } if *code == JSONRPC_INTERNAL_ERROR || *code == GATEWAY_TIMEOUT_ERROR => {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) {
                error!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed");
            } else {
                error!(server, tool, err = %message, code, "gateway tool call failed");
            }
        }
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) {
                warn!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed");
            } else {
                warn!(server, tool, err = %message, code, "gateway tool call failed");
            }
        }
        _ => warn!(server, tool, err = %err, "gateway tool call failed"),
    }
}
