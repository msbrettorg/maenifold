use std::process::Command;

use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

pub(crate) struct TrayMenuState {
    pub menu: Menu,
    pub quit_id: MenuId,
    pub help_id: MenuId,
    pub login_id: MenuId,
    pub restart_id: Option<MenuId>,
    pub login_item: MenuItem,
}

pub(crate) fn build_tray_menu() -> TrayMenuState {
    let menu = Menu::new();
    let login_enabled = is_login_enabled();

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
        help_id: help_item.id().clone(),
        login_id: login_item.id().clone(),
        restart_id: restart_item.map(|item| item.id().clone()),
        login_item,
    }
}

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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (port, login_item);
    }
}

pub fn restart_daemon(service_label: &str) {
    #[cfg(target_os = "macos")]
    {
        let output = match Command::new("id").arg("-u").output() {
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
        if let Err(err) = Command::new("launchctl")
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
    <string>com.ma-collective.maenifold</string>
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
    <false/>
</dict>
</plist>
"#,
        exe_path = xml_escape(&exe_path.display().to_string()),
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
