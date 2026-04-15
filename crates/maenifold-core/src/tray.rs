//! T-29.2: RTM FR-35 — System tray icon for `--http` mode on macOS.
//!
//! Architecture (required by macOS):
//! - **Main thread**: winit event loop driving the tray icon + menu.
//! - **Background `std::thread`**: tokio runtime + axum server.
//!
//! The winit `EventLoop` properly integrates with macOS's `NSRunLoop`,
//! ensuring menu events are delivered reliably.
//!
//! `--mcp` mode never enters this module — it remains fully headless.

use crate::tray_menu::{CoreTrayMenuState, build_tray_menu, endophasia_url, toggle_login_item};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tray_icon::menu::{MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

/// Which HTTP-based server mode was requested.
#[derive(Debug, Clone, Copy)]
pub enum ServerMode {
    Http(u16),
}

impl ServerMode {
    pub fn port(self) -> u16 {
        match self {
            Self::Http(p) => p,
        }
    }
}

// ── Icon ──────────────────────────────────────────────────────────────

/// Build a 64×64 RGBA icon from the embedded font-rendered glyph.
fn build_icon() -> Icon {
    let rgba = include_bytes!("tray_icon.rgba");
    Icon::from_rgba(rgba.to_vec(), 64, 64).expect("valid 64×64 RGBA icon")
}

// ── URL helpers ───────────────────────────────────────────────────────

fn open_url(url: &str) -> crate::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(crate::error::MaenifoldError::Message(format!(
            "No URL launcher configured for this platform; cannot open {url}"
        )))
    }
}

fn resolve_open_url(
    event_id: &MenuId,
    endophasia_id: Option<&MenuId>,
    help_id: Option<&MenuId>,
    mode: ServerMode,
) -> Option<String> {
    if endophasia_id.is_some_and(|id| event_id == id) {
        return Some(endophasia_url(mode.port()));
    }

    if help_id.is_some_and(|id| event_id == id) {
        return Some("https://github.com/ma-collective/maenifold-rs/issues".to_string());
    }

    None
}

// ── TrayApp (winit ApplicationHandler) ────────────────────────────────

/// State for the winit `ApplicationHandler` that drives the tray icon.
pub struct TrayApp<Mode, State> {
    pub mode: Mode,
    pub shutdown_token: CancellationToken,
    pub server_handle: Option<std::thread::JoinHandle<i32>>,
    pub tray_icon: Option<tray_icon::TrayIcon>,
    pub quit_id: Option<MenuId>,
    pub exit_code: i32,
    pub state: State,
}

#[derive(Default)]
struct CoreTrayState {
    endophasia_id: Option<MenuId>,
    help_id: Option<MenuId>,
    login_id: Option<MenuId>,
    restart_id: Option<MenuId>,
    login_item: Option<MenuItem>,
    restart_on_exit: bool,
}

impl TrayApp<ServerMode, CoreTrayState> {
    fn initialize_tray(&mut self, event_loop: &ActiveEventLoop) {
        let menu_state = build_tray_menu();
        let CoreTrayMenuState {
            endophasia_id,
            help_id,
            login_id,
            restart_id,
            login_item,
        } = menu_state.state;
        self.quit_id = Some(menu_state.quit_id);
        self.state.endophasia_id = Some(endophasia_id);
        self.state.help_id = Some(help_id);
        self.state.login_id = Some(login_id);
        self.state.restart_id = restart_id;
        self.state.login_item = Some(login_item);

        let icon = build_icon();
        match TrayIconBuilder::new()
            .with_menu(Box::new(menu_state.menu))
            .with_tooltip("maenifold")
            .with_icon(icon)
            .build()
        {
            Ok(tray_icon) => {
                self.tray_icon = Some(tray_icon);
            }
            Err(err) => {
                warn!(error = %err, "failed to create tray icon");
                self.shutdown_token.cancel();
                self.exit_code = 1;
                event_loop.exit();
                return;
            }
        }

        #[cfg(target_os = "macos")]
        {
            use objc2_core_foundation::CFRunLoop;
            let rl = CFRunLoop::main().unwrap();
            rl.wake_up();
        }
    }

    fn handle_menu_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: tray_icon::menu::MenuEvent,
    ) {
        if self.quit_id.as_ref().is_some_and(|id| event.id == *id) {
            info!("maenifold tray quit requested");
            self.shutdown_token.cancel();
            self.tray_icon.take();
            event_loop.exit();
            return;
        }

        if let Some(url) = resolve_open_url(
            &event.id,
            self.state.endophasia_id.as_ref(),
            self.state.help_id.as_ref(),
            self.mode,
        ) {
            if let Err(err) = open_url(&url) {
                let label = if self
                    .state
                    .endophasia_id
                    .as_ref()
                    .is_some_and(|id| event.id == *id)
                {
                    "Endophasia"
                } else {
                    "help"
                };
                error!(label, error = %err, "failed to open tray URL");
            }
            return;
        }

        if self
            .state
            .restart_id
            .as_ref()
            .is_some_and(|id| event.id == *id)
        {
            info!("maenifold tray restart initiated");
            self.state.restart_on_exit = true;
            self.shutdown_token.cancel();
            self.tray_icon.take();
            event_loop.exit();
            return;
        }

        if self
            .state
            .login_id
            .as_ref()
            .is_some_and(|id| event.id == *id)
        {
            toggle_login_item(self.mode.port(), self.state.login_item.as_ref());
        }
    }
}

