use crate::config::Config;
use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::markdown::path_to_uri;
use crate::sessions;
use crate::sync::leader::LeaderElection;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

static WATCHER: Lazy<Mutex<Option<WatcherState>>> = Lazy::new(|| Mutex::new(None));

// T-15.21: RTM FR-15 — Accumulated file changes since last FTS optimize.
// Mirrors C# `IncrementalSyncTools._changesSinceOptimize` counter.
static CHANGES_SINCE_OPTIMIZE: AtomicU32 = AtomicU32::new(0);

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
        // T-11.2: RTM FR-15
        .unwrap_or(150);
    let config = context.config.clone();
    let holder_id = generate_sync_holder_id();

    if WATCHER.lock().is_some() {
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

    WATCHER.lock().replace(WatcherState {
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
    if let Some(state) = WATCHER.lock().take() {
        state.stop();
        return Ok("Stopped incremental sync watcher".to_string());
    }
    Ok("Watcher not running".to_string())
}

// T-COV-001.3: RTM FR-17.5 — expose watcher state so integration tests can verify lifecycle guards.
#[doc(hidden)]
pub fn watcher_is_running() -> bool {
    WATCHER.lock().is_some()
}

// T-15.21: RTM FR-15 — expose optimize counter for integration tests to verify FTS maintenance scheduling.
#[doc(hidden)]
pub fn changes_since_last_optimize() -> u32 {
    CHANGES_SINCE_OPTIMIZE.load(Ordering::Relaxed)
}

// T-15.21: RTM FR-15 — reset optimize counter for test isolation.
#[doc(hidden)]
pub fn reset_optimize_counter() {
    CHANGES_SINCE_OPTIMIZE.store(0, Ordering::Relaxed);
}

// T-COV-001.3: RTM FR-17.5 — expose private incremental path processing so integration tests can port C# handler coverage.
#[doc(hidden)]
pub fn process_path_for_test(context: &ToolContext, path: &Path) -> Result<()> {
    if path_to_memory_uri(&context.config, path)?.is_none() {
        return Ok(());
    }
    crate::sync::sync_single_file(context, path)?;
    Ok(())
}

// T-COV-001.3: RTM FR-17.5 — expose debounced batch semantics so integration tests can port watcher coalescing coverage deterministically.
#[doc(hidden)]
pub fn process_pending_paths_for_test(context: &ToolContext, paths: &[PathBuf]) -> Result<()> {
    let mut unique_paths = HashSet::new();
    for path in paths {
        if path_to_memory_uri(&context.config, path)?.is_some() {
            unique_paths.insert(path.clone());
        }
    }

    let paths = unique_paths.into_iter().collect::<Vec<_>>();
    process_incremental_batch(&context.config, &paths)
}

// T-COV-001.3: RTM FR-17.5 — keep watcher error text stable for parity-style assertions in integration tests.
#[doc(hidden)]
pub fn format_watcher_error_message(error: &str) -> String {
    format!("[FileWatcher] Error: {error}")
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
                    Err(RecvTimeoutError::Timeout) => {
                        // T-0.1: RTM NFR-3 — allow one short grace window so late duplicate filesystem
                        // events from the same write burst still collapse into a single debounced batch.
                        match event_rx.recv_timeout(flush_grace_window) {
                            Ok(WatcherSignal::Event(event)) => {
                                collect_pending_paths(
                                    &config.memory_path,
                                    &event,
                                    &mut pending_paths,
                                );
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
                        }
                    }
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

fn collect_pending_paths(memory_path: &Path, event: &Event, pending_paths: &mut HashSet<PathBuf>) {
    for path in &event.paths {
        if !is_watchable_markdown_path(memory_path, path) {
            continue;
        }
        pending_paths.insert(path.clone());
    }
}

fn flush_pending_paths(config: &Config, holder_id: &str, pending_paths: &mut HashSet<PathBuf>) {
    if pending_paths.is_empty() {
        return;
    }

    let paths = pending_paths.drain().collect::<Vec<_>>();
    if let Err(error) = process_pending_paths(config, holder_id, &paths) {
        eprintln!("maenifold watcher incremental sync failed: {error}");
    }
}

fn process_pending_paths(config: &Config, holder_id: &str, paths: &[PathBuf]) -> Result<()> {
    let role = sync_role();
    // T-4.1: RTM FR-30.6
    if role == "follower" {
        eprintln!("[maenifold] watcher: follower role, skipping sync");
        return Ok(());
    }

    let leader_election = LeaderElection::new(&config.database_path);
    // T-5.4: RTM FR-30.4 / FR-30.9
    let (lease_owned, reason, epoch) = if role == "leader" {
        // T-11.2: RTM FR-15
        leader_election.force_acquire(holder_id, config.sync_lease_ms())?
    } else {
        // T-5.4: RTM FR-30.1
        // T-11.2: RTM FR-15
        leader_election.try_acquire(holder_id, config.sync_lease_ms())?
    };
    // T-4.1: RTM FR-30.6
    if !lease_owned {
        eprintln!(
            "[maenifold] watcher: another process is leader, skipping sync (reason: {})",
            reason
        );
        return Ok(());
    }

    GraphDatabase::initialize_database(config)?;
    let result = process_incremental_batch(config, paths);
    // T-8.1: RTM FR-30.7
    let _ = leader_election.release(holder_id, epoch);
    result
}

fn process_incremental_batch(config: &Config, paths: &[PathBuf]) -> Result<()> {
    let mut sorted_paths = paths.to_vec();
    sorted_paths.sort();
    let context = ToolContext::new(config.clone(), OutputFormat::Markdown);

    for path in &sorted_paths {
        match crate::sync::sync_single_file(&context, path) {
            Ok(_) => {}
            Err(error) => {
                // T-5.8: RTM NFR-3, NFR-4 isolates watcher batch failures per file.
                eprintln!(
                    "maenifold watcher failed to process {}: {error}",
                    path.display()
                );
            }
        }
    }

    let _communities_detected = crate::sync::recompute_communities_if_needed(&context)?;
    let _abandoned_sessions = sessions::cleanup_abandoned_sessions(config)?;

    // T-15.21: RTM FR-15 — schedule FTS optimize after accumulated changes reach threshold.
    // Mirrors C# IncrementalSyncTools.ScheduleMaintenanceIfNeeded() which runs
    // DatabaseMaintenance.RunLight() (FTS optimize + passive WAL checkpoint) after
    // Config.IncrementalOptimizeEvery accumulated file changes.
    schedule_maintenance_if_needed(config, sorted_paths.len() as u32);

    Ok(())
}

// T-15.21: RTM FR-15 — FTS optimize scheduling for incremental sync.
// Accumulates processed file count and triggers DatabaseMaintenance.RunLight()
// (FTS optimize + passive WAL checkpoint) when the threshold is reached.
// Mirrors C# IncrementalSyncTools.ScheduleMaintenanceIfNeeded().
fn schedule_maintenance_if_needed(config: &Config, batch_size: u32) {
    if batch_size == 0 {
        return;
    }

    let previous = CHANGES_SINCE_OPTIMIZE.fetch_add(batch_size, Ordering::Relaxed);
    let accumulated = previous + batch_size;
    let threshold = config.incremental_optimize_every();

    if accumulated >= threshold {
        CHANGES_SINCE_OPTIMIZE.store(0, Ordering::Relaxed);
        match db::open_connection(&config.database_path) {
            Ok(conn) => {
                if let Err(error) = db::run_light_maintenance(&conn) {
                    eprintln!("[maenifold] incremental maintenance failed: {error}");
                }
            }
            Err(error) => {
                eprintln!("[maenifold] incremental maintenance failed: {error}");
            }
        }
    }
}

fn path_to_memory_uri(config: &Config, path: &Path) -> Result<Option<String>> {
    if !path.starts_with(&config.memory_path) {
        return Ok(None);
    }
    if !path.exists() && path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Ok(None);
    }
    if !path.exists() && is_hidden_relative_path(&config.memory_path, path) {
        return Ok(None);
    }
    if path.exists() && !is_watchable_markdown_path(&config.memory_path, path) {
        return Ok(None);
    }
    path_to_uri(path, &config.memory_path).map(Some)
}

fn is_relevant_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn is_watchable_markdown_path(memory_path: &Path, path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return false;
    }
    if !path.starts_with(memory_path) {
        return false;
    }
    !is_hidden_relative_path(memory_path, path)
}

fn is_hidden_relative_path(memory_path: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(memory_path) else {
        return true;
    };
    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}

fn read_debounce_ms_from_env() -> Option<u64> {
    std::env::var("MAENIFOLD_DEBOUNCE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn sync_role() -> String {
    match std::env::var("MAENIFOLD_SYNC_ROLE") {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "leader" => "leader".to_string(),
            "follower" => "follower".to_string(),
            _ => "auto".to_string(),
        },
        Err(_) => "auto".to_string(),
    }
}

fn generate_sync_holder_id() -> String {
    format!("watcher-{}-{}", std::process::id(), uuid::Uuid::new_v4())
}

fn map_notify_error(error: notify::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("watcher error: {error}"))
}
