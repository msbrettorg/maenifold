use crate::error::{MaenifoldError, Result};
use crate::mcp::resources::pipeline::{PIPELINE_SESSION_PREFIX, PIPELINE_SESSIONS_URI};
use crate::mcp::tool_utils::{MCP_APP_HTML_MIME_TYPE, PIPELINE_UI_RESOURCE_URI};
use rmcp::model::ResourceContents;
use serde_json::{Value, json};
use tray_icon::menu::MenuId;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

pub(crate) const PIPELINE_VIEWER_MENU_TEXT: &str = "Pipeline Viewer";
const PIPELINE_VIEWER_CLOSE_MESSAGE: &str = "close-pipeline-viewer";
const PIPELINE_VIEWER_TITLE: &str = "Pipeline";
const PIPELINE_DATA_MARKER: &str =
    r#"<script type="application/json" id="pipeline-data"></script>"#;
const HEAD_STYLE_END_MARKER: &str = "</style>";
const BODY_START_MARKER: &str = "<body>";

pub(crate) enum TrayViewerEvent {
    ClosePipelineViewer,
}

pub(crate) fn is_pipeline_viewer_event(
    event_id: &MenuId,
    pipeline_viewer_id: Option<&MenuId>,
) -> bool {
    pipeline_viewer_id.is_some_and(|id| event_id == id)
}

pub(crate) fn pipeline_viewer_resource_uri() -> &'static str {
    PIPELINE_UI_RESOURCE_URI
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
                .with_decorations(false)
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

pub(crate) fn pipeline_viewer_html_for_payload(payload: &Value) -> Result<String> {
    let html = pipeline_app_resource_html()?;
    let html = inject_pipeline_viewer_host_controls(&html)?;
    embed_pipeline_payload(&html, payload)
}

pub(crate) fn pipeline_viewer_current_payload() -> Result<Value> {
    let sessions_payload = read_json_resource_payload(PIPELINE_SESSIONS_URI)?;
    let Some(session_id) = select_pipeline_session_id(&sessions_payload) else {
        return Ok(empty_pipeline_view_payload());
    };
    read_json_resource_payload(&format!("{PIPELINE_SESSION_PREFIX}{session_id}"))
}

fn pipeline_app_resource_html() -> Result<String> {
    let result = crate::mcp::resources::handle_read_resource(pipeline_viewer_resource_uri())?;
    for content in result.contents {
        if let ResourceContents::TextResourceContents {
            text, mime_type, ..
        } = content
        {
            if mime_type.as_deref() == Some(MCP_APP_HTML_MIME_TYPE) {
                return Ok(text);
            }
            return Err(MaenifoldError::Message(format!(
                "Pipeline app resource returned unexpected MIME type: {:?}",
                mime_type
            )));
        }
    }
    Err(MaenifoldError::Message(
        "Pipeline app resource did not return HTML text content".to_string(),
    ))
}

fn embed_pipeline_payload(html: &str, payload: &Value) -> Result<String> {
    let payload = escape_json_for_script(&serde_json::to_string(payload)?);
    let script =
        format!(r#"<script type="application/json" id="pipeline-data">{payload}</script>"#);
    let embedded = html.replacen(PIPELINE_DATA_MARKER, &script, 1);
    if embedded == html {
        return Err(MaenifoldError::Message(
            "Pipeline app HTML is missing the pipeline-data script marker".to_string(),
        ));
    }
    Ok(embedded)
}

fn inject_pipeline_viewer_host_controls(html: &str) -> Result<String> {
    let style = format!(
        r#"
    .viewer-close {{
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
    }}
    .viewer-close:hover {{ background: var(--line); }}
    header {{ padding-right: 64px; }}
"#
    );
    let html = html.replacen(
        HEAD_STYLE_END_MARKER,
        &format!("{style}{HEAD_STYLE_END_MARKER}"),
        1,
    );
    if !html.contains(&format!("{style}{HEAD_STYLE_END_MARKER}")) {
        return Err(MaenifoldError::Message(
            "Pipeline app HTML is missing the style close marker".to_string(),
        ));
    }

    let control = format!(
        r#"{BODY_START_MARKER}
  <button class="viewer-close" type="button" aria-label="Close Pipeline viewer">x</button>
  <script>
    (() => {{
      const close = () => {{
        if (window.ipc && typeof window.ipc.postMessage === "function") {{
          window.ipc.postMessage("{PIPELINE_VIEWER_CLOSE_MESSAGE}");
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
        return Err(MaenifoldError::Message(
            "Pipeline app HTML is missing the body close marker".to_string(),
        ));
    }
    Ok(html)
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
mod tests {
    use super::{
        PIPELINE_VIEWER_MENU_TEXT, is_pipeline_viewer_event, pipeline_viewer_html_for_payload,
        pipeline_viewer_resource_uri, select_pipeline_session_id,
    };
    use crate::mcp::tool_utils::PIPELINE_UI_RESOURCE_URI;
    use serde_json::json;
    use tray_icon::menu::MenuId;

    #[test]
    fn pipeline_viewer_menu_event_matches_only_configured_id() {
        let viewer_id = MenuId::new("pipeline-viewer");
        assert!(is_pipeline_viewer_event(&viewer_id, Some(&viewer_id)));
        assert!(!is_pipeline_viewer_event(
            &MenuId::new("help"),
            Some(&viewer_id)
        ));
        assert!(!is_pipeline_viewer_event(&viewer_id, None));
    }

    #[test]
    fn pipeline_viewer_targets_pipeline_mcp_app_resource() {
        assert_eq!(PIPELINE_VIEWER_MENU_TEXT, "Pipeline Viewer");
        assert_eq!(pipeline_viewer_resource_uri(), PIPELINE_UI_RESOURCE_URI);
    }

    #[test]
    fn pipeline_viewer_html_embeds_view_payload_from_mcp_resource() {
        let html = pipeline_viewer_html_for_payload(&json!({
            "pipelineId": "visual-uat",
            "status": "active",
            "ready": ["next"],
            "active": ["now"],
            "completed": ["done"],
            "nodes": [
                { "id": "now", "type": "tool", "status": "active" },
                { "id": "next", "type": "workflow", "status": "ready" }
            ],
            "edges": [{ "from": "now", "to": "next", "type": "success" }]
        }))
        .expect("Pipeline viewer HTML should embed sample payload");

        assert!(html.contains("window.renderPipeline"));
        assert!(html.contains("visual-uat"));
        assert!(html.contains(r#"aria-label="Close Pipeline viewer""#));
        assert!(html.contains("window.ipc.postMessage"));
        assert!(html.contains("close-pipeline-viewer"));
        assert!(html.contains(r#"<script type="application/json" id="pipeline-data">{"#));
    }

    #[test]
    fn pipeline_viewer_selects_active_pipeline_before_completed() {
        let payload = json!({
            "sessions": [
                { "pipelineId": "pipeline-999", "status": "completed" },
                { "pipelineId": "pipeline-100", "status": "active" },
                { "pipelineId": "pipeline-101", "status": "pending" }
            ]
        });

        assert_eq!(
            select_pipeline_session_id(&payload).as_deref(),
            Some("pipeline-101")
        );
    }

    #[test]
    fn pipeline_viewer_falls_back_to_pipeline_list_shape() {
        let payload = json!({
            "pipelines": [
                { "id": "pipeline-100", "status": "completed" },
                { "sessionId": "pipeline-101", "status": "completed" }
            ]
        });

        assert_eq!(
            select_pipeline_session_id(&payload).as_deref(),
            Some("pipeline-101")
        );
    }
}
