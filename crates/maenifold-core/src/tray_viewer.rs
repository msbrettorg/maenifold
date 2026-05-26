use crate::error::{MaenifoldError, Result};
use crate::mcp::resources::pipeline::{PIPELINE_SESSIONS_URI, PIPELINE_SESSION_PREFIX};
use crate::mcp::tool_utils::{
    GATE_UI_RESOURCE_URI, MCP_APP_HTML_MIME_TYPE, PIPELINE_UI_RESOURCE_URI,
};
use rmcp::model::ResourceContents;
use serde_json::{json, Value};
use tray_icon::menu::MenuId;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::WindowId;

pub(crate) const PIPELINE_VIEWER_MENU_TEXT: &str = "Pipeline Viewer";
pub(crate) const GATE_VIEWER_MENU_TEXT: &str = "Gate Board";
const PIPELINE_VIEWER_CLOSE_MESSAGE: &str = "close-pipeline-viewer";
const GATE_VIEWER_CLOSE_MESSAGE: &str = "close-gate-viewer";
const PIPELINE_VIEWER_TITLE: &str = "Pipeline";
const GATE_VIEWER_TITLE: &str = "Gate Board";
const MAENIFOLD_GATE_ROOT_ENV: &str = "MAENIFOLD_GATE_ROOT";
const PIPELINE_DATA_MARKER: &str =
    r#"<script type="application/json" id="pipeline-data"></script>"#;
const GATE_DATA_MARKER: &str = r#"<script type="application/json" id="gate-data"></script>"#;
const HEAD_STYLE_END_MARKER: &str = "</style>";
const BODY_START_MARKER: &str = "<body>";

pub(crate) enum TrayViewerEvent {
    ClosePipelineViewer,
    CloseGateViewer,
}

pub(crate) fn is_pipeline_viewer_event(
    event_id: &MenuId,
    pipeline_viewer_id: Option<&MenuId>,
) -> bool {
    pipeline_viewer_id.is_some_and(|id| event_id == id)
}

pub(crate) fn is_gate_viewer_event(event_id: &MenuId, gate_viewer_id: Option<&MenuId>) -> bool {
    gate_viewer_id.is_some_and(|id| event_id == id)
}

pub(crate) fn pipeline_viewer_resource_uri() -> &'static str {
    PIPELINE_UI_RESOURCE_URI
}

pub(crate) fn gate_viewer_resource_uri() -> &'static str {
    GATE_UI_RESOURCE_URI
}

pub(crate) struct PipelineViewer {
    #[cfg(target_os = "macos")]
    window: winit::window::Window,
    #[cfg(not(target_os = "macos"))]
    window_id: WindowId,
    #[cfg(target_os = "macos")]
    _webview: wry::WebView,
}