impl ApplicationHandler for TrayApp<ServerMode, CoreTrayState> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            self.initialize_tray(event_loop);
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            self.handle_menu_event(event_loop, event);
        }

        if self.server_handle.as_ref().is_some_and(|h| h.is_finished()) {
            self.tray_icon.take();
            event_loop.exit();
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // No windows — nothing to do.
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }
}

// ── Public entry point ────────────────────────────────────────────────

/// Run the server with a system-tray icon on the main thread.
///
/// 1. Initialises storage / watchers (matching `CliApp::run_http`).
/// 2. Spawns the tokio/axum server on a background `std::thread`.
/// 3. Builds a winit `EventLoop` and drives the tray icon via `ApplicationHandler`.
/// 4. On "Quit", cancels the shared shutdown token so the server thread exits.
/// 5. Joins the server thread and returns its exit code.
#[allow(clippy::too_many_lines)]
pub fn run_with_tray(mode: ServerMode) -> i32 {
    let config = crate::config::Config::current();
    if let Err(err) = crate::mcp::initialize_runtime(&config) {
        error!(error = %err, "failed to initialize storage");
        return 1;
    }
    if let Err(err) = crate::cli::start_mcp_watchers(&config) {
        error!(error = %err, "failed to start MCP watchers");
        return 1;
    }

    let port = mode.port();
    let shutdown_token = CancellationToken::new();

    // ── Spawn server on background thread ──────────────────────────────
    let server_shutdown_token = shutdown_token.clone();
    let server_handle = std::thread::Builder::new()
        .name("maenifold-server".into())
        .spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(err) => {
                    error!(error = %err, "failed to create async runtime");
                    return 1i32;
                }
            };
            let result = rt.block_on(async move {
                crate::mcp::MaenifoldMcpServer::serve_http_with_shutdown(
                    port,
                    server_shutdown_token,
                )
                .await
            });
            rt.shutdown_timeout(Duration::from_secs(5));
            match result {
                Ok(()) => 0,
                Err(err) => {
                    error!(error = %err, "maenifold tray server exited with error");
                    1
                }
            }
        })
        .expect("failed to spawn server thread");

    // ── Build winit event loop & run ───────────────────────────────────
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    builder.with_activation_policy(ActivationPolicy::Accessory);
    let event_loop = builder.build().unwrap();
    let mut app = TrayApp {
        mode,
        shutdown_token,
        server_handle: Some(server_handle),
        tray_icon: None,
        quit_id: None,
        exit_code: 0,
        state: CoreTrayState::default(),
    };

    if let Err(err) = event_loop.run_app(&mut app) {
        error!(error = %err, "failed to run tray event loop");
        app.shutdown_token.cancel();
        app.exit_code = 1;
    }

    // ── Join server thread and return its exit code ────────────────────
    let server_exit_code = match app.server_handle.take() {
        Some(handle) => handle.join().unwrap_or(1),
        None => 1,
    };

    // Spawn replacement process AFTER the server thread has joined
    // (and thus released the listening port).
    if app.state.restart_on_exit {
        match std::env::current_exe() {
            Ok(exe) => {
                let args: Vec<String> = std::env::args().skip(1).collect();
                match std::process::Command::new(exe).args(&args).spawn() {
                    Ok(child) => {
                        info!(pid = child.id(), ?args, "spawned maenifold tray restart process");
                    }
                    Err(err) => {
                        warn!(error = %err, ?args, "failed to spawn restart process");
                    }
                }
            }
            Err(err) => {
                warn!(error = %err, "failed to resolve executable path for restart");
            }
        }
        return 0;
    }

    if app.exit_code != 0 {
        app.exit_code
    } else {
        server_exit_code
    }
}

#[cfg(test)]
mod tests {
    use super::{ServerMode, resolve_open_url};
    use tray_icon::menu::MenuId;

    #[test]
    fn endophasia_menu_routes_to_runtime_browser_url() {
        assert_eq!(
            resolve_open_url(
                &MenuId::new("endophasia"),
                Some(&MenuId::new("endophasia")),
                Some(&MenuId::new("help")),
                ServerMode::Http(1234),
            ),
            Some("http://127.0.0.1:1234/endophasia".to_string())
        );
    }

    #[test]
    fn help_menu_routes_to_issues_url() {
        assert_eq!(
            resolve_open_url(
                &MenuId::new("help"),
                Some(&MenuId::new("endophasia")),
                Some(&MenuId::new("help")),
                ServerMode::Http(1234),
            ),
            Some("https://github.com/ma-collective/maenifold-rs/issues".to_string())
        );
    }

    #[test]
    fn unrelated_menu_ids_do_not_open_urls() {
        assert_eq!(
            resolve_open_url(
                &MenuId::new("quit"),
                Some(&MenuId::new("endophasia")),
                Some(&MenuId::new("help")),
                ServerMode::Http(1234),
            ),
            None
        );
    }

    #[test]
    fn missing_menu_ids_do_not_open_urls() {
        assert_eq!(
            resolve_open_url(
                &MenuId::new("endophasia"),
                None,
                None,
                ServerMode::Http(1234)
            ),
            None
        );
    }

    #[test]
    fn server_mode_port_returns_http_port() {
        assert_eq!(ServerMode::Http(1234).port(), 1234);
    }
}
