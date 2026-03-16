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

use tokio_util::sync::CancellationToken;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

// ── Icon ──────────────────────────────────────────────────────────────

/// Build a 32×32 RGBA icon with a simple "M" glyph (placeholder).
fn build_icon() -> Icon {
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            if is_m_pixel(x, y, size) {
                rgba.extend_from_slice(&[220, 220, 220, 255]); // light gray M
            } else {
                rgba.extend_from_slice(&[60, 60, 60, 255]); // dark background
            }
        }
    }
    Icon::from_rgba(rgba, size, size).expect("valid 32×32 RGBA icon")
}

/// Rough lowercase "m" letter mask for a 32×32 grid.
fn is_m_pixel(x: u32, y: u32, size: u32) -> bool {
    let top = size * 3 / 8;
    let baseline = size * 3 / 4;

    if y < top || y > baseline {
        return false;
    }

    let left_stem = size * 3 / 16;
    let first_peak = size * 13 / 32;
    let middle_stem = size / 2 + 1;
    let second_peak = size * 21 / 32;
    let right_stem = size * 3 / 4 + 1;
    let stroke_half_width = 1.5f32; // ≈3 px strokes on a 32×32 icon

    let px = x as f32;
    let py = y as f32;

    let hits_segment = |x0: u32, y0: u32, x1: u32, y1: u32| -> bool {
        let x0 = x0 as f32;
        let y0 = y0 as f32;
        let x1 = x1 as f32;
        let y1 = y1 as f32;
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len_sq = dx * dx + dy * dy;
        if len_sq == 0.0 {
            return (px - x0).abs() <= stroke_half_width && (py - y0).abs() <= stroke_half_width;
        }

        let t = (((px - x0) * dx + (py - y0) * dy) / len_sq).clamp(0.0, 1.0);
        let nearest_x = x0 + t * dx;
        let nearest_y = y0 + t * dy;
        let dist_sq = (px - nearest_x).powi(2) + (py - nearest_y).powi(2);
        dist_sq <= stroke_half_width.powi(2)
    };

    hits_segment(left_stem, top, left_stem, baseline)
        || hits_segment(left_stem + 1, baseline, first_peak, top)
        || hits_segment(first_peak, top, middle_stem, baseline)
        || hits_segment(middle_stem, top + 3, middle_stem, baseline)
        || hits_segment(middle_stem, baseline, second_peak, top)
        || hits_segment(second_peak, top, right_stem, baseline)
        || hits_segment(right_stem, top, right_stem, baseline)
}

// ── URL helpers ───────────────────────────────────────────────────────

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = url; // suppress unused warning on non-macOS
    }
}

fn launch_agent_path() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Library/LaunchAgents/com.ma-collective.maenifold.plist")
}

fn is_login_enabled() -> bool {
    launch_agent_path().exists()
}

fn login_menu_text(enabled: bool) -> &'static str {
    if enabled {
        "✓ Open at Login"
    } else {
        "Open at Login"
    }
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ── TrayApp (winit ApplicationHandler) ────────────────────────────────

/// State for the winit `ApplicationHandler` that drives the tray icon.
struct TrayApp {
    port: u16,
    shutdown_token: CancellationToken,
    server_handle: Option<std::thread::JoinHandle<i32>>,
    // Tray state — initialised in `new_events` when `cause == StartCause::Init`.
    tray_icon: Option<tray_icon::TrayIcon>,
    quit_id: Option<MenuId>,
    help_id: Option<MenuId>,
    login_id: Option<MenuId>,
    login_item: Option<MenuItem>,
    exit_code: i32,
}

impl TrayApp {
    #[cfg(target_os = "macos")]
    fn write_launch_agent(&self) -> std::io::Result<()> {
        let path = launch_agent_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let exe_path = std::env::current_exe()?;
        let mode_flag = "--http";
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.ma-collective.maenifold</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>{mode_flag}</string>
        <string>--port</string>
        <string>{port}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
            exe_path = xml_escape(&exe_path.display().to_string()),
            mode_flag = mode_flag,
            port = self.port,
        );

