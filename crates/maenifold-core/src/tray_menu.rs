#[cfg(target_os = "macos")]
use libc;

use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

pub(crate) const SERVICE_LABEL: &str = "com.ma-collective.maenifold";

#[cfg(target_os = "macos")]
fn get_uid() -> u32 {
    // SAFETY: getuid() is always safe — no arguments, no failure mode.
    unsafe { libc::getuid() }
}

#[cfg(target_os = "macos")]
pub(crate) fn launchd_service_target() -> String {
    format!("gui/{}/{SERVICE_LABEL}", get_uid())
}

#[cfg(target_os = "macos")]
pub(crate) fn is_launchd_managed() -> bool {
    // Our process is launchd-managed iff (a) the LaunchAgent plist exists and
    // (b) our parent PID is 1 (launchd). Without (b) we may be a developer's
    // `maenifold --http` run from a shell while a separate launchd job also
    // exists — in that case `launchctl kickstart` would restart a *different*
    // process and the user-launched one would be silently leaked.
    if !launch_agent_path().exists() {
        return false;
    }
    // SAFETY: getppid() is always safe.
    let ppid = unsafe { libc::getppid() };
    ppid == 1
}

pub struct TrayMenuState<State> {
    pub menu: Menu,
    pub quit_id: MenuId,
    pub state: State,
}

pub(crate) struct CoreTrayMenuState {
    pub help_id: MenuId,
    pub login_id: MenuId,
    pub restart_id: Option<MenuId>,
    pub login_item: MenuItem,
}

