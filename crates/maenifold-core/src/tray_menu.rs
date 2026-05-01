#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(target_os = "macos")]
use libc;

use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

const SERVICE_LABEL: &str = "com.ma-collective.maenifold";
const ENDOPHASIA_MENU_TITLE: &str = "Endophasia";

#[cfg(target_os = "macos")]
fn get_uid() -> u32 {
    // SAFETY: getuid() is always safe — no arguments, no failure mode.
    unsafe { libc::getuid() }
}

pub struct TrayMenuState<State> {
    pub menu: Menu,
    pub quit_id: MenuId,
    pub state: State,
}

pub(crate) struct CoreTrayMenuState {
    pub endophasia_id: MenuId,
    pub help_id: MenuId,
    pub login_id: MenuId,
    pub restart_id: Option<MenuId>,
    pub login_item: MenuItem,
}

pub(crate) fn build_tray_menu() -> TrayMenuState<CoreTrayMenuState> {
    let menu = Menu::new();
    let login_enabled = is_login_enabled();
    let endophasia_item = MenuItem::new(ENDOPHASIA_MENU_TITLE, true, None);
    let _ = menu.append(&endophasia_item);

    #[cfg(target_os = "macos")]
    let login_item = MenuItem::new(login_menu_text(login_enabled), true, None);
    #[cfg(not(target_os = "macos"))]
    let login_item = MenuItem::new(login_menu_text(login_enabled), false, None);
    let _ = menu.append(&login_item);

    let help_item = MenuItem::new("Help", true, None);
    let _ = menu.append(&help_item);
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

    TrayMenuState {
        menu,
        quit_id: quit_item.id().clone(),
        state: CoreTrayMenuState {
            endophasia_id: endophasia_item.id().clone(),
            help_id: help_item.id().clone(),
            login_id: login_item.id().clone(),
            restart_id: restart_item.map(|item| item.id().clone()),
            login_item,
        },
    }
}

pub(crate) fn endophasia_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/endophasia")
}

pub(crate) fn toggle_login_item(port: u16, login_item: Option<&MenuItem>) {
    #[cfg(target_os = "macos")]
    {
        let result = if is_login_enabled() {
            bootout_launch_agent();
            remove_launch_agent()
        } else {
            let r = write_launch_agent(port);
            if r.is_ok() {
                bootstrap_launch_agent();
            }
            r
        };

        if let Err(err) = result {
            eprintln!("Warning: failed to toggle launch agent: {err}");
        }

        if let Some(item) = login_item {
            item.set_text(login_menu_text(is_login_enabled()));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (port, login_item);
    }
}

#[cfg(target_os = "macos")]
fn bootstrap_launch_agent() {
    let uid = get_uid();
    let path = launch_agent_path();
    let domain = format!("gui/{uid}");
    match Command::new("launchctl")
        .args(["bootstrap", &domain, &path.display().to_string()])
        .output()
    {
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Exit code 37 = "service already loaded" — not an error
            if !stderr.contains("already loaded") && output.status.code() != Some(37) {
                eprintln!("Warning: launchctl bootstrap failed: {}", stderr.trim());
            }
        }
        Err(err) => eprintln!("Warning: failed to run launchctl bootstrap: {err}"),
        _ => {}
    }
}

#[cfg(target_os = "macos")]
fn bootout_launch_agent() {
    let uid = get_uid();
    let target = format!("gui/{uid}/{SERVICE_LABEL}");
    match Command::new("launchctl")
        .args(["bootout", &target])
        .output()
    {
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "not loaded" / "no such process" — service may not be registered
            if !stderr.contains("No such process") && !stderr.contains("not loaded") {
                eprintln!("Warning: launchctl bootout failed: {}", stderr.trim());
            }
        }
        Err(err) => eprintln!("Warning: failed to run launchctl bootout: {err}"),
        _ => {}
    }
}

fn launch_agent_path() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(format!("Library/LaunchAgents/{SERVICE_LABEL}.plist"))
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

#[cfg(test)]
mod tests {
    use super::{ENDOPHASIA_MENU_TITLE, endophasia_url};

    #[test]
    fn endophasia_url_uses_runtime_port() {
        assert_eq!(endophasia_url(8087), "http://127.0.0.1:8087/endophasia");
    }

    #[test]
    fn endophasia_menu_title_is_stable() {
        assert_eq!(ENDOPHASIA_MENU_TITLE, "Endophasia");
    }
}
