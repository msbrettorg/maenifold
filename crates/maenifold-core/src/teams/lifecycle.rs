//! Append-only JSONL lifecycle journal for the Teams monitor supervisor.
//!
//! Port of `_log_lifecycle()` and `_rotate_lifecycle()` from agency-cowork
//! `skills/teams/scripts/monitor/service.py` L55-L94 (`_LIFECYCLE_MAX_ENTRIES = 500`).
//!
//! Each entry is a single JSON object on its own line:
//!     {"ts":"2026-...Z","event":"connected","detail":"..."}
//!
//! Best-effort: every IO error is logged via `tracing::warn!` and swallowed so
//! that lifecycle logging never crashes the supervisor.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde_json::json;

/// Maximum entries retained after rotation. Mirrors agency-cowork
/// `_LIFECYCLE_MAX_ENTRIES` (service.py L40-ish).
pub const LIFECYCLE_MAX_ENTRIES: usize = 500;

struct LifecycleJournal {
    path: PathBuf,
    write_lock: Mutex<()>,
}

static JOURNAL: OnceLock<LifecycleJournal> = OnceLock::new();

/// Initialize the journal at `<maenifold_root>/logs/monitor-lifecycle.jsonl`.
/// Idempotent — first call wins (subsequent calls are no-ops).
pub fn init_lifecycle_journal(maenifold_root: &Path) {
    let path = maenifold_root.join("logs").join("monitor-lifecycle.jsonl");
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            tracing::warn!(
                target: "teams::lifecycle",
                %error,
                path = %parent.display(),
                "failed to create lifecycle journal directory"
            );
            return;
        }
    }
    let _ = JOURNAL.set(LifecycleJournal {
        path,
        write_lock: Mutex::new(()),
    });
}

/// Append a lifecycle event. No-op (with `tracing::trace!`) if not initialized.
pub fn log_lifecycle(event: &str, detail: Option<&str>) {
    let Some(journal) = JOURNAL.get() else {
        tracing::trace!(target: "teams::lifecycle", event, "journal not initialized");
        return;
    };

    let mut entry = json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "event": event,
    });
    if let Some(detail) = detail {
        entry
            .as_object_mut()
            .expect("entry is an object")
            .insert("detail".to_string(), json!(detail));
    }
    let line = entry.to_string();

    let _guard = match journal.write_lock.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&journal.path)
    {
        Ok(mut file) => {
            if let Err(error) = writeln!(file, "{line}") {
                tracing::warn!(
                    target: "teams::lifecycle",
                    %error,
                    path = %journal.path.display(),
                    "failed to append lifecycle entry"
                );
                return;
            }
        }
        Err(error) => {
            tracing::warn!(
                target: "teams::lifecycle",
                %error,
                path = %journal.path.display(),
                "failed to open lifecycle journal"
            );
            return;
        }
    }

    rotate_if_needed(&journal.path);
}

/// Keep only the last `LIFECYCLE_MAX_ENTRIES` lines via atomic rename.
/// Mirrors `_rotate_lifecycle()` (service.py L80-L94).
fn rotate_if_needed(path: &Path) {
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(error) => {
            tracing::warn!(
                target: "teams::lifecycle",
                %error,
                path = %path.display(),
                "failed to read journal for rotation"
            );
            return;
        }
    };
    let lines: Vec<&str> = contents.lines().collect();
    if lines.len() <= LIFECYCLE_MAX_ENTRIES {
        return;
    }
    let kept = &lines[lines.len() - LIFECYCLE_MAX_ENTRIES..];
    let mut next = String::with_capacity(contents.len());
    for line in kept {
        next.push_str(line);
        next.push('\n');
    }
    let tmp = path.with_extension("tmp");
    if let Err(error) = fs::write(&tmp, next) {
        tracing::warn!(
            target: "teams::lifecycle",
            %error,
            path = %tmp.display(),
            "failed to write rotated journal tmp"
        );
        return;
    }
    if let Err(error) = fs::rename(&tmp, path) {
        tracing::warn!(
            target: "teams::lifecycle",
            %error,
            path = %path.display(),
            "failed to rename rotated journal"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_lines(path: &Path) -> Vec<String> {
        fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn log_before_init_is_noop() {
        // Must not panic.
        log_lifecycle("uninitialized", None);
    }

    #[test]
    fn rotation_trims_to_max_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("monitor-lifecycle.jsonl");
        // Pre-populate with > LIFECYCLE_MAX_ENTRIES lines.
        let mut seed = String::new();
        for i in 0..(LIFECYCLE_MAX_ENTRIES + 50) {
            seed.push_str(&format!("{{\"n\":{i}}}\n"));
        }
        fs::write(&path, seed).unwrap();
        rotate_if_needed(&path);
        let lines = read_lines(&path);
        assert_eq!(lines.len(), LIFECYCLE_MAX_ENTRIES);
        assert_eq!(lines[0], "{\"n\":50}");
        assert_eq!(
            lines[LIFECYCLE_MAX_ENTRIES - 1],
            format!("{{\"n\":{}}}", LIFECYCLE_MAX_ENTRIES + 49)
        );
    }
}
