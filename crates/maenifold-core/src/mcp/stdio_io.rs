use crate::error::{MaenifoldError, Result};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::io;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::Child;

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(25);

pub(super) async fn send_message<W: AsyncWrite + Unpin>(
    stdin: &mut W,
    message: &Value,
) -> Result<()> {
    let mut body = serde_json::to_vec(message)?;
    body.push(b'\n');
    stdin.write_all(&body).await.map_err(connection_error)?;
    stdin.flush().await.map_err(connection_error)?;
    Ok(())
}

pub(super) async fn read_response<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
    writer: &mut W,
    reader: &mut BufReader<R>,
    id: u64,
    server_name: &str,
) -> Result<Value> {
    loop {
        let mut line = Vec::new();
        loop {
            let chunk = reader.fill_buf().await.map_err(connection_error)?;
            if chunk.is_empty() {
                return Err(MaenifoldError::Message(
                    "connection error: unexpected EOF while reading MCP response".to_string(),
                ));
            }
            let take = chunk
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(chunk.len(), |idx| idx + 1);
            if line.len() + take > MAX_FRAME_BYTES {
                return Err(MaenifoldError::Message(format!(
                    "connection error: MCP response exceeded max frame size of {MAX_FRAME_BYTES} bytes"
                )));
            }
            line.extend_from_slice(&chunk[..take]);
            reader.consume(take);
            if line.last() == Some(&b'\n') {
                break;
            }
        }
        let message: Value = serde_json::from_slice(&line)?;
        if message.get("method").is_some() && message.get("id").is_none() {
            continue;
        }
        if message.get("method").is_some() && message.get("id").is_some() {
            let request_id = message.get("id").cloned().unwrap_or(Value::Null);
            let method = message
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            tracing::warn!(
                server_name,
                server_method = method,
                "upstream server sent unsupported request, responding with method-not-found"
            );
            let error_response = json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: client does not implement '{method}'")
                }
            });
            let _ = send_message(writer, &error_response).await;
            continue;
        }
        if message.get("id").and_then(Value::as_u64) != Some(id) {
            continue;
        }
        if let Some(error) = message.get("error").filter(|error| !error.is_null()) {
            let code = error.get("code").and_then(Value::as_i64).ok_or_else(|| {
                MaenifoldError::Message(format!("JSON-RPC error missing integer code: {error}"))
            })?;
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown JSON-RPC error");
            let data = error.get("data").cloned().unwrap_or(Value::Null);
            return Err(MaenifoldError::JsonRpc {
                code,
                message: message.to_string(),
                data,
            });
        }
        return Ok(message["result"].clone());
    }
}

pub(super) fn parse_result<T: DeserializeOwned>(value: Value) -> Result<T> {
    serde_json::from_value(value).map_err(Into::into)
}

pub(super) fn connection_error(error: io::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("connection error: {error}"))
}

pub(super) async fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {}
            Err(_) => return false,
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
