//! Persistent dispatch store for the Teams supervisor.
//!
//! Rust port of the Python [`DispatchStore`] used by agency-cowork
//! (`skills/teams/scripts/monitor/dispatch_store.py`, 244 LOC). Records track
//! Teams dispatches across the lifecycle
//! `pending → in_progress → done|failed|timeout|cancelled`, persisted to a
//! single JSON file via atomic temp-file + rename so the queue survives
//! supervisor crashes and restarts.
//!
//! On-disk schema is byte-compatible with the Python implementation
//! (snake_case status strings, `f64` epoch-seconds timestamps, `0.0`
//! sentinels for unset times) so a phased rollout can share the same
//! `monitor/dispatch-queue.json` file.
//!
//! No internal locking — the supervisor wraps this in
//! `Arc<tokio::sync::Mutex<DispatchStore>>` (see sub-dispatch B). Mirrors
//! the Python "single-threaded asyncio" contract (py L67).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MaenifoldError;

const SCHEMA_VERSION: u32 = 1;

/// Lifecycle states for a [`DispatchRecord`]. Mirrors Python module-level
/// constants `PENDING / IN_PROGRESS / DONE / FAILED / TIMEOUT / CANCELLED`
/// (py L17–L22). `#[serde(rename_all = "snake_case")]` keeps the on-disk
/// strings identical to the Python writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchStatus {
    Pending,
    InProgress,
    Done,
    Failed,
    Timeout,
    Cancelled,
}

impl DispatchStatus {
    /// Terminal states (py L24, `_TERMINAL_STATUSES`).
    fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Done | Self::Failed | Self::Timeout | Self::Cancelled
        )
    }
}

/// A single dispatch tracked across its lifecycle. Field order, names, and
/// defaults mirror the Python `@dataclass DispatchRecord` (py L29–L42) so
/// the JSON wire format is interchangeable. `started_at` / `completed_at`
/// use the Python `0.0` sentinel rather than `Option` to preserve schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRecord {
    pub id: String,
    pub prompt: String,
    pub sender_name: String,
    #[serde(default)]
    pub sender_mri: String,
    pub conversation_id: String,
    #[serde(default)]
    pub session_key: String,
    pub status: DispatchStatus,
    pub enqueued_at: f64,
    #[serde(default)]
    pub started_at: f64,
    #[serde(default)]
    pub completed_at: f64,
    #[serde(default)]
    pub result_summary: String,
    #[serde(default)]
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)] // schema doc; serialization uses an inline json! envelope
struct OnDisk {
    version: u32,
    dispatches: Vec<DispatchRecord>,
}

/// JSON-backed dispatch persistence. Port of Python `DispatchStore`
/// (py L64–L238). Records live at `path` and are flushed on every mutating
/// op via [`Self::save`] (atomic temp-file + rename, py L113–L128).
#[derive(Debug)]
pub struct DispatchStore {
    path: PathBuf,
    records: HashMap<String, DispatchRecord>,
}

impl DispatchStore {
    /// Open the store at `path`, replaying recovery (py L70–L92 constructor
    /// + L225–L238 `mark_interrupted_as_pending`). A missing file yields an
    /// empty store; a corrupt file logs a warning and starts empty so a
    /// bad on-disk record cannot brick the supervisor (py L107–L110).
    pub fn open(path: PathBuf) -> Result<Self, MaenifoldError> {
        let records = Self::load(&path);
        let mut store = Self { path, records };
        store.mark_interrupted_as_pending()?;
        Ok(store)
    }