        std::fs::write(path, plist)
    }

    #[cfg(target_os = "macos")]
    fn remove_launch_agent(&self) -> std::io::Result<()> {
        let path = launch_agent_path();
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }

    #[cfg(target_os = "macos")]
    fn refresh_login_item(&self) {
        if let Some(item) = &self.login_item {
            item.set_text(login_menu_text(is_login_enabled()));
        }
    }

    fn initialize_tray(&mut self, event_loop: &ActiveEventLoop) {
        let login_enabled = is_login_enabled();

        let menu = Menu::new();

        #[cfg(target_os = "macos")]
        let login_item = MenuItem::new(login_menu_text(login_enabled), true, None);
        #[cfg(not(target_os = "macos"))]
        let login_item = MenuItem::new(login_menu_text(login_enabled), false, None);
        let _ = menu.append(&login_item);

        let help_item = MenuItem::new("Help", true, None);
        let _ = menu.append(&help_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let quit_item = MenuItem::new("Quit", true, None);
        let _ = menu.append(&quit_item);

        self.quit_id = Some(quit_item.id().clone());
        self.help_id = Some(help_item.id().clone());
        self.login_id = Some(login_item.id().clone());
        self.login_item = Some(login_item);

        let icon = build_icon();
        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("maenifold")
            .with_icon(icon)
            .build()
        {
            Ok(tray_icon) => {
                self.tray_icon = Some(tray_icon);
            }
            Err(err) => {
                eprintln!("Warning: failed to create tray icon: {err}");
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
            self.shutdown_token.cancel();
            self.tray_icon.take();
            event_loop.exit();
            return;
        }

        if self.help_id.as_ref().is_some_and(|id| event.id == *id) {
            open_url("https://github.com/ma-collective/maenifold-rs/issues");
        }

        if self.login_id.as_ref().is_some_and(|id| event.id == *id) {
            #[cfg(target_os = "macos")]
            {
                let result = if is_login_enabled() {
                    self.remove_launch_agent()
                } else {
                    self.write_launch_agent()
                };

                if let Err(err) = result {
                    eprintln!("Warning: failed to toggle launch agent: {err}");
                }

                self.refresh_login_item();
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
        // No windows — nothing to do.
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
pub fn run_with_tray(port: u16) -> i32 {
    // ── Initialise runtime (mirrors cli::run_http pattern) ──
    let config = crate::config::Config::current();
    if let Err(err) = crate::mcp::initialize_runtime(&config) {
        eprintln!("Error initializing storage: {err}");
        return 1;
    }
    if let Err(err) = crate::cli::start_mcp_watchers(&config) {
        eprintln!("Error starting MCP watchers: {err}");
        return 1;
    }

    let shutdown_token = CancellationToken::new();

    // ── Spawn server on background thread ──────────────────────────────
    let server_shutdown_token = shutdown_token.clone();
    let server_handle = std::thread::Builder::new()
        .name("maenifold-server".into())
        .spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(err) => {
                    eprintln!("Error creating async runtime: {err}");
                    return 1i32;
                }
            };
            let result = rt.block_on(async move {
                tokio::select! {
                    result = crate::mcp::MaenifoldMcpServer::serve_http(port) => result,
                    _ = server_shutdown_token.cancelled() => Ok(()),
                }
            });
            match result {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("Server error: {err}");
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
        port,
        shutdown_token,
        server_handle: Some(server_handle),
        tray_icon: None,
        quit_id: None,
        help_id: None,
        login_id: None,
        login_item: None,
        exit_code: 0,
    };

    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("Error running tray event loop: {err}");
        app.shutdown_token.cancel();
        app.exit_code = 1;
    }

    // ── Join server thread and return its exit code ────────────────────
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
