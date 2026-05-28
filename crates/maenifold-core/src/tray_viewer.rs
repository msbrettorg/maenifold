use crate::error::{MaenifoldError, Result};
use std::sync::Arc;
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
    DispatchScript {
        target: ViewerTarget,
        script: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ViewerTarget {
    Pipeline,
    Gate,
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

    #[cfg(target_os = "macos")]
    pub(crate) fn webview(&self) -> &wry::WebView {
        &self._webview
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn window_id(&self) -> WindowId {
        self.window_id
    }
}

#[cfg(target_os = "macos")]
fn open_viewer(
    event_loop: &ActiveEventLoop,
    event_proxy: EventLoopProxy<TrayViewerEvent>,
    title: &'static str,
    initial_url: &'static str,
    close_message: &'static str,
    close_event: fn() -> TrayViewerEvent,
    target: ViewerTarget,
    viewer_label: &'static str,
) -> Result<PipelineViewer> {
    use winit::dpi::LogicalSize;
    use winit::window::Window;
    use wry::WebViewBuilder;

    let window = event_loop
        .create_window(
            Window::default_attributes()
                .with_title(title)
                .with_inner_size(LogicalSize::new(920.0, 640.0))
                .with_min_inner_size(LogicalSize::new(520.0, 360.0))
                .with_decorations(true)
                .with_title_hidden(true)
                .with_visible(true),
        )
        .map_err(|err| MaenifoldError::Message(format!("failed to create viewer window: {err}")))?;

    let console_root = crate::config::Config::current().assets_path.join("console");
    let proxy = event_proxy.clone();
    let runtime_context =
        crate::mcp::resources::tokio_handle().zip(crate::mcp::resources::gateway_state());
    if runtime_context.is_none() {
        tracing::warn!("{viewer_label} IPC unavailable: gateway runtime not initialized");
    }

    let webview =
        crate::console_protocol::register_console_protocol(WebViewBuilder::new(), console_root)
            .with_initialization_script(crate::console_protocol::WINDOW_MAENIFOLD_INIT_SCRIPT)
            .with_url(initial_url)
            .with_ipc_handler(move |request| {
                if request.body() == close_message {
                    let _ = proxy.send_event(close_event());
                } else {
                    match runtime_context.as_ref() {
                        Some((tokio_handle, gateway)) => {
                            let proxy = proxy.clone();
                            let gateway = Arc::clone(gateway);
                            if let Err(err) = crate::console_protocol::handle_console_ipc(
                                request.body(),
                                tokio_handle,
                                gateway,
                                move |script| {
                                    if let Err(err) =
                                        proxy.send_event(TrayViewerEvent::DispatchScript {
                                            target,
                                            script,
                                        })
                                    {
                                        tracing::warn!(
                                            error = %err,
                                            "failed to dispatch script to {viewer_label}"
                                        );
                                    }
                                },
                            ) {
                                tracing::warn!("console ipc error: {err}");
                            }
                        }
                        None => tracing::warn!(
                            message = request.body(),
                            "dropping {viewer_label} IPC message before gateway runtime init"
                        ),
                    }
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

#[cfg(target_os = "macos")]
pub(crate) fn open_pipeline_viewer(
    event_loop: &ActiveEventLoop,
    event_proxy: EventLoopProxy<TrayViewerEvent>,
) -> Result<PipelineViewer> {
    open_viewer(
        event_loop,
        event_proxy,
        PIPELINE_VIEWER_TITLE,
        "console://maenifold/index.html?surface=pipeline",
        PIPELINE_VIEWER_CLOSE_MESSAGE,
        || TrayViewerEvent::ClosePipelineViewer,
        ViewerTarget::Pipeline,
        "pipeline viewer",
    )
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
    open_viewer(
        event_loop,
        event_proxy,
        GATE_VIEWER_TITLE,
        "console://maenifold/index.html?surface=gate",
        GATE_VIEWER_CLOSE_MESSAGE,
        || TrayViewerEvent::CloseGateViewer,
        ViewerTarget::Gate,
        "gate viewer",
    )
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
