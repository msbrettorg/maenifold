//! T-29.2: support helpers for [`crate::tray`] — kept separate to keep `tray.rs`
//! within the per-file LOC budget. No behavioral changes.
//!
//! Contains: icon construction, URL launchers, menu-event URL routing, and
//! the post-exit restart spawner.

use super::tray::ServerMode;
use crate::tray_menu::endophasia_url;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tray_icon::Icon;
use tray_icon::menu::MenuId;

// ── Icon ──────────────────────────────────────────────────────────────

/// Build a 64×64 RGBA icon from the embedded font-rendered glyph.
pub(super) fn build_icon() -> Icon {
    let rgba = include_bytes!("tray_icon.rgba");
    Icon::from_rgba(rgba.to_vec(), 64, 64).expect("valid 64×64 RGBA icon")
}

// ── URL helpers ───────────────────────────────────────────────────────

pub(super) fn open_url(url: &str) -> crate::Result<()> {
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

pub(super) fn resolve_open_url(
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

// ── Server thread ─────────────────────────────────────────────────────

/// Spawn the tokio/axum server on a dedicated background `std::thread`,
/// returning the join handle. Used by `run_with_tray`.
pub(super) fn spawn_server_thread(
    port: u16,
    shutdown_token: CancellationToken,
) -> std::thread::JoinHandle<i32> {
    std::thread::Builder::new()
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
                crate::mcp::MaenifoldMcpServer::serve_http_with_shutdown(port, shutdown_token)
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
        .expect("failed to spawn server thread")
}

// ── Restart on exit ───────────────────────────────────────────────────

/// Spawn a replacement process using the current executable and CLI args.
/// Logs (but does not fail) on errors — caller decides the exit code.
pub(super) fn spawn_restart_process() {
    match std::env::current_exe() {
        Ok(exe) => {
            let args: Vec<String> = std::env::args().skip(1).collect();
            match std::process::Command::new(exe).args(&args).spawn() {
                Ok(child) => {
                    info!(
                        pid = child.id(),
                        ?args,
                        "spawned maenifold tray restart process"
                    );
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
}
