use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
#[cfg(not(target_os = "macos"))]
use tracing::error;
use tray_icon::Icon;

#[cfg(target_os = "macos")]
use winit::event_loop::EventLoopProxy;

#[cfg(not(target_os = "macos"))]
use crate::file_watcher::GateFileWatcher;
use crate::mcp::{GateMcpServer, KnownRoots};

const ROOT_STATE_POLL_MS: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateServerMode {
    Stdio,
    Http(u16),
}

impl GateServerMode {
    pub fn port(self) -> Option<u16> {
        match self {
            Self::Stdio => None,
            Self::Http(port) => Some(port),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayUserEvent {
    RootAvailabilityChanged(bool),
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn run_headless(mode: GateServerMode, initial_root: Option<PathBuf>) -> i32 {
    let root = Arc::new(RwLock::new(initial_root.clone()));
    let shutdown_token = CancellationToken::new();
    let _file_watcher = match GateFileWatcher::start(root.clone(), &shutdown_token) {
        Ok(watcher) => watcher,
        Err(err) => {
            error!(error = %err, "failed to start gate file watcher");
            return 1;
        }
    };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            error!(error = %err, "failed to create async runtime");
            return 1;
        }
    };
    let known_roots = KnownRoots::with_initial(initial_root);
    let result = rt.block_on(run_mode(mode, known_roots));
    shutdown_token.cancel();
    rt.shutdown_timeout(Duration::from_secs(5));
    match result {
        Ok(()) => 0,
        Err(err) => {
            error!(error = %err, "gate headless server exited with error");
            1
        }
    }
}

pub(crate) async fn run_mode(
    mode: GateServerMode,
    known_roots: KnownRoots,
) -> Result<(), crate::error::GateError> {
    match mode {
        GateServerMode::Stdio => GateMcpServer::serve_stdio(known_roots).await,
        GateServerMode::Http(port) => GateMcpServer::serve_http(port, known_roots).await,
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn spawn_root_monitor(
    root: Arc<RwLock<Option<PathBuf>>>,
    shutdown_token: CancellationToken,
    proxy: EventLoopProxy<TrayUserEvent>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("gate-tray-root-monitor".into())
        .spawn(move || {
            let mut last_has_root = root
                .read()
                .expect("tray root monitor root: lock poisoned")
                .is_some();
            while !shutdown_token.is_cancelled() {
                std::thread::sleep(Duration::from_millis(ROOT_STATE_POLL_MS));
                let has_root = root
                    .read()
                    .expect("tray root monitor root: lock poisoned")
                    .is_some();
                if has_root != last_has_root {
                    last_has_root = has_root;
                    let _ = proxy.send_event(TrayUserEvent::RootAvailabilityChanged(has_root));
                }
            }
        })
        .expect("failed to spawn gate tray root monitor")
}

pub(crate) fn build_icon() -> Icon {
    let rgba = include_bytes!("tray_icon.rgba");
    Icon::from_rgba(rgba.to_vec(), 64, 64).expect("valid 64x64 RGBA tray icon")
}

pub(crate) fn tooltip_for_mode(mode: GateServerMode) -> String {
    match mode {
        GateServerMode::Stdio => "gate MCP stdio".to_string(),
        GateServerMode::Http(port) => format!("gate MCP :{port}"),
    }
}
