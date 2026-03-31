use tokio_util::sync::CancellationToken;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
fn build_icon() -> Icon {
    Icon::from_rgba(include_bytes!("tray_icon.rgba").to_vec(), 64, 64)
        .expect("valid 64x64 RGBA icon")
}
fn launch_agent_path() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Library/LaunchAgents/com.ma-collective.gate.plist")
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
fn restart_daemon(service_label: &str) {
    #[cfg(target_os = "macos")]
    {
        let output = match std::process::Command::new("id").arg("-u").output() {
            Ok(output) if output.status.success() => output,
            Ok(output) => {
                eprintln!(
                    "Warning: failed to resolve uid for restart (status {}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                );
                return;
            }
            Err(err) => {
                eprintln!("Warning: failed to resolve uid for restart: {err}");
                return;
            }
        };
        let uid = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if uid.is_empty() {
            eprintln!("Warning: restart aborted because uid lookup returned empty output");
            return;
        }
        let target = format!("gui/{uid}/{service_label}");
        if let Err(err) = std::process::Command::new("launchctl")
            .args(["kickstart", "-k", &target])
            .spawn()
        {
            eprintln!("Warning: failed to restart {service_label}: {err}");
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = service_label;
    }
}
struct TrayApp {
    port: u16,
    shutdown_token: CancellationToken,
    server_handle: Option<std::thread::JoinHandle<i32>>,
    tray_icon: Option<tray_icon::TrayIcon>,
    quit_id: Option<MenuId>,
    kanban_id: Option<MenuId>,
    trace_id: Option<MenuId>,
    login_id: Option<MenuId>,
    restart_id: Option<MenuId>,
    login_item: Option<MenuItem>,
    exit_code: i32,
}
impl TrayApp {
    fn open_local(&self, path: &str) {
        let url = format!("http://localhost:{}{path}", self.port);
        if let Err(err) = open::that(url) {
            eprintln!("Warning: failed to open browser: {err}");
        }
    }
    #[cfg(target_os = "macos")]
    fn write_launch_agent(&self) -> std::io::Result<()> {
        let path = launch_agent_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let exe_path = std::env::current_exe()?;
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.ma-collective.gate</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>--http</string>
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
            port = self.port,
        );
        std::fs::write(path, plist)
    }
    #[cfg(target_os = "macos")]
    fn remove_launch_agent(&self) -> std::io::Result<()> {
        match std::fs::remove_file(launch_agent_path()) {
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
        let menu = Menu::new();
        let login_enabled = is_login_enabled();
        let kanban_item = MenuItem::new("Kanban Board", true, None);
        let trace_item = MenuItem::new("Trace", true, None);
        #[cfg(target_os = "macos")]
        let login_item = MenuItem::new(login_menu_text(login_enabled), true, None);
        #[cfg(not(target_os = "macos"))]
        let login_item = MenuItem::new(login_menu_text(login_enabled), false, None);
        let _ = menu.append(&kanban_item);
        let _ = menu.append(&trace_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&login_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        #[cfg(target_os = "macos")]
        let restart_item = {
            let item = MenuItem::new("Restart", true, None);
            let _ = menu.append(&item);
            Some(item)
        };
        #[cfg(not(target_os = "macos"))]
        let restart_item: Option<MenuItem> = None;
        let quit_item = MenuItem::new("Quit", true, None);
        let _ = menu.append(&quit_item);
        self.quit_id = Some(quit_item.id().clone());
        self.kanban_id = Some(kanban_item.id().clone());
        self.trace_id = Some(trace_item.id().clone());
        self.login_id = Some(login_item.id().clone());
        self.restart_id = restart_item.map(|item| item.id().clone());
        self.login_item = Some(login_item);
        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("gate")
            .with_icon(build_icon())
            .build()
        {
            Ok(tray_icon) => self.tray_icon = Some(tray_icon),
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
            CFRunLoop::main().unwrap().wake_up();
        }
    }
    fn handle_menu_event(&mut self, event_loop: &ActiveEventLoop, event: MenuEvent) {
        if self.quit_id.as_ref().is_some_and(|id| event.id == *id) {
            self.shutdown_token.cancel();
            self.tray_icon.take();
            event_loop.exit();
            return;
        }
        if self.kanban_id.as_ref().is_some_and(|id| event.id == *id) {
            self.open_local("/");
        }
        if self.trace_id.as_ref().is_some_and(|id| event.id == *id) {
            self.open_local("/trace");
        }
        if self.restart_id.as_ref().is_some_and(|id| event.id == *id) {
            restart_daemon("com.ma-collective.gate");
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
        _: &ActiveEventLoop,
        _: winit::window::WindowId,
        _: winit::event::WindowEvent,
    ) {
    }
}
pub fn run_with_tray(port: u16) -> i32 {
    let shutdown_token = CancellationToken::new();
    let server_shutdown_token = shutdown_token.clone();
    let server_handle = std::thread::Builder::new()
        .name("gate-mcp-http".into())
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
                    result = crate::mcp::GateMcpServer::serve_http(port) => result,
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
        kanban_id: None,
        trace_id: None,
        login_id: None,
        restart_id: None,
        login_item: None,
        exit_code: 0,
    };
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("Error running tray event loop: {err}");
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
