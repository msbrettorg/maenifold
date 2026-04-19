use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use maenifold_core::tray::TrayApp;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tray_icon::menu::{MenuEvent, MenuId, MenuItem};
use tray_icon::TrayIconBuilder;
use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, EventLoop};

#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

use crate::file_watcher::GateFileWatcher;
use crate::tray_lifecycle::*;
use crate::tray_menu::{
    GateTrayMenuState, build_tray_menu, open_ui, set_open_ui_enabled, toggle_login_item,
};

struct GateTrayState {
    root: Arc<RwLock<Option<PathBuf>>>,
    open_ui_id: Option<MenuId>,
    open_item: Option<MenuItem>,
    login_id: Option<MenuId>,
    login_item: Option<MenuItem>,
}

struct GateTrayApp {
    tray: TrayApp<GateServerMode, GateTrayState>,
    _file_watcher: Option<GateFileWatcher>,
    root_monitor: Option<JoinHandle<()>>,
}

impl GateTrayApp {
    fn initialize_tray(&mut self, event_loop: &ActiveEventLoop) {
        let has_root = self.tray.state.root.read().unwrap().is_some();
        let menu_state = build_tray_menu(self.tray.mode, has_root);
        let GateTrayMenuState {
            open_ui_id,
            open_item,
            login_id,
            login_item,
        } = menu_state.state;
        self.tray.quit_id = Some(menu_state.quit_id);
        self.tray.state.open_ui_id = open_ui_id;
        self.tray.state.open_item = open_item;
        self.tray.state.login_id = login_id;
        self.tray.state.login_item = login_item;

        match TrayIconBuilder::new()
            .with_menu(Box::new(menu_state.menu))
            .with_tooltip(tooltip_for_mode(self.tray.mode))
            .with_icon(build_icon())
            .build()
        {
            Ok(tray_icon) => {
                self.tray.tray_icon = Some(tray_icon);
            }
            Err(err) => {
                warn!(error = %err, "failed to create gate tray icon");
                self.tray.shutdown_token.cancel();
                self.tray.exit_code = 1;
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
        if self.tray.quit_id.as_ref().is_some_and(|id| event.id == *id) {
            info!("gate tray quit requested");
            self.tray.shutdown_token.cancel();
            self.tray.tray_icon.take();
            event_loop.exit();
            return;
        }

        if self
            .tray
            .state
            .open_ui_id
            .as_ref()
            .is_some_and(|id| event.id == *id)
        {
            let root = self.tray.state.root.read().unwrap().clone();
            open_ui(root.as_deref());
            return;
        }

        if self
            .tray
            .state
            .login_id
            .as_ref()
            .is_some_and(|id| event.id == *id)
            && let Some(port) = self.tray.mode.port()
        {
            let root = self.tray.state.root.read().unwrap().clone();
            toggle_login_item(port, root.as_deref(), self.tray.state.login_item.as_ref());
        }
    }

    fn handle_root_availability_changed(&mut self, has_root: bool) {
        set_open_ui_enabled(self.tray.state.open_item.as_ref(), has_root);
    }
}

impl ApplicationHandler<TrayUserEvent> for GateTrayApp {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            self.initialize_tray(event_loop);
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            self.handle_menu_event(event_loop, event);
        }

        if self
            .tray
            .server_handle
            .as_ref()
            .is_some_and(|handle| handle.is_finished())
        {
            self.tray.shutdown_token.cancel();
            self.tray.tray_icon.take();
            event_loop.exit();
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: TrayUserEvent) {
        match event {
            TrayUserEvent::RootAvailabilityChanged(has_root) => {
                self.handle_root_availability_changed(has_root);
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }
}

pub fn run_with_tray(mode: GateServerMode, initial_root: Option<PathBuf>) -> i32 {
    #[cfg(not(target_os = "macos"))]
    {
        return run_headless(mode, initial_root);
    }

    #[cfg(target_os = "macos")]
    {
        let server_initial_root = initial_root.clone();
        let root = Arc::new(RwLock::new(initial_root));
        let shutdown_token = CancellationToken::new();
        let file_watcher = match GateFileWatcher::start(root.clone(), &shutdown_token) {
            Ok(watcher) => watcher,
            Err(err) => {
                error!(error = %err, "failed to start gate file watcher");
                return 1;
            }
        };
        let server_shutdown_token = shutdown_token.clone();
        let known_roots = crate::mcp::KnownRoots::with_initial(server_initial_root);
        let server_handle = std::thread::Builder::new()
            .name("gate-mcp-server".into())
            .spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(err) => {
                        error!(error = %err, "failed to create async runtime");
                        return 1;
                    }
                };
                let result = rt.block_on(async move {
                    tokio::select! {
                        result = run_mode(mode, known_roots) => result,
                        _ = server_shutdown_token.cancelled() => Ok(()),
                    }
                });
                rt.shutdown_timeout(std::time::Duration::from_secs(5));
                match result {
                    Ok(()) => 0,
                    Err(err) => {
                        error!(error = %err, "gate tray server exited with error");
                        1
                    }
                }
            })
            .expect("failed to spawn gate MCP server thread");

        let mut builder = EventLoop::<TrayUserEvent>::with_user_event();
        builder.with_activation_policy(ActivationPolicy::Accessory);
        let event_loop = builder.build().expect("build tray event loop");
        let root_monitor = spawn_root_monitor(
            root.clone(),
            shutdown_token.clone(),
            event_loop.create_proxy(),
        );
        let mut app = GateTrayApp {
            tray: TrayApp {
                mode,
                shutdown_token,
                server_handle: Some(server_handle),
                tray_icon: None,
                quit_id: None,
                exit_code: 0,
                state: GateTrayState {
                    root,
                    open_ui_id: None,
                    open_item: None,
                    login_id: None,
                    login_item: None,
                },
            },
            _file_watcher: Some(file_watcher),
            root_monitor: Some(root_monitor),
        };

        if let Err(err) = event_loop.run_app(&mut app) {
            error!(error = %err, "failed to run gate tray event loop");
            app.tray.shutdown_token.cancel();
            app.tray.exit_code = 1;
        }
        app.tray.shutdown_token.cancel();

        let server_exit_code = match app.tray.server_handle.take() {
            Some(handle) => handle.join().unwrap_or(1),
            None => 1,
        };
        if let Some(handle) = app.root_monitor.take() {
            let _ = handle.join();
        }

        if app.tray.exit_code != 0 {
            app.tray.exit_code
        } else {
            server_exit_code
        }
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
