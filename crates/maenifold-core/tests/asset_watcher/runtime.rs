// T-12.7: RTM NFR-1 - shared asset watcher runtime helpers extracted to keep helper modules small.
use super::WatcherSignal;
use notify::{Event, EventKind};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub(super) fn spawn_worker(
    assets_path: PathBuf,
    debounce_ms: u64,
    notifications: Arc<AtomicUsize>,
    event_rx: Receiver<WatcherSignal>,
    batch_tx: Sender<Vec<PathBuf>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let debounce_window = Duration::from_millis(debounce_ms.max(1));
        let flush_grace_window = debounce_grace_window(debounce_window);
        let mut pending_paths = HashSet::new();

        loop {
            match event_rx.recv() {
                Ok(WatcherSignal::Event(event)) => {
                    collect_pending_paths(&assets_path, &event, &mut pending_paths);
                }
                Ok(WatcherSignal::Stop) | Err(_) => {
                    flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                    break;
                }
            }

            loop {
                match event_rx.recv_timeout(debounce_window) {
                    Ok(WatcherSignal::Event(event)) => {
                        collect_pending_paths(&assets_path, &event, &mut pending_paths);
                    }
                    Ok(WatcherSignal::Stop) => {
                        flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        // T-0.1: RTM NFR-3 — mirror production watcher debounce semantics so
                        // delayed poll events from the same burst still produce one notification.
                        match event_rx.recv_timeout(flush_grace_window) {
                            Ok(WatcherSignal::Event(event)) => {
                                collect_pending_paths(&assets_path, &event, &mut pending_paths);
                            }
                            Ok(WatcherSignal::Stop) => {
                                flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                                return;
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                                break;
                            }
                            Err(RecvTimeoutError::Disconnected) => {
                                flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                                return;
                            }
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                        return;
                    }
                }
            }
        }
    })
}

pub(super) fn is_relevant_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

pub(super) fn wait_for(description: &str, timeout: Duration, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }

    panic!("timed out waiting for {description}");
}

fn debounce_grace_window(debounce_window: Duration) -> Duration {
    debounce_window.min(Duration::from_millis(50))
}

fn collect_pending_paths(assets_path: &Path, event: &Event, pending_paths: &mut HashSet<PathBuf>) {
    for path in &event.paths {
        if is_watchable_asset_json_path(assets_path, path) {
            pending_paths.insert(path.clone());
        }
    }
}

fn flush_pending_paths(
    notifications: &AtomicUsize,
    batch_tx: &Sender<Vec<PathBuf>>,
    pending_paths: &mut HashSet<PathBuf>,
) {
    if pending_paths.is_empty() {
        return;
    }

    let mut batch = pending_paths.drain().collect::<Vec<_>>();
    batch.sort();
    notifications.fetch_add(1, Ordering::SeqCst);
    let _ = batch_tx.send(batch);
}

fn is_watchable_asset_json_path(assets_path: &Path, path: &Path) -> bool {
    path.starts_with(assets_path)
        && path.extension().and_then(|ext| ext.to_str()) == Some("json")
        && !is_hidden_relative_path(assets_path, path)
}

fn is_hidden_relative_path(assets_path: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(assets_path) else {
        return true;
    };

    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}
