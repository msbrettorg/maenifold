use std::path::Path;

use tray_icon::menu::{Menu, MenuId, MenuItem};

use crate::tray::GateServerMode;

const SERVICE_LABEL: &str = "com.ma-collective.gate";

pub(crate) struct TrayMenuState {
    pub menu: Menu,
    pub quit_id: MenuId,
    pub login_id: Option<MenuId>,
    pub login_item: Option<MenuItem>,
}

pub(crate) fn build_tray_menu(mode: GateServerMode) -> TrayMenuState {
    let menu = Menu::new();

    let (login_id, login_item) = match mode {
        GateServerMode::Http(port) => {
            #[cfg(target_os = "macos")]
            let item = MenuItem::new(login_menu_text(is_login_enabled()), true, None);
            #[cfg(not(target_os = "macos"))]
            let item = MenuItem::new(login_menu_text(false), false, None);
            let _ = menu.append(&item);
            let _ = port;
            (Some(item.id().clone()), Some(item))
        }
        GateServerMode::Stdio => {
            let item = MenuItem::new("Open at Login (HTTP only)", false, None);
            let _ = menu.append(&item);
            (None, None)
        }
    };

    let quit_item = MenuItem::new("Quit", true, None);
    let _ = menu.append(&quit_item);

    TrayMenuState {
        menu,
        quit_id: quit_item.id().clone(),
        login_id,
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
            eprintln!("Warning: failed to toggle gate launch agent: {err}");
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

fn launch_agent_path() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Library/LaunchAgents")
        .join(format!("{SERVICE_LABEL}.plist"))
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
    std::fs::write(&path, launch_agent_plist(&exe_path, port))
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
fn launch_agent_plist(exe_path: &Path, port: u16) -> String {
    format!(
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
    <false/>
</dict>
</plist>
"#,
        service_label = SERVICE_LABEL,
        exe_path = xml_escape(&exe_path.display().to_string()),
        port = port
    )
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
    #[cfg(target_os = "macos")]
    use std::path::Path;

    #[cfg(target_os = "macos")]
    use super::launch_agent_plist;

    #[cfg(target_os = "macos")]
    #[test]
    fn http_login_item_plist_contains_gate_service_label_and_port() {
        let plist = launch_agent_plist(Path::new("/usr/local/bin/gate"), 8181);
        assert!(plist.contains("com.ma-collective.gate"));
        assert!(plist.contains("--http"));
        assert!(plist.contains("<string>8181</string>"));
    }
}
