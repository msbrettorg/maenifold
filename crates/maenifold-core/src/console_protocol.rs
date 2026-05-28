use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[cfg(target_os = "macos")]
use std::borrow::Cow;
#[cfg(target_os = "macos")]
use wry::http::header::{CONTENT_TYPE, HeaderValue};
#[cfg(target_os = "macos")]
use wry::http::{Response, StatusCode};
#[cfg(target_os = "macos")]
use wry::WebViewBuilder;

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
        window.ipc.postMessage(JSON.stringify({ type: "invoke", requestId, payload: { tool, args } }));
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
pub fn register_console_protocol<'a>(builder: WebViewBuilder<'a>, console_root: PathBuf) -> WebViewBuilder<'a> {
    builder.with_custom_protocol("console".into(), move |_id, request| {
        let request_path = request.uri().path().trim_start_matches('/');
        match resolve_console_asset(&console_root, if request_path.is_empty() { "index.html" } else { request_path }) {
            Ok((bytes, mime_type)) => build_response(StatusCode::OK, mime_type, Cow::Owned(bytes)),
            Err(ConsoleProtocolError::NotFound) => build_response(StatusCode::NOT_FOUND, "text/plain", Cow::Borrowed(b"Not Found")),
            Err(ConsoleProtocolError::PathTraversal) => build_response(StatusCode::FORBIDDEN, "text/plain", Cow::Borrowed(b"Forbidden")),
            Err(ConsoleProtocolError::Io(error)) => build_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "text/plain",
                Cow::Owned(format!("Internal Server Error: {error}").into_bytes()),
            ),
        }
    })
}

pub fn resolve_console_asset(console_root: &Path, request_path: &str) -> Result<(Vec<u8>, &'static str), ConsoleProtocolError> {
    let mut relative_path = PathBuf::new();
    for component in Path::new(request_path.trim_start_matches('/')).components() {
        match component {
            Component::Normal(segment) => relative_path.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ConsoleProtocolError::PathTraversal)
            }
        }
    }
    let asset_path = console_root.join(&relative_path);
    let canonical_console_root = fs::canonicalize(console_root).map_err(ConsoleProtocolError::Io)?;
    let canonical_asset_path = asset_path.canonicalize().map_err(|error| match error.kind() {
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
