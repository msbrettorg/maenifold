use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuSpec {
    pub label: &'static str,
    pub enabled: bool,
}

pub fn launch_agent_path_from(home: Option<&Path>) -> PathBuf {
    home.unwrap_or_else(|| Path::new("."))
        .join("Library/LaunchAgents/com.ma-collective.gate.plist")
}

pub fn login_menu_text(enabled: bool) -> &'static str {
    if enabled {
        "✓ Open at Login"
    } else {
        "Open at Login"
    }
}

pub fn tray_menu_specs(login_enabled: bool) -> Vec<TrayMenuSpec> {
    let mut items = vec![
        TrayMenuSpec {
            label: "Kanban Board",
            enabled: true,
        },
        TrayMenuSpec {
            label: "Trace",
            enabled: true,
        },
        TrayMenuSpec {
            label: login_menu_text(login_enabled),
            enabled: cfg!(target_os = "macos"),
        },
    ];
    if cfg!(target_os = "macos") {
        items.push(TrayMenuSpec {
            label: "Restart",
            enabled: true,
        });
    }
    items.push(TrayMenuSpec {
        label: "Quit",
        enabled: true,
    });
    items
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

pub(super) fn restart_daemon(service_label: &str) {
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

#[cfg(target_os = "macos")]
pub(super) fn write_launch_agent(port: u16) -> std::io::Result<()> {
    let path = launch_agent_path_from(std::env::var_os("HOME").as_deref().map(Path::new));
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
        port = port,
    );
    std::fs::write(path, plist)
}

#[cfg(target_os = "macos")]
pub(super) fn remove_launch_agent() -> std::io::Result<()> {
    match std::fs::remove_file(launch_agent_path_from(
        std::env::var_os("HOME").as_deref().map(Path::new),
    )) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}
