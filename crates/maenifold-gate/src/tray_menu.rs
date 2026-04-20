use std::path::Path;

use maenifold_core::tray_menu::TrayMenuState;
use tray_icon::menu::{Menu, MenuId, MenuItem};

use crate::tray_lifecycle::GateServerMode;

const SERVICE_LABEL: &str = "com.ma-collective.gate";

#[cfg(target_os = "macos")]
fn get_uid() -> u32 {
    // SAFETY: getuid() is always safe — no arguments, no failure mode.
    unsafe { libc::getuid() }
}

pub(crate) struct GateTrayMenuState {
    pub open_ui_id: Option<MenuId>,
    pub open_item: Option<MenuItem>,
    pub login_id: Option<MenuId>,
    pub login_item: Option<MenuItem>,
}

pub(crate) fn build_tray_menu(
    mode: GateServerMode,
    has_root: bool,
) -> TrayMenuState<GateTrayMenuState> {
    let menu = Menu::new();
    let open_item = MenuItem::new("Open Gate UI", open_ui_enabled(has_root), None);
    let _ = menu.append(&open_item);

    let (open_ui_id, login_id, login_item) = match mode {
        GateServerMode::Http(port) => {
            #[cfg(target_os = "macos")]
            let item = MenuItem::new(login_menu_text(is_login_enabled()), true, None);
            #[cfg(not(target_os = "macos"))]
            let item = MenuItem::new(login_menu_text(false), false, None);
            let _ = menu.append(&item);
            let _ = port;
            (
                Some(open_item.id().clone()),
                Some(item.id().clone()),
                Some(item),
            )
        }
        GateServerMode::Stdio => {
            let item = MenuItem::new("Open at Login (HTTP only)", false, None);
            let _ = menu.append(&item);
            (Some(open_item.id().clone()), None, None)
        }
    };

    let quit_item = MenuItem::new("Quit", true, None);
    let _ = menu.append(&quit_item);

    TrayMenuState {
        menu,
        quit_id: quit_item.id().clone(),
        state: GateTrayMenuState {
            open_ui_id,
            open_item: Some(open_item),
            login_id,
            login_item,
        },
    }
}

pub(crate) fn open_ui(root: Option<&Path>) {
    let Some(root) = root else {
        eprintln!(
            "Warning: gate root is not set; pass an explicit root to the gate service before opening the UI"
        );
        return;
    };

    if let Err(err) = spawn_ui_process(root) {
        eprintln!("Warning: failed to open gate UI: {err}");
    }
}

pub(crate) fn set_open_ui_enabled(open_item: Option<&MenuItem>, enabled: bool) {
    if let Some(item) = open_item {
        item.set_enabled(enabled);
    }
}

pub(crate) fn toggle_login_item(port: u16, root: Option<&Path>, login_item: Option<&MenuItem>) {
    #[cfg(target_os = "macos")]
    {
        let result = if is_login_enabled() {
            bootout_launch_agent();
            remove_launch_agent()
        } else {
            let r = write_launch_agent(port, root);
            if r.is_ok() {
                bootstrap_launch_agent();
            }
            r
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
        let _ = (port, root, login_item);
    }
}

pub(crate) fn ui_args(root: &Path) -> Vec<String> {
    vec![
        "ui".to_string(),
        "--root".to_string(),
        root.display().to_string(),
    ]
}

fn spawn_ui_process(root: &Path) -> std::io::Result<()> {
    let exe_path = std::env::current_exe()?;
    let args = ui_args(root);
    std::process::Command::new(exe_path).args(args).spawn()?;
    Ok(())
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

#[cfg(target_os = "macos")]
fn bootstrap_launch_agent() {
    let uid = get_uid();
    let path = launch_agent_path();
    let domain = format!("gui/{uid}");
    match std::process::Command::new("launchctl")
        .args(["bootstrap", &domain, &path.display().to_string()])
        .output()
    {
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
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
    match std::process::Command::new("launchctl")
        .args(["bootout", &target])
        .output()
    {
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("No such process") && !stderr.contains("not loaded") {
                eprintln!("Warning: launchctl bootout failed: {}", stderr.trim());
            }
        }
        Err(err) => eprintln!("Warning: failed to run launchctl bootout: {err}"),
        _ => {}
    }
}

fn login_menu_text(enabled: bool) -> &'static str {
    if enabled {
        "✓ Open at Login"
    } else {
        "Open at Login"
    }
}

#[cfg(target_os = "macos")]
fn write_launch_agent(port: u16, root: Option<&Path>) -> std::io::Result<()> {
    let path = launch_agent_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let exe_path = std::env::current_exe()?;
    std::fs::write(&path, launch_agent_plist(&exe_path, port, root))
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
fn launch_agent_plist(exe_path: &Path, port: u16, root: Option<&Path>) -> String {
    let root_args = launch_agent_root_args(root);
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
{root_args}
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
        port = port,
        root_args = root_args,
    )
}

#[cfg(target_os = "macos")]
fn launch_agent_root_args(root: Option<&Path>) -> String {
    root.map(|root| {
        format!(
            "        <string>--root</string>\n        <string>{}</string>",
            xml_escape(&root.display().to_string())
        )
    })
    .unwrap_or_default()
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
    use std::path::Path;

    #[cfg(target_os = "macos")]
    use super::launch_agent_plist;
    use super::open_ui_enabled;
    use super::ui_args;

    #[test]
    fn ui_args_target_requested_root() {
        let args = ui_args(Path::new("/tmp/example"));
        assert_eq!(args, vec!["ui", "--root", "/tmp/example"]);
    }

    #[test]
    fn open_ui_is_enabled_only_when_root_exists() {
        assert!(open_ui_enabled(true));
        assert!(!open_ui_enabled(false));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn http_login_item_plist_contains_gate_service_label_and_port() {
        let plist = launch_agent_plist(Path::new("/usr/local/bin/gate"), 8181, None);
        assert!(plist.contains("com.ma-collective.gate"));
        assert!(plist.contains("--http"));
        assert!(plist.contains("<string>8181</string>"));
        assert!(!plist.contains("--root"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn http_login_item_plist_preserves_root_when_present() {
        let plist = launch_agent_plist(
            Path::new("/usr/local/bin/gate"),
            8181,
            Some(Path::new("/workspace/project")),
        );
        assert!(plist.contains("<string>--root</string>"));
        assert!(plist.contains("<string>/workspace/project</string>"));
    }
}

pub(crate) fn open_ui_enabled(has_root: bool) -> bool {
    has_root
}