    /// Defensive load (py `_load`, L94–L110). Unknown status strings or
    /// malformed records are skipped individually; a top-level parse error
    /// drops to an empty store with a `tracing::warn!`.
    fn load(path: &Path) -> HashMap<String, DispatchRecord> {
        if !path.exists() {
            return HashMap::new();
        }
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(err) => {
                tracing::warn!(?path, %err, "failed to read dispatch store; starting empty");
                return HashMap::new();
            }
        };
        // Per-record tolerance: deserialize the envelope as raw values, then
        // try each record individually and skip the bad ones (py L100–L107).
        let envelope: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(?path, %err, "dispatch store JSON corrupt; starting empty");
                return HashMap::new();
            }
        };
        let mut out = HashMap::new();
        if let Some(arr) = envelope.get("dispatches").and_then(|v| v.as_array()) {
            for raw in arr {
                match serde_json::from_value::<DispatchRecord>(raw.clone()) {
                    Ok(rec) => {
                        out.insert(rec.id.clone(), rec);
                    }
                    Err(err) => {
                        let id = raw.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        tracing::warn!(id, %err, "skipped malformed dispatch record");
                    }
                }
            }
        }
        tracing::info!(count = out.len(), ?path, "loaded dispatch records");
        out
    }

    /// Atomic write (py `_save`, L113–L128). Writes `<path>.tmp` and renames
    /// over `path` so a crash mid-write leaves the previous good file
    /// intact. Creates parent dirs on first write.
    fn save(&self) -> Result<(), MaenifoldError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut dispatches: Vec<&DispatchRecord> = self.records.values().collect();
        dispatches.sort_by(|a, b| a.id.cmp(&b.id)); // byte-stable diffs
        let envelope = serde_json::json!({
            "version": SCHEMA_VERSION,
            "dispatches": dispatches,
        });
        let body = serde_json::to_string_pretty(&envelope)?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, body)?;
        // Atomic replace: on Unix `rename` overwrites the destination in
        // a single syscall, so the previous good file is never absent
        // (unlike a remove-then-rename, which loses data on crash).
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            MaenifoldError::from(e)
        })?;
        Ok(())
    }

    /// Construct an in-memory store bound to `path` without performing any
    /// file I/O. Used as a last-resort fallback when `open()` fails so the
    /// supervisor can keep running; subsequent `save()` calls will surface
    /// real I/O errors per-call instead of panicking at construction.
    pub(crate) fn empty(path: PathBuf) -> Self {
        Self {
            path,
            records: HashMap::new(),
        }
    }

    /// Add a new dispatch as `Pending`, returning its 12-char lowercase hex
    /// ID (py `add`, L132–L153 — `uuid.uuid4().hex[:12]`).
    pub fn add(
        &mut self,
        prompt: String,
        sender_name: String,
        sender_mri: String,
        conversation_id: String,
        session_key: String,
    ) -> Result<String, MaenifoldError> {
        let id: String = Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(12)
            .collect();
        let rec = DispatchRecord {
            id: id.clone(),
            prompt,
            sender_name,
            sender_mri,
            conversation_id,
            session_key,
            status: DispatchStatus::Pending,
            enqueued_at: now_epoch(),
            started_at: 0.0,
            completed_at: 0.0,
            result_summary: String::new(),
            error: String::new(),
        };
        self.records.insert(id.clone(), rec);
        self.save()?;
        Ok(id)
    }

    /// Update lifecycle state; sets `started_at` on `InProgress` and
    /// `completed_at` on any terminal state (py `update_status`,
    /// L155–L178). Empty `result_summary` / `error` leave the prior value
    /// intact, matching the Python truthy guard.
    pub fn update_status(
        &mut self,
        id: &str,
        status: DispatchStatus,
        result_summary: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), MaenifoldError> {
        let Some(rec) = self.records.get_mut(id) else {
            tracing::warn!(id, "dispatch not found in store");
            return Ok(());
        };
        rec.status = status;
        if status == DispatchStatus::InProgress {
            rec.started_at = now_epoch();
        }
        if status.is_terminal() {
            rec.completed_at = now_epoch();
        }
        if let Some(s) = result_summary.filter(|s| !s.is_empty()) {
            rec.result_summary = s.to_string();
        }
        if let Some(e) = error.filter(|e| !e.is_empty()) {
            rec.error = e.to_string();
        }
        self.save()
    }

    /// Crash recovery: flip stale `InProgress` rows back to `Pending` so a
    /// supervisor restart re-dispatches them (py
    /// `mark_interrupted_as_pending`, L225–L238). Returns the count reset.
    pub fn mark_interrupted_as_pending(&mut self) -> Result<usize, MaenifoldError> {
        let mut count = 0usize;
        for rec in self.records.values_mut() {
            if rec.status == DispatchStatus::InProgress {
                rec.status = DispatchStatus::Pending;
                rec.started_at = 0.0;
                count += 1;
            }
        }
        if count > 0 {
            self.save()?;
            tracing::info!(count, "reset in-progress dispatches back to pending");
        }
        Ok(count)
    }

    /// Read-only iterator over all records, in arbitrary order.
    /// Used by the command executor to format dispatch listings.
    pub(crate) fn records(&self) -> impl Iterator<Item = &DispatchRecord> {
        self.records.values()
    }

    /// Drop terminal records older than `max_age_hours` (py `cleanup_old`,
    /// L211–L223). Pending / in-progress rows are preserved regardless of
    /// age. Returns the count removed.
    pub fn cleanup_old(&mut self, max_age_hours: f64) -> Result<usize, MaenifoldError> {
        let cutoff = now_epoch() - max_age_hours * 3600.0;
        let before = self.records.len();
        self.records
            .retain(|_, r| !(r.status.is_terminal() && r.completed_at < cutoff));
        let removed = before - self.records.len();
        if removed > 0 {
            self.save()?;
            tracing::info!(count = removed, "pruned stale dispatch records");
        }
        Ok(removed)
    }
}

/// Wall-clock seconds since the Unix epoch as `f64`, matching Python's
/// `time.time()` (py L142, used throughout for timestamps).
fn now_epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
