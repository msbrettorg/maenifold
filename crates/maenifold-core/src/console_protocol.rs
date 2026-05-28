use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[cfg(target_os = "macos")]
use std::borrow::Cow;
#[cfg(target_os = "macos")]
use wry::WebViewBuilder;
#[cfg(target_os = "macos")]
use wry::http::header::{CONTENT_TYPE, HeaderValue};
#[cfg(target_os = "macos")]
use wry::http::{Response, StatusCode};

pub const WINDOW_MAENIFOLD_INIT_SCRIPT: &str = r#"
(() => {
  const pending = new Map();
  const subscriptions = new Map();
  const nextRequestId = () => globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  window.__maenifoldDispatch = (json) => {
    const message = typeof json === "string" ? JSON.parse(json) : json;
    if (message?.requestId && pending.has(message.requestId)) {
      const { resolve, reject } = pending.get(message.requestId);
      pending.delete(message.requestId);
      if (message.error) reject(message.error); else resolve(message.result);
      return;
    }
    if (message?.channel && subscriptions.has(message.channel)) {
      subscriptions.get(message.channel).forEach((callback) => callback(message.payload));
    }
  };
  window.maenifold = {
    invoke(tool, args) {
      return new Promise((resolve, reject) => {
        const requestId = nextRequestId();
        pending.set(requestId, { resolve, reject });
        window.ipc.postMessage(JSON.stringify({ type: "invoke", requestId, tool, args }));
      });
    },
    subscribe(channel, cb) {
      const callbacks = subscriptions.get(channel) ?? new Set();
      callbacks.add(cb);
      subscriptions.set(channel, callbacks);
      window.ipc.postMessage(JSON.stringify({ type: "subscribe", requestId: nextRequestId(), payload: { channel } }));
      return () => {
        const current = subscriptions.get(channel);
        if (!current) return;
        current.delete(cb);
        if (current.size === 0) subscriptions.delete(channel);
        window.ipc.postMessage(JSON.stringify({ type: "unsubscribe", requestId: nextRequestId(), payload: { channel } }));
      };
    }
  };
})();
"#;

#[derive(Debug)]
pub enum ConsoleIpcError {
    ParseError(serde_json::Error),
    UnknownType(String),
    UnknownTool(String),
    GatewayError(String),
}

impl fmt::Display for ConsoleIpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(error) => write!(f, "failed to parse console IPC message: {error}"),
            Self::UnknownType(message_type) => {
                write!(f, "unknown console IPC message type: {message_type}")
            }
            Self::UnknownTool(tool) => write!(f, "unsupported console IPC tool: {tool}"),
            Self::GatewayError(message) => write!(f, "console IPC gateway error: {message}"),
        }
    }
}

impl std::error::Error for ConsoleIpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseError(error) => Some(error),
            Self::UnknownType(_) | Self::UnknownTool(_) | Self::GatewayError(_) => None,
        }
    }
}

/// Parsed action from a WRY IPC envelope.
#[derive(Debug, PartialEq)]
pub enum ConsoleIpcAction {
    Invoke {
        request_id: String,
        tool: String,
        args: serde_json::Value,
    },
    /// `subscribe` / `unsubscribe` — T-251.14 owns live updates;
    /// for now we acknowledge silently so the shim's auto-posted
    /// subscription messages don't error out.
    SubscriptionStub,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnvelopeDiscriminator {
    r#type: String,
    request_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvokePayload {
    tool: String,
    #[serde(default)]
    args: serde_json::Value,
}

/// Synchronous parse of a WRY ipc message body. Pure function, no I/O,
/// safe to call from any thread or test context.
pub fn parse_console_ipc(message: &str) -> Result<ConsoleIpcAction, ConsoleIpcError> {
    let raw: serde_json::Value =
        serde_json::from_str(message).map_err(ConsoleIpcError::ParseError)?;
    let envelope: EnvelopeDiscriminator =
        serde_json::from_value(raw.clone()).map_err(ConsoleIpcError::ParseError)?;

    match envelope.r#type.as_str() {
        "invoke" => {
            let payload: InvokePayload =
                serde_json::from_value(raw).map_err(ConsoleIpcError::ParseError)?;
            Ok(ConsoleIpcAction::Invoke {
                request_id: envelope.request_id,
                tool: payload.tool,
                args: payload.args,
            })
        }
        "subscribe" | "unsubscribe" => Ok(ConsoleIpcAction::SubscriptionStub),
        other => Err(ConsoleIpcError::UnknownType(other.to_string())),
    }
}

/// Forward an Invoke action to the gateway and produce the JS dispatch
/// script that should be injected back into the WebView.
pub(crate) async fn dispatch_console_invoke(
    request_id: String,
    tool: String,
    args: serde_json::Value,
    gateway: Arc<crate::mcp::gateway_runtime::GatewayState>,
) -> String {
    if tool != "GatewayExecute" {
        let payload = serde_json::json!({
            "requestId": request_id,
            "error": { "message": ConsoleIpcError::UnknownTool(tool).to_string() },
        });
        return format!("window.__maenifoldDispatch({payload});");
    }

    let payload = match gateway.handle_gateway_execute(args).await {
        Ok(result) => {
            let is_error = result.is_error.unwrap_or(false);
            match tool_result_value(result) {
                Ok(value) if is_error => serde_json::json!({
                    "requestId": request_id,
                    "error": value,
                }),
                Ok(value) => serde_json::json!({
                    "requestId": request_id,
                    "result": value,
                }),
                Err(error) => serde_json::json!({
                    "requestId": request_id,
                    "error": { "message": error.to_string() },
                }),
            }
        }
        Err(error) => serde_json::json!({
            "requestId": request_id,
            "error": {
                "message": error.message.to_string(),
                "code": error.code.0,
            },
        }),
    };

    format!("window.__maenifoldDispatch({payload});")
}

/// Synchronously parse the incoming WRY ipc message body, dispatch the
/// requested gateway call via `tokio_handle.spawn(...)`, and post the
/// result back to the WebView's JS context via `evaluate_script` →
/// `window.__maenifoldDispatch({requestId, result|error})`.
pub(crate) fn handle_console_ipc<F>(
    message: &str,
    tokio_handle: &tokio::runtime::Handle,
    gateway: Arc<crate::mcp::gateway_runtime::GatewayState>,
    dispatch_script: F,
) -> Result<(), ConsoleIpcError>
where
    F: Fn(String) + Send + Sync + 'static,
{
    let action = parse_console_ipc(message)?;
    match action {
        ConsoleIpcAction::Invoke {
            request_id,
            tool,
            args,
        } => {
            let dispatch_script = Arc::new(dispatch_script);
            tokio_handle.spawn(async move {
                let script = dispatch_console_invoke(request_id, tool, args, gateway).await;
                dispatch_script(script);
            });
            Ok(())
        }
        ConsoleIpcAction::SubscriptionStub => Ok(()),
    }
}

#[derive(Debug)]
pub enum ConsoleProtocolError {
    NotFound,
    PathTraversal,
    Io(std::io::Error),
}

impl fmt::Display for ConsoleProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "console asset not found"),
            Self::PathTraversal => write!(f, "console asset path traversal rejected"),
            Self::Io(error) => write!(f, "console asset I/O error: {error}"),
        }
    }
}

