//! T-29.2: support helpers for [`crate::tray`] — kept separate to keep `tray.rs`
//! within the per-file LOC budget. No behavioral changes.
//!
//! Contains: icon construction, URL launchers, menu-event URL routing, and
//! the post-exit restart spawner.

use super::tray::ServerMode;
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
    help_id: Option<&MenuId>,
    _mode: ServerMode,
) -> Option<String> {
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
                crate::mcp::MaenifoldMcpServer::serve_http_with_shutdown(port, shutdown_token).await
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

/// Restart the tray's hosting process.
///
/// Two paths, picked at runtime:
///
/// 1. **Launchd-managed** (`is_launchd_managed()` is true): hand control to
///    `launchctl kickstart -k gui/<uid>/com.ma-collective.maenifold`. launchd
///    will SIGTERM the current process, wait for it to exit, and start a
///    fresh instance from the LaunchAgent plist. This is the only path that
///    survives our parent's clean (exit-code 0) shutdown — the
///    `KeepAlive { SuccessfulExit: false }` plist policy would otherwise leave
///    the service inactive after the user clicks Restart.
///
/// 2. **Standalone** (developer running `maenifold --http` from a shell):
///    fork an independent grandchild via `setsid` so the new process detaches
///    from our process group. Without `setsid` macOS sends SIGHUP to the
///    parent's process group on session shutdown, which kills the orphan and
///    is exactly what happened to PID 93066 at 15:32:25 today — the spawned
///    child ran for ~17 seconds, then died silently when the launchd job's
///    process group was torn down.
pub(super) fn spawn_restart_process() {
    if crate::tray_menu::is_launchd_managed() {
        let target = crate::tray_menu::launchd_service_target();
        match std::process::Command::new("launchctl")
            .args(["kickstart", "-k", &target])
            .spawn()
        {
            Ok(child) => {
                info!(
                    pid = child.id(),
                    target,
                    "launchctl kickstart issued for maenifold tray restart"
                );
                return;
            }
            Err(err) => {
                warn!(error = %err, target, "launchctl kickstart failed; falling back to direct respawn");
            }
        }
    }

    match std::env::current_exe() {
        Ok(exe) => {
            let args: Vec<String> = std::env::args().skip(1).collect();
            let mut command = std::process::Command::new(&exe);
            command.args(&args);
            detach_from_parent_session(&mut command);
            match command.spawn() {
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

/// Detach the spawned child from the parent's process group / controlling
/// session via `setsid(2)`. On macOS this is the safe default: when the
/// launchd-managed parent exits, the kernel sends signals to the parent's
/// process group, and any child still in that group dies with it.
#[cfg(unix)]
fn detach_from_parent_session(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: `pre_exec` runs after fork() and before exec() in the child.
    // setsid() is async-signal-safe and only fails if the caller is already a
    // process group leader — which we cannot be here because we forked from
    // the launchd-managed parent. Returning `Ok(())` regardless preserves the
    // historical "best-effort respawn" behaviour even if the libc call errors.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn detach_from_parent_session(_command: &mut std::process::Command) {
    // Windows fallback: parent/child process trees are not subject to the
    // POSIX process-group SIGHUP-on-exit behaviour, so no extra detach work
    // is needed for the standalone path.
}
