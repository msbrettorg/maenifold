use crate::error::{MaenifoldError, Result};
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

    let console_root = crate::config::Config::current().assets_path.join("console");
    let proxy = event_proxy.clone();
    let webview =
        crate::console_protocol::register_console_protocol(WebViewBuilder::new(), console_root)
            .with_initialization_script(crate::console_protocol::WINDOW_MAENIFOLD_INIT_SCRIPT)
            .with_url("console://maenifold/index.html?surface=pipeline")
            .with_ipc_handler(move |request| {
                if request.body() == PIPELINE_VIEWER_CLOSE_MESSAGE {
                    let _ = proxy.send_event(TrayViewerEvent::ClosePipelineViewer);
                } else {
                    tracing::debug!(
                        message = request.body(),
                        "dropping unhandled Pipeline viewer IPC message"
                    );
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

    let console_root = crate::config::Config::current().assets_path.join("console");
    let proxy = event_proxy.clone();
    let webview =
        crate::console_protocol::register_console_protocol(WebViewBuilder::new(), console_root)
            .with_initialization_script(crate::console_protocol::WINDOW_MAENIFOLD_INIT_SCRIPT)
            .with_url("console://maenifold/index.html?surface=gate")
            .with_ipc_handler(move |request| {
                if request.body() == GATE_VIEWER_CLOSE_MESSAGE {
                    let _ = proxy.send_event(TrayViewerEvent::CloseGateViewer);
                } else {
                    tracing::debug!(
                        message = request.body(),
                        "dropping unhandled Gate viewer IPC message"
                    );
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

#[cfg(test)]
#[path = "tray_viewer_tests.rs"]
mod tests;