impl std::error::Error for ConsoleProtocolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::NotFound | Self::PathTraversal => None,
        }
    }
}

#[cfg(target_os = "macos")]
pub fn register_console_protocol<'a>(
    builder: WebViewBuilder<'a>,
    console_root: PathBuf,
) -> WebViewBuilder<'a> {
    builder.with_custom_protocol("console".into(), move |_id, request| {
        let request_path = request.uri().path().trim_start_matches('/');
        match resolve_console_asset(
            &console_root,
            if request_path.is_empty() {
                "index.html"
            } else {
                request_path
            },
        ) {
            Ok((bytes, mime_type)) => build_response(StatusCode::OK, mime_type, Cow::Owned(bytes)),
            Err(ConsoleProtocolError::NotFound) => build_response(
                StatusCode::NOT_FOUND,
                "text/plain",
                Cow::Borrowed(b"Not Found"),
            ),
            Err(ConsoleProtocolError::PathTraversal) => build_response(
                StatusCode::FORBIDDEN,
                "text/plain",
                Cow::Borrowed(b"Forbidden"),
            ),
            Err(ConsoleProtocolError::Io(error)) => build_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "text/plain",
                Cow::Owned(format!("Internal Server Error: {error}").into_bytes()),
            ),
        }
    })
}

pub fn resolve_console_asset(
    console_root: &Path,
    request_path: &str,
) -> Result<(Vec<u8>, &'static str), ConsoleProtocolError> {
    let mut relative_path = PathBuf::new();
    for component in Path::new(request_path.trim_start_matches('/')).components() {
        match component {
            Component::Normal(segment) => relative_path.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ConsoleProtocolError::PathTraversal);
            }
        }
    }
    let asset_path = console_root.join(&relative_path);
    let canonical_console_root =
        fs::canonicalize(console_root).map_err(ConsoleProtocolError::Io)?;
    let canonical_asset_path = asset_path
        .canonicalize()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => ConsoleProtocolError::NotFound,
            _ => ConsoleProtocolError::Io(error),
        })?;
    if !canonical_asset_path.starts_with(&canonical_console_root) {
        return Err(ConsoleProtocolError::PathTraversal);
    }
    let bytes = fs::read(&canonical_asset_path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => ConsoleProtocolError::NotFound,
        _ => ConsoleProtocolError::Io(error),
    })?;
    Ok((bytes, mime_type_for(&canonical_asset_path)))
}

fn mime_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html",
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("woff2") => "font/woff2",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

fn tool_result_value(result: rmcp::model::CallToolResult) -> serde_json::Result<serde_json::Value> {
    let rmcp::model::CallToolResult {
        content,
        structured_content,
        ..
    } = result;
    if let Some(value) = structured_content {
        return Ok(value);
    }

    let rendered = if content.len() == 1 {
        content
            .first()
            .and_then(|content| content.raw.as_text())
            .map(|text| text.text.clone())
            .unwrap_or_else(|| serde_json::to_string(&content).unwrap_or_else(|_| "[]".to_string()))
    } else {
        serde_json::to_string(&content)?
    };

    Ok(serde_json::from_str(&rendered).unwrap_or(serde_json::Value::String(rendered)))
}

#[cfg(target_os = "macos")]
fn build_response(
    status: StatusCode,
    mime_type: &'static str,
    body: Cow<'static, [u8]>,
) -> Response<Cow<'static, [u8]>> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(mime_type));
    response
}

#[cfg(test)]
#[path = "console_protocol_tests.rs"]
mod tests;