pub(crate) fn build_tray_menu() -> TrayMenuState<CoreTrayMenuState> {
    let menu = Menu::new();
    let login_enabled = is_login_enabled();

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let login_item = MenuItem::new(login_menu_text(login_enabled), true, None);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let login_item = MenuItem::new(login_menu_text(login_enabled), false, None);
    let _ = menu.append(&login_item);

    let help_item = MenuItem::new("Help", true, None);
    let _ = menu.append(&help_item);
    let _ = menu.append(&PredefinedMenuItem::separator());

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let restart_item = {
        let item = MenuItem::new("Restart", true, None);
        let _ = menu.append(&item);
        Some(item)
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let restart_item: Option<MenuItem> = None;

    let quit_item = MenuItem::new("Quit", true, None);
    let _ = menu.append(&quit_item);

    TrayMenuState {
        menu,
        quit_id: quit_item.id().clone(),
        state: CoreTrayMenuState {
            help_id: help_item.id().clone(),
            login_id: login_item.id().clone(),
            restart_id: restart_item.map(|item| item.id().clone()),
            login_item,
        },
    }
}

/// Toggle the "Open at Login" launchd LaunchAgent plist.
///
/// Treated as a pure autostart-policy toggle:
/// - **Enable**: write `~/Library/LaunchAgents/<label>.plist`. launchd reads
///   this directory at user login, so the change takes effect at the next
///   login. We deliberately do NOT call `launchctl bootstrap` here: with
///   `RunAtLoad=true` it would tell launchd to start a *second* maenifold
///   instance immediately, which would lose the port-8080 binding race
///   against the already-running tray and crash-loop under `KeepAlive`.
/// - **Disable**: remove the plist. We deliberately do NOT call `launchctl
///   bootout` here: bootout SIGTERMs the running service (us) and waits for
///   exit, which kills the tray before this function can complete its job
///   (verified pre-fix: tray icon vanished, plist remained, next login still
///   autostarted the daemon). The user can click Quit if they also want to
///   stop the currently-running daemon.
///
/// Net semantics: "Open at Login" only controls behaviour at the next user
/// login. Lifecycle of the currently-running daemon is controlled by Quit /
/// Restart, independently.
pub(crate) fn toggle_login_item(port: u16, login_item: Option<&MenuItem>) {
    #[cfg(target_os = "macos")]
    {
        let result = if is_login_enabled() {
            remove_launch_agent()
        } else {
            write_launch_agent(port)
        };

        if let Err(err) = result {
            eprintln!("Warning: failed to toggle launch agent: {err}");
        }

        if let Some(item) = login_item {
            item.set_text(login_menu_text(is_login_enabled()));
        }
    }
    #[cfg(target_os = "windows")]
    {
        let result = if is_login_enabled() {
            disable_login_run_key()
        } else {
            enable_login_run_key(port)
        };

        if let Err(err) = result {
            eprintln!("Warning: failed to toggle Run registry entry: {err}");
        }

        if let Some(item) = login_item {
            item.set_text(login_menu_text(is_login_enabled()));
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (port, login_item);
    }
}

#[cfg(target_os = "macos")]
fn launch_agent_path() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(format!("Library/LaunchAgents/{SERVICE_LABEL}.plist"))
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn is_launchd_managed() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn launchd_service_target() -> String {
    String::new()
}

#[cfg(target_os = "macos")]
fn is_login_enabled() -> bool {
    launch_agent_path().exists()
}

#[cfg(target_os = "windows")]
fn is_login_enabled() -> bool {
    run_key_entry_exists()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn is_login_enabled() -> bool {
    false
}

fn login_menu_text(enabled: bool) -> &'static str {
    if enabled {
        "✓ Open at Login"
    } else {
        "Open at Login"
    }
}

#[cfg(target_os = "macos")]
fn write_launch_agent(port: u16) -> std::io::Result<()> {
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
    <string>{service_label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>--http</string>
        <string>--port</string>
        <string>{port}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
</dict>
</plist>
"#,
        exe_path = xml_escape(&exe_path.display().to_string()),
        service_label = SERVICE_LABEL,
        port = port,
    );

    std::fs::write(path, plist)
}

#[cfg(target_os = "macos")]
fn remove_launch_agent() -> std::io::Result<()> {
    match std::fs::remove_file(launch_agent_path()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
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

// ── Windows: "Open at Login" via the HKCU Run registry key ────────────
// Mirrors the macOS launchd approach but shells out to `reg.exe`, keeping
// this dependency-free and consistent with the launchctl shell-out pattern
// above. The current user's Run key starts the tray (`--http`) at logon.
#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
fn login_run_command_value(exe_display: &str, port: u16) -> String {
    format!("\"{exe_display}\" --http --port {port}")
}

#[cfg(target_os = "windows")]
fn run_key_entry_exists() -> bool {
    std::process::Command::new("reg")
        .args(["query", RUN_KEY, "/v", SERVICE_LABEL])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn enable_login_run_key(port: u16) -> std::io::Result<()> {
    let exe_path = std::env::current_exe()?;
    let value = login_run_command_value(&exe_path.display().to_string(), port);
    let output = std::process::Command::new("reg")
        .args([
            "add", RUN_KEY, "/v", SERVICE_LABEL, "/t", "REG_SZ", "/d", &value, "/f",
        ])
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "reg add failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(target_os = "windows")]
fn disable_login_run_key() -> std::io::Result<()> {
    let output = std::process::Command::new("reg")
        .args(["delete", RUN_KEY, "/v", SERVICE_LABEL, "/f"])
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    // "unable to find" → entry already absent; treat as success (idempotent).
    if stderr.contains("unable to find") {
        return Ok(());
    }
    Err(std::io::Error::other(format!(
        "reg delete failed: {}",
        stderr.trim()
    )))
}

#[cfg(test)]
mod tests {
    use super::login_menu_text;

    #[test]
    fn login_menu_text_reflects_enabled_state() {
        assert_eq!(login_menu_text(true), "✓ Open at Login");
        assert_eq!(login_menu_text(false), "Open at Login");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn login_run_command_value_quotes_exe_and_sets_http_port() {
        use super::login_run_command_value;
        assert_eq!(
            login_run_command_value(r"C:\Program Files\maenifold\maenifold.exe", 8080),
            "\"C:\\Program Files\\maenifold\\maenifold.exe\" --http --port 8080"
        );
    }
}
