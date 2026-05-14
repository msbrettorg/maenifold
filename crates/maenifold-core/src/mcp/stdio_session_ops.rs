use crate::error::{GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, MaenifoldError, Result};
use rmcp::{
    model::{
        CallToolRequest, CallToolRequestParams, CallToolResult, ClientRequest, ReadResourceRequest,
        GetPromptRequest, GetPromptRequestParams, GetPromptResult, ReadResourceRequestParams,
        ReadResourceResult, ServerResult,
    },
    service::{PeerRequestOptions, ServiceError},
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::Mutex;

use super::stdio_session::StdioSession;

impl StdioSession {
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult> {
        let arguments = match arguments {
            Value::Object(arguments) => arguments,
            other => {
                return Err(MaenifoldError::Message(format!(
                    "MCP tool arguments for '{name}' must be an object, got {other}"
                )));
            }
        };
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new(name.to_string()).with_arguments(arguments),
        ));
        match self.send_request(request, format!("tool '{name}'")).await? {
            ServerResult::CallToolResult(result) => Ok(result),
            _ => Err(MaenifoldError::Message(
                "unexpected MCP tools/call response".to_string(),
            )),
        }
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        let request = ClientRequest::ReadResourceRequest(ReadResourceRequest::new(
            ReadResourceRequestParams::new(uri.to_string()),
        ));
        match self
            .send_request(request, format!("resource '{uri}'"))
            .await?
        {
            ServerResult::ReadResourceResult(result) => Ok(result),
            _ => Err(MaenifoldError::Message(
                "unexpected MCP resources/read response".to_string(),
            )),
        }
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, Value>>,
    ) -> Result<GetPromptResult> {
        let mut params = GetPromptRequestParams::new(name.to_string());
        if let Some(arguments) = arguments {
            params = params.with_arguments(arguments);
        }
        let request = ClientRequest::GetPromptRequest(GetPromptRequest::new(params));
        match self.send_request(request, format!("prompt '{name}'")).await? {
            ServerResult::GetPromptResult(result) => Ok(result),
            _ => Err(MaenifoldError::Message(
                "unexpected MCP prompts/get response".to_string(),
            )),
        }
    }

    async fn send_request(&self, request: ClientRequest, what: String) -> Result<ServerResult> {
        if !self.is_usable() {
            return Err(MaenifoldError::SessionUnusable);
        }
        let mut options = PeerRequestOptions::no_options();
        options.timeout = Some(self.call_timeout);
        let handle = match self.peer.send_cancellable_request(request, options).await {
            Ok(handle) => handle,
            Err(error) => return Err(self.map_request_error(error, &what).await),
        };
        let response = match handle.await_response().await {
            Ok(response) => response,
            Err(error) => return Err(self.map_request_error(error, &what).await),
        };
        Ok(response)
    }

    async fn map_request_error(&self, error: ServiceError, what: &str) -> MaenifoldError {
        match error {
            ServiceError::Timeout { .. } => {
                self.poisoned.store(true, Ordering::SeqCst);
                self.terminate_process_group().await;
                MaenifoldError::JsonRpc {
                    code: GATEWAY_TIMEOUT_ERROR,
                    message: format!(
                        "Gateway: {} {} timed out after {}s",
                        self.server_name,
                        what,
                        self.call_timeout.as_secs()
                    ),
                    data: stderr_tail_value(&self.stderr).await,
                }
            }
            ServiceError::TransportClosed | ServiceError::TransportSend(_) => {
                self.dead.store(true, Ordering::SeqCst);
                MaenifoldError::JsonRpc {
                    code: JSONRPC_INTERNAL_ERROR,
                    message: format!("Gateway: {} connection lost: {}", self.server_name, error),
                    data: stderr_tail_value(&self.stderr).await,
                }
            }
            ServiceError::McpError(error) => MaenifoldError::JsonRpc {
                code: i64::from(error.code.0),
                message: error.message.to_string(),
                data: error.data.unwrap_or(Value::Null),
            },
            other => MaenifoldError::Message(other.to_string()),
        }
    }
}

async fn stderr_tail_value(stderr: &Arc<Mutex<String>>) -> Value {
    let stderr = stderr.lock().await;
    let tail = tail_chars(stderr.trim(), 500);
    if tail.is_empty() {
        Value::Null
    } else {
        json!({ "stderrTail": tail })
    }
}

fn tail_chars(value: &str, max_chars: usize) -> String {
    let len = value.chars().count();
    if len <= max_chars {
        value.to_string()
    } else {
        value.chars().skip(len - max_chars).collect()
    }
}
