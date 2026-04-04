use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tokio_util::sync::CancellationToken;
use tray_icon::menu::MenuEvent;
use tray_icon::{Icon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, EventLoop};

#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

use crate::file_watcher::GateFileWatcher;
use crate::mcp::GateMcpServer;
use crate::tray_menu::{build_tray_menu, open_ui, toggle_login_item};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateServerMode {
    Stdio,
    Http(u16),
}

impl GateServerMode {
    pub fn port(self) -> Option<u16> {
        match self {
            Self::Stdio => None,
            Self::Http(port) => Some(port),
        }
    }
}

struct TrayApp {
    mode: GateServerMode,
    root: Arc<RwLock<Option<PathBuf>>>,
    shutdown_token: CancellationToken,
    _file_watcher: Option<GateFileWatcher>,
    server_handle: Option<std::thread::JoinHandle<i32>>,
    tray_icon: Option<tray_icon::TrayIcon>,
    quit_id: Option<tray_icon::menu::MenuId>,
    open_ui_id: Option<tray_icon::menu::MenuId>,
    login_id: Option<tray_icon::menu::MenuId>,
    login_item: Option<tray_icon::menu::MenuItem>,
    exit_code: i32,
}

impl TrayApp {
    fn initialize_tray(&mut self, event_loop: &ActiveEventLoop) {
        let menu_state = build_tray_menu(self.mode);
        self.quit_id = Some(menu_state.quit_id);
        self.open_ui_id = menu_state.open_ui_id;
        self.login_id = menu_state.login_id;
        self.login_item = menu_state.login_item;

        match TrayIconBuilder::new()
            .with_menu(Box::new(menu_state.menu))
            .with_tooltip(tooltip_for_mode(self.mode))
            .with_icon(build_icon())
            .build()
        {
            Ok(tray_icon) => {
                self.tray_icon = Some(tray_icon);
            }
            Err(err) => {
                eprintln!("Warning: failed to create gate tray icon: {err}");
                self.shutdown_token.cancel();
                self.exit_code = 1;
                event_loop.exit();
                return;
            }
        }

        #[cfg(target_os = "macos")]
        {
            use objc2_core_foundation::CFRunLoop;
            let run_loop = CFRunLoop::main().unwrap();
            run_loop.wake_up();
        }
    }

    fn handle_menu_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: tray_icon::menu::MenuEvent,
    ) {
        if self.quit_id.as_ref().is_some_and(|id| event.id == *id) {
            self.shutdown_token.cancel();
            self.tray_icon.take();
            event_loop.exit();
            return;
        }

        if self.open_ui_id.as_ref().is_some_and(|id| event.id == *id) {
            let root = self.root.read().unwrap().clone();
            open_ui(root.as_deref());
            return;
        }

        if self.login_id.as_ref().is_some_and(|id| event.id == *id) {
            if let Some(port) = self.mode.port() {
                toggle_login_item(port, self.login_item.as_ref());
            }
        }
    }
}

impl ApplicationHandler for TrayApp {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            self.initialize_tray(event_loop);
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            self.handle_menu_event(event_loop, event);
        }

        if self
            .server_handle
            .as_ref()
            .is_some_and(|handle| handle.is_finished())
        {
            self.tray_icon.take();
            event_loop.exit();
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }
}

pub fn run_with_tray(mode: GateServerMode) -> i32 {
    #[cfg(not(target_os = "macos"))]
    {
        return run_headless(mode);
    }

    #[cfg(target_os = "macos")]
    {
        let root = Arc::new(RwLock::new(None));
        let shutdown_token = CancellationToken::new();
        let file_watcher = match GateFileWatcher::start(root.clone(), &shutdown_token) {
            Ok(watcher) => watcher,
            Err(err) => {
                eprintln!("Error starting gate file watcher: {err}");
                return 1;
            }
        };
        let server_shutdown_token = shutdown_token.clone();
        let server_root = root.clone();
        let server_handle = std::thread::Builder::new()
            .name("gate-mcp-server".into())
            .spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(err) => {
                        eprintln!("Error creating async runtime: {err}");
                        return 1;
                    }
                };
                let result = rt.block_on(async move {
                    tokio::select! {
                        result = run_mode(mode, server_root) => result,
                        _ = server_shutdown_token.cancelled() => Ok(()),
                    }
                });
                rt.shutdown_timeout(std::time::Duration::from_secs(5));
                match result {
                    Ok(()) => 0,
                    Err(err) => {
                        eprintln!("Server error: {err}");
                        1
                    }
                }
            })
            .expect("failed to spawn gate MCP server thread");

        let mut builder = EventLoop::builder();
        builder.with_activation_policy(ActivationPolicy::Accessory);
        let event_loop = builder.build().expect("build tray event loop");
        let mut app = TrayApp {
            mode,
            root,
            shutdown_token,
            _file_watcher: Some(file_watcher),
            server_handle: Some(server_handle),
            tray_icon: None,
            quit_id: None,
            open_ui_id: None,
            login_id: None,
            login_item: None,
            exit_code: 0,
        };

        if let Err(err) = event_loop.run_app(&mut app) {
            eprintln!("Error running gate tray event loop: {err}");
            app.shutdown_token.cancel();
            app.exit_code = 1;
        }

        let server_exit_code = match app.server_handle.take() {
            Some(handle) => handle.join().unwrap_or(1),
            None => 1,
        };

        if app.exit_code != 0 {
            app.exit_code
        } else {
            server_exit_code
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn run_headless(mode: GateServerMode) -> i32 {
    let root = Arc::new(RwLock::new(None));
    let shutdown_token = CancellationToken::new();
    let _file_watcher = match GateFileWatcher::start(root.clone(), &shutdown_token) {
        Ok(watcher) => watcher,
        Err(err) => {
            eprintln!("Error starting gate file watcher: {err}");
            return 1;
        }
    };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("Error creating async runtime: {err}");
            return 1;
        }
    };
    let result = rt.block_on(run_mode(mode, root));
    shutdown_token.cancel();
    rt.shutdown_timeout(std::time::Duration::from_secs(5));
    match result {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("Server error: {err}");
            1
        }
    }
}

async fn run_mode(
    mode: GateServerMode,
    root: Arc<RwLock<Option<PathBuf>>>,
) -> Result<(), crate::error::GateError> {
    match mode {
        GateServerMode::Stdio => GateMcpServer::serve_stdio(root).await,
        GateServerMode::Http(port) => GateMcpServer::serve_http(port, root).await,
    }
}

fn build_icon() -> Icon {
    let rgba = include_bytes!("tray_icon.rgba");
    Icon::from_rgba(rgba.to_vec(), 64, 64).expect("valid 64x64 RGBA tray icon")
}

pub(crate) fn tooltip_for_mode(mode: GateServerMode) -> String {
    match mode {
        GateServerMode::Stdio => "gate MCP stdio".to_string(),
        GateServerMode::Http(port) => format!("gate MCP :{port}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{GateServerMode, tooltip_for_mode};

    #[test]
    fn tray_tooltip_renders_stdio_mode() {
        assert_eq!(tooltip_for_mode(GateServerMode::Stdio), "gate MCP stdio");
    }

    #[test]
    fn tray_tooltip_renders_http_port_mode() {
        assert_eq!(
            tooltip_for_mode(GateServerMode::Http(8080)),
            "gate MCP :8080"
        );
    }
}