impl PipelineViewer {
    #[cfg(target_os = "macos")]
    pub(crate) fn window_id(&self) -> WindowId {
        self.window.id()
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn window_id(&self) -> WindowId {
        self.window_id
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn open_pipeline_viewer(
    event_loop: &ActiveEventLoop,
    event_proxy: EventLoopProxy<TrayViewerEvent>,
) -> Result<PipelineViewer> {
    use winit::dpi::LogicalSize;
    use winit::window::Window;
    use wry::WebViewBuilder;

    let window = event_loop
        .create_window(
            Window::default_attributes()
                .with_title(PIPELINE_VIEWER_TITLE)
                .with_inner_size(LogicalSize::new(920.0, 640.0))
                .with_min_inner_size(LogicalSize::new(520.0, 360.0))
                .with_decorations(true)
                .with_title_hidden(true)
                .with_visible(true),
        )
        .map_err(|err| MaenifoldError::Message(format!("failed to create viewer window: {err}")))?;

    let html = pipeline_viewer_html_for_payload(&pipeline_viewer_current_payload()?)?;
    let proxy = event_proxy.clone();
    let webview = WebViewBuilder::new()
        .with_html(html)
        .with_ipc_handler(move |request| {
            if request.body() == PIPELINE_VIEWER_CLOSE_MESSAGE {
                let _ = proxy.send_event(TrayViewerEvent::ClosePipelineViewer);
            }
        })
        .build(&window)
        .map_err(|err| {
            MaenifoldError::Message(format!("failed to create viewer webview: {err}"))
        })?;

    Ok(PipelineViewer {
        window,
        _webview: webview,
    })
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn open_pipeline_viewer(
    _event_loop: &ActiveEventLoop,
    _event_proxy: EventLoopProxy<TrayViewerEvent>,
) -> Result<PipelineViewer> {
    Err(MaenifoldError::Message(
        "Pipeline viewer is currently implemented for macOS tray builds".to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub(crate) fn open_gate_viewer(
    event_loop: &ActiveEventLoop,
    event_proxy: EventLoopProxy<TrayViewerEvent>,
) -> Result<PipelineViewer> {
    use winit::dpi::LogicalSize;
    use winit::window::Window;
    use wry::WebViewBuilder;

    let window = event_loop
        .create_window(
            Window::default_attributes()
                .with_title(GATE_VIEWER_TITLE)
                .with_inner_size(LogicalSize::new(920.0, 640.0))
                .with_min_inner_size(LogicalSize::new(520.0, 360.0))
                .with_decorations(true)
                .with_title_hidden(true)
                .with_visible(true),
        )
        .map_err(|err| MaenifoldError::Message(format!("failed to create viewer window: {err}")))?;

    let html = gate_viewer_html_for_payload(&gate_viewer_current_payload()?)?;
    let proxy = event_proxy.clone();
    let webview = WebViewBuilder::new()
        .with_html(html)
        .with_ipc_handler(move |request| {
            if request.body() == GATE_VIEWER_CLOSE_MESSAGE {
                let _ = proxy.send_event(TrayViewerEvent::CloseGateViewer);
            }
        })
        .build(&window)
        .map_err(|err| {
            MaenifoldError::Message(format!("failed to create viewer webview: {err}"))
        })?;

    Ok(PipelineViewer {
        window,
        _webview: webview,
    })
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn open_gate_viewer(
    _event_loop: &ActiveEventLoop,
    _event_proxy: EventLoopProxy<TrayViewerEvent>,
) -> Result<PipelineViewer> {
    Err(MaenifoldError::Message(
        "Gate viewer is currently implemented for macOS tray builds".to_string(),
    ))
}

pub(crate) fn pipeline_viewer_html_for_payload(payload: &Value) -> Result<String> {
    let html = viewer_app_resource_html(pipeline_viewer_resource_uri(), "Pipeline")?;
    let html = inject_pipeline_viewer_host_controls(&html)?;
    embed_pipeline_payload(&html, payload)
}

pub(crate) fn gate_viewer_html_for_payload(payload: &Value) -> Result<String> {
    let html = viewer_app_resource_html(gate_viewer_resource_uri(), "Gate")?;
    let html = inject_gate_viewer_host_controls(&html)?;
    embed_gate_payload(&html, payload)
}

pub(crate) fn pipeline_viewer_current_payload() -> Result<Value> {
    let sessions_payload = read_json_resource_payload(PIPELINE_SESSIONS_URI)?;
    let Some(session_id) = select_pipeline_session_id(&sessions_payload) else {
        return Ok(empty_pipeline_view_payload());
    };
    read_json_resource_payload(&format!("{PIPELINE_SESSION_PREFIX}{session_id}"))
}

pub(crate) fn gate_viewer_current_payload() -> Result<Value> {
    fetch_gate_status_payload().or_else(|error| {
        Ok(json!({
            "tasks": [],
            "issue": format!("Gate status unavailable: {error}")
        }))
    })
}

fn viewer_app_resource_html(uri: &str, label: &str) -> Result<String> {
    let result = crate::mcp::resources::handle_read_resource(uri)?;
    for content in result.contents {
        if let ResourceContents::TextResourceContents {
            text, mime_type, ..
        } = content
        {
            if mime_type.as_deref() == Some(MCP_APP_HTML_MIME_TYPE) {
                return Ok(text);
            }
            return Err(MaenifoldError::Message(format!(
                "{label} app resource returned unexpected MIME type: {:?}",
                mime_type
            )));
        }
    }
    Err(MaenifoldError::Message(format!(
        "{label} app resource did not return HTML text content"
    )))
}

fn embed_pipeline_payload(html: &str, payload: &Value) -> Result<String> {
    embed_json_payload(
        html,
        PIPELINE_DATA_MARKER,
        "pipeline-data",
        payload,
        "Pipeline",
    )
}

fn embed_gate_payload(html: &str, payload: &Value) -> Result<String> {
    embed_json_payload(html, GATE_DATA_MARKER, "gate-data", payload, "Gate")
}

fn embed_json_payload(
    html: &str,
    marker: &str,
    element_id: &str,
    payload: &Value,
    label: &str,
) -> Result<String> {
    let payload = escape_json_for_script(&serde_json::to_string(payload)?);
    let script = format!(r#"<script type="application/json" id="{element_id}">{payload}</script>"#);
    let embedded = html.replacen(marker, &script, 1);
    if embedded == html {
        return Err(MaenifoldError::Message(format!(
            "{label} app HTML is missing the {element_id} script marker"
        )));
    }
    Ok(embedded)
}

fn inject_pipeline_viewer_host_controls(html: &str) -> Result<String> {
    inject_viewer_host_controls(
        html,
        "Pipeline",
        "Close Pipeline viewer",
        PIPELINE_VIEWER_CLOSE_MESSAGE,
    )
}

fn inject_gate_viewer_host_controls(html: &str) -> Result<String> {
    inject_viewer_host_controls(html, "Gate", "Close Gate viewer", GATE_VIEWER_CLOSE_MESSAGE)
}

fn inject_viewer_host_controls(
    html: &str,
    label: &str,
    close_label: &str,
    close_message: &str,
) -> Result<String> {
    let style = r#"
    .viewer-close {
      position: fixed;
      z-index: 20;
      top: 12px;
      right: 12px;
      width: 32px;
      height: 32px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--surface);
      color: var(--fg);
      font: 16px/1 ui-sans-serif, system-ui, sans-serif;
      cursor: pointer;
    }
    .viewer-close:hover { background: var(--line); }
    header { padding-right: 64px; }
"#;
    let html = html.replacen(
        HEAD_STYLE_END_MARKER,
        &format!("{style}{HEAD_STYLE_END_MARKER}"),
        1,
    );
    if !html.contains(&format!("{style}{HEAD_STYLE_END_MARKER}")) {
        return Err(MaenifoldError::Message(format!(
            "{label} app HTML is missing the style close marker"
        )));
    }
    let control = format!(
        r#"{BODY_START_MARKER}
  <button class="viewer-close" type="button" aria-label="{close_label}">x</button>
  <script>
    (() => {{
      const close = () => {{
        if (window.ipc && typeof window.ipc.postMessage === "function") {{
          window.ipc.postMessage("{close_message}");
        }} else {{
          window.close();
        }}
      }};
      window.addEventListener("DOMContentLoaded", () => {{
        document.querySelector(".viewer-close")?.addEventListener("click", close);
      }});
    }})();
  </script>"#
    );
    let html = html.replacen(BODY_START_MARKER, &control, 1);
    if !html.contains(r#"class="viewer-close""#) {
        return Err(MaenifoldError::Message(format!(
            "{label} app HTML is missing the body close marker"
        )));
    }
    Ok(html)
}

fn fetch_gate_status_payload() -> Result<Value> {
    let root = resolve_gate_root()?;
    let payload = serde_json::json!({
        "domain": "maenifold.gate",
        "command": "gate_Status",
        "parameters": { "root": root.to_string_lossy().to_string() },
    });
    let text = invoke_gateway_tool_sync("GatewayExecute", payload)?;
    serde_json::from_str(&text).map_err(|error| {
        MaenifoldError::Message(format!("Failed to decode gate_Status payload: {error}"))
    })
}

fn resolve_gate_root() -> Result<std::path::PathBuf> {
    if let Some(root) = std::env::var(MAENIFOLD_GATE_ROOT_ENV)
        .ok()
        .and_then(|root| (!root.trim().is_empty()).then(|| std::path::PathBuf::from(root.trim())))
    {
        return Ok(root);
    }
    std::env::current_dir().map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to resolve Gate root from current dir: {error}"
        ))
    })
}

fn invoke_gateway_tool_sync(tool_name: &str, arguments: Value) -> Result<String> {
    let config = crate::config::Config::current();
    let future = crate::mcp::invoke_gateway_cli_tool(&config, tool_name, arguments);
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            if matches!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ) {
                tokio::task::block_in_place(|| handle.block_on(future))
            } else {
                std::thread::scope(|scope| {
                    scope
                        .spawn(|| {
                            tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .map_err(|error| {
                                    MaenifoldError::Message(format!(
                                        "Failed to build runtime for gateway tool '{tool_name}': {error}"
                                    ))
                                })?
                                .block_on(future)
                        })
                        .join()
                        .map_err(|_| {
                            MaenifoldError::Message(format!(
                                "Gateway tool '{tool_name}' thread panicked"
                            ))
                        })?
                })
            }
        }
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                MaenifoldError::Message(format!(
                    "Failed to build runtime for gateway tool '{tool_name}': {error}"
                ))
            })?
            .block_on(future),
    }
}

