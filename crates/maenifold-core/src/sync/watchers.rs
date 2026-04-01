use super::watcher_batch::{collect_pending_paths, flush_pending_paths, is_relevant_event_kind};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use notify::{Event, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// T-64.4: RTM NFR-1 re-export WatcherHealth from the dedicated health submodule.
pub use super::watcher_health::WatcherHealth;

static WATCHER: Lazy<Mutex<Option<WatcherState>>> = Lazy::new(|| Mutex::new(None));

struct WatcherState {
    watcher: notify::RecommendedWatcher,
    control_tx: Sender<WatcherSignal>,
    worker: Option<JoinHandle<()>>,
}

impl WatcherState {
    fn stop(mut self) {
        let _ = self.control_tx.send(WatcherSignal::Stop);
        drop(self.watcher);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

enum WatcherSignal {
    Event(Event),
    Stop,
}

pub fn start_watcher(context: &ToolContext, payload: &Value) -> Result<String> {
    let debounce_ms = payload
        .get("debounceMs")
        .and_then(Value::as_u64)
        .or_else(read_debounce_ms_from_env)
        .unwrap_or(150);
    let config = context.config.clone();
    let holder_id = generate_sync_holder_id();

    if WATCHER.lock().unwrap_or_else(|e| e.into_inner()).is_some() {
        return Ok("Watcher already running".to_string());
    }

    let (event_tx, event_rx) = mpsc::channel();
    let worker = spawn_worker(config.clone(), holder_id, debounce_ms, event_rx);
    let callback_tx = event_tx.clone();

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        let Ok(event) = event else {
            if let Err(error) = event {
                eprintln!("{}", format_watcher_error_message(&error.to_string()));
            }
            return;
        };
        if !is_relevant_event_kind(&event.kind) {
            return;
        }
        let _ = callback_tx.send(WatcherSignal::Event(event));
    })
    .map_err(map_notify_error)?;

    if let Err(error) = watcher.watch(&config.memory_path, RecursiveMode::Recursive) {
        let _ = event_tx.send(WatcherSignal::Stop);
        let _ = worker.join();
        return Err(map_notify_error(error));
    }

    WATCHER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .replace(WatcherState {
            watcher,
            control_tx: event_tx,
            worker: Some(worker),
        });

    Ok(format!(
        "Started watching {} for incremental sync (debounce: {}ms)",
        context.config.memory_path.display(),
        debounce_ms,
    ))
}

pub fn stop_watcher(_context: &ToolContext, _payload: &Value) -> Result<String> {
    if let Some(state) = WATCHER.lock().unwrap_or_else(|e| e.into_inner()).take() {
        state.stop();
        return Ok("Stopped incremental sync watcher".to_string());
    }
    Ok("Watcher not running".to_string())
}

// T-COV-001.3: RTM FR-17.5 — expose watcher state so integration tests can verify lifecycle guards.
#[doc(hidden)]
pub fn watcher_is_running() -> bool {
    WATCHER.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

#[doc(hidden)]
pub fn changes_since_last_optimize() -> u32 {
    super::watcher_health::changes_since_last_optimize()
}

pub fn watcher_health() -> WatcherHealth {
    super::watcher_health::snapshot(watcher_is_running())
}

pub fn reset_health_counters() {
    super::watcher_health::reset_health_counters();
}

#[doc(hidden)]
pub fn reset_optimize_counter() {
    super::watcher_health::reset_optimize_counter();
}

#[doc(hidden)]
pub fn process_path_for_test(context: &ToolContext, path: &Path) -> Result<()> {
    super::watcher_batch::process_path_for_test(context, path)
}

#[doc(hidden)]
pub fn process_pending_paths_for_test(context: &ToolContext, paths: &[PathBuf]) -> Result<()> {
    super::watcher_batch::process_pending_paths_for_test(context, paths)
}

#[doc(hidden)]
pub fn format_watcher_error_message(error: &str) -> String {
    super::watcher_batch::format_watcher_error_message(error)
}

fn spawn_worker(
    config: Config,
    holder_id: String,
    debounce_ms: u64,
    event_rx: Receiver<WatcherSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let debounce_window = Duration::from_millis(debounce_ms.max(1));
        let flush_grace_window = debounce_grace_window(debounce_window);
        let mut pending_paths = HashSet::new();

        loop {
            match event_rx.recv() {
                Ok(WatcherSignal::Event(event)) => {
                    collect_pending_paths(&config.memory_path, &event, &mut pending_paths);
                }
                Ok(WatcherSignal::Stop) | Err(_) => {
                    flush_pending_paths(&config, &holder_id, &mut pending_paths);
                    break;
                }
            }

            loop {
                match event_rx.recv_timeout(debounce_window) {
                    Ok(WatcherSignal::Event(event)) => {
                        collect_pending_paths(&config.memory_path, &event, &mut pending_paths);
                    }
                    Ok(WatcherSignal::Stop) => {
                        flush_pending_paths(&config, &holder_id, &mut pending_paths);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => match event_rx
                        .recv_timeout(flush_grace_window)
                    {
                        Ok(WatcherSignal::Event(event)) => {
                            collect_pending_paths(&config.memory_path, &event, &mut pending_paths);
                        }
                        Ok(WatcherSignal::Stop) => {
                            flush_pending_paths(&config, &holder_id, &mut pending_paths);
                            return;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            flush_pending_paths(&config, &holder_id, &mut pending_paths);
                            break;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            flush_pending_paths(&config, &holder_id, &mut pending_paths);
                            return;
                        }
                    },
                    Err(RecvTimeoutError::Disconnected) => {
                        flush_pending_paths(&config, &holder_id, &mut pending_paths);
                        return;
                    }
                }
            }
        }
    })
}

fn debounce_grace_window(debounce_window: Duration) -> Duration {
    debounce_window.min(Duration::from_millis(50))
}

fn read_debounce_ms_from_env() -> Option<u64> {
    std::env::var("MAENIFOLD_DEBOUNCE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn generate_sync_holder_id() -> String {
    format!("watcher-{}-{}", std::process::id(), uuid::Uuid::new_v4())
}

fn map_notify_error(error: notify::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("watcher error: {error}"))
}
