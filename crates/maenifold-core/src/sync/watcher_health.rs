use crate::config::Config;
use crate::db;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

// T-64.4: RTM NFR-1 — watcher health counters extracted from watcher_batch into a dedicated module.
// Mirrors C# `IncrementalSyncTools._changesSinceOptimize` counter.
static CHANGES_SINCE_OPTIMIZE: AtomicU32 = AtomicU32::new(0);
static SYNC_FAILURES: AtomicU32 = AtomicU32::new(0);
static CONSECUTIVE_FAILURES: AtomicU32 = AtomicU32::new(0);
static LAST_FAILURE_MSG: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub struct WatcherHealth {
    pub is_running: bool,
    pub total_failures: u32,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub changes_processed: u32,
}

pub fn changes_since_last_optimize() -> u32 {
    CHANGES_SINCE_OPTIMIZE.load(Ordering::Relaxed)
}

pub fn reset_health_counters() {
    SYNC_FAILURES.store(0, Ordering::Relaxed);
    CONSECUTIVE_FAILURES.store(0, Ordering::Relaxed);
    *LAST_FAILURE_MSG.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub fn reset_optimize_counter() {
    CHANGES_SINCE_OPTIMIZE.store(0, Ordering::Relaxed);
}

pub(crate) fn clear_consecutive_failures() {
    CONSECUTIVE_FAILURES.store(0, Ordering::Relaxed);
}

pub(crate) fn record_batch_failure(message: String) {
    SYNC_FAILURES.fetch_add(1, Ordering::Relaxed);
    CONSECUTIVE_FAILURES.fetch_add(1, Ordering::Relaxed);
    *LAST_FAILURE_MSG.lock().unwrap_or_else(|e| e.into_inner()) = Some(message);
}

pub(crate) fn record_file_failure(message: String) {
    record_batch_failure(message);
}

pub(crate) fn schedule_maintenance_if_needed(config: &Config, batch_size: u32) {
    if batch_size == 0 {
        return;
    }

    let previous = CHANGES_SINCE_OPTIMIZE.fetch_add(batch_size, Ordering::Relaxed);
    let accumulated = previous + batch_size;
    if accumulated < config.incremental_optimize_every() {
        return;
    }

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

pub(crate) fn snapshot(is_running: bool) -> WatcherHealth {
    WatcherHealth {
        is_running,
        total_failures: SYNC_FAILURES.load(Ordering::Relaxed),
        consecutive_failures: CONSECUTIVE_FAILURES.load(Ordering::Relaxed),
        last_error: LAST_FAILURE_MSG
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone(),
        changes_processed: CHANGES_SINCE_OPTIMIZE.load(Ordering::Relaxed),
    }
}
