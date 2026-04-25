use super::stdio_io::{parse_result, read_response, send_message};
use super::stdio_session::StdioSession;
use crate::error::{GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, MaenifoldError, Result};
use rmcp::model::ReadResourceResult;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::Mutex;

impl StdioSession {
    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        self.do_request(
            "resources/read",
            json!({"uri": uri}),
            format!("resource '{uri}'"),
        )
        .await
    }

    pub(super) async fn do_request<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
        what: String,
    ) -> Result<T> {
        if !self.is_usable() {
            return Err(MaenifoldError::SessionUnusable);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut io = self.io.lock().await;
        if self.is_poisoned() || !self.refresh_liveness() {
            return Err(MaenifoldError::SessionUnusable);
        }
        let request = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let response = match tokio::time::timeout(self.call_timeout, async {
            send_message(&mut io.0, &request).await?;
            let (writer, reader) = &mut *io;
            read_response(writer, reader, id, &self.server_name).await
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                let _ = send_message(&mut io.0, &json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id}})).await;
                self.poisoned.store(true, Ordering::SeqCst);
                return Err(self
                    .gateway_error(
                        GATEWAY_TIMEOUT_ERROR,
                        format!(
                            "Gateway: {} {} timed out after {}s",
                            self.server_name,
                            what,
                            self.call_timeout.as_secs()
                        ),
                    )
                    .await);
            }
        };
        match response {
            Ok(value) => parse_result(value),
            Err(error) if is_connection_error(&error) => {
                self.dead.store(true, Ordering::SeqCst);
                Err(self
                    .gateway_error(
                        JSONRPC_INTERNAL_ERROR,
                        format!(
                            "Gateway: {} connection lost: {}",
                            self.server_name,
                            connection_detail(&error)
                        ),
                    )
                    .await)
            }
            Err(error) => Err(error),
        }
    }

    async fn gateway_error(&self, code: i64, message: String) -> MaenifoldError {
        MaenifoldError::JsonRpc {
            code,
            message,
            data: stderr_data(&self.stderr).await,
        }
    }
}

fn is_connection_error(error: &MaenifoldError) -> bool {
    matches!(error, MaenifoldError::Message(msg) if msg.contains("connection error"))
}

fn connection_detail(error: &MaenifoldError) -> String {
    match error {
        MaenifoldError::Message(msg) => msg
            .strip_prefix("connection error: ")
            .unwrap_or(msg)
            .to_string(),
        _ => error.to_string(),
    }
}

async fn stderr_data(stderr: &Arc<Mutex<String>>) -> Value {
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
