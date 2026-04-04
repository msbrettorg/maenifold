use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio_util::sync::CancellationToken;

use crate::cli::{GateContext, SyncArgs};
use crate::error::GateError;
use crate::sync_command::sync_gate;

const DEFAULT_DEBOUNCE_MS: u64 = 150;
const ROOT_POLL_MS: u64 = 100;

#[derive(Debug)]
pub struct GateFileWatcher {
    shutdown_token: CancellationToken,
    worker: Option<JoinHandle<()>>,
}

impl GateFileWatcher {
    pub fn start(
        root: Arc<RwLock<Option<PathBuf>>>,
        parent_shutdown: &CancellationToken,
    ) -> Result<Self, GateError> {
        let shutdown_token = parent_shutdown.child_token();
        let worker_shutdown = shutdown_token.clone();
        let worker = thread::Builder::new()
            .name("gate-file-watcher".into())
            .spawn(move || {
                if let Err(err) = watch_loop(root, worker_shutdown) {
                    eprintln!("Warning: gate file watcher stopped: {err}");
                }
            })
            .map_err(|err| GateError::Cli(format!("failed to spawn gate file watcher: {err}")))?;

        Ok(Self {
            shutdown_token,
            worker: Some(worker),
        })
    }

    pub fn handle_debounced_change(root: &Path) -> Result<(), GateError> {
        sync_gate(SyncArgs {
            context: GateContext {
                root: root.to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            rebuild: false,
        })
    }
}

impl Drop for GateFileWatcher {
    fn drop(&mut self) {
        self.shutdown_token.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn watch_loop(
    root: Arc<RwLock<Option<PathBuf>>>,
    shutdown_token: CancellationToken,
) -> Result<(), GateError> {
    let debounce_window = Duration::from_millis(read_debounce_ms());
    let poll_window = Duration::from_millis(ROOT_POLL_MS);
    let (event_tx, event_rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = event_tx.send(event);
    })
    .map_err(map_notify_error)?;

    let mut watched_root: Option<PathBuf> = None;
    let mut pending_root: Option<PathBuf> = None;
    let mut last_event_at: Option<Instant> = None;

    loop {
        if shutdown_token.is_cancelled() {
            flush_pending_sync(&mut pending_root, &mut last_event_at);
            break;
        }

        let current_root = root.read().unwrap().clone();
        if watched_root != current_root {
            let previous_root = watched_root.clone();
            if pending_root
                .as_ref()
                .zip(previous_root.as_ref())
                .is_some_and(|(pending, previous)| pending == previous)
            {
                flush_pending_sync(&mut pending_root, &mut last_event_at);
            }
            update_watches(
                &mut watcher,
                previous_root.as_deref(),
                current_root.as_deref(),
            )?;
            watched_root = current_root;
        }

        match event_rx.recv_timeout(poll_window) {
            Ok(Ok(event)) => {
                if is_relevant_event_kind(&event.kind) {
                    pending_root = watched_root.clone();
                    last_event_at = Some(Instant::now());
                }
            }
            Ok(Err(error)) => {
                eprintln!("Warning: gate file watcher error: {error}");
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if last_event_at.is_some_and(|timestamp| timestamp.elapsed() >= debounce_window) {
            flush_pending_sync(&mut pending_root, &mut last_event_at);
        }
    }

    Ok(())
}

fn flush_pending_sync(pending_root: &mut Option<PathBuf>, last_event_at: &mut Option<Instant>) {
    if let Some(root) = pending_root.take()
        && let Err(err) = GateFileWatcher::handle_debounced_change(&root)
    {
        eprintln!("Warning: failed to reconcile gate DB after file change: {err}");
    }
    *last_event_at = None;
}

fn update_watches(
    watcher: &mut RecommendedWatcher,
    previous_root: Option<&Path>,
    next_root: Option<&Path>,
) -> Result<(), GateError> {
    if let Some(root) = previous_root {
        for path in watch_paths(root) {
            let _ = watcher.unwatch(&path);
        }
    }

    if let Some(root) = next_root {
        for path in watch_paths(root) {
            if path.exists() {
                watcher
                    .watch(&path, RecursiveMode::NonRecursive)
                    .map_err(map_notify_error)?;
            }
        }
    }

    Ok(())
}

fn watch_paths(root: &Path) -> [PathBuf; 2] {
    [root.join("todo"), root.join("done")]
}

fn is_relevant_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn read_debounce_ms() -> u64 {
    std::env::var("MAENIFOLD_GATE_DEBOUNCE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_DEBOUNCE_MS)
}

fn map_notify_error(error: notify::Error) -> GateError {
    GateError::Cli(format!("gate file watcher error: {error}"))
}