fn read_json_resource_payload(uri: &str) -> Result<Value> {
    let result = crate::mcp::resources::handle_read_resource(uri)?;
    for content in result.contents {
        if let ResourceContents::TextResourceContents {
            text, mime_type, ..
        } = content
        {
            if mime_type.as_deref() == Some("application/json") {
                return serde_json::from_str(&text).map_err(|error| {
                    MaenifoldError::Message(format!(
                        "Failed to decode Pipeline resource {uri}: {error}"
                    ))
                });
            }
            return Err(MaenifoldError::Message(format!(
                "Pipeline resource {uri} returned unexpected MIME type: {:?}",
                mime_type
            )));
        }
    }
    Err(MaenifoldError::Message(format!(
        "Pipeline resource {uri} did not return JSON text content"
    )))
}

fn select_pipeline_session_id(payload: &Value) -> Option<String> {
    let sessions = payload
        .get("sessions")
        .or_else(|| payload.get("pipelines"))
        .and_then(Value::as_array)?;
    sessions
        .iter()
        .filter_map(pipeline_session_candidate)
        .max_by(|left, right| left.cmp_key().cmp(&right.cmp_key()))
        .map(|candidate| candidate.id.to_string())
}

struct PipelineSessionCandidate<'a> {
    id: &'a str,
    status: &'a str,
}

impl PipelineSessionCandidate<'_> {
    fn cmp_key(&self) -> (u8, &str) {
        (pipeline_status_priority(self.status), self.id)
    }
}

fn pipeline_session_candidate(value: &Value) -> Option<PipelineSessionCandidate<'_>> {
    let id = value
        .get("pipelineId")
        .or_else(|| value.get("sessionId"))
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    Some(PipelineSessionCandidate { id, status })
}

fn pipeline_status_priority(status: &str) -> u8 {
    match status {
        "running" | "active" | "paused" | "pending" | "ready" => 2,
        "failed" | "cancelled" | "completed" => 1,
        _ => 0,
    }
}

fn escape_json_for_script(json: &str) -> String {
    json.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn empty_pipeline_view_payload() -> Value {
    json!({
        "pipelineId": "Pipeline",
        "status": "empty",
        "ready": [],
        "active": [],
        "completed": [],
        "running": [],
        "nodes": [],
        "edges": []
    })
}

#[cfg(test)]
#[path = "tray_viewer_tests.rs"]
mod tests;
