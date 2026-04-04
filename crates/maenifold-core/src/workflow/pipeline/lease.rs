// T-93.4: OS-level advisory file locking for pipeline session ownership.
//
// Uses `fd-lock` (Cargo's flock.rs pattern) for RAII-guarded exclusive locks.
// Lock files live alongside the pipeline session markdown file:
//   {memory_path}/pipelines/{year}/{month}/{day}/{session_id}.lock
//
// Lock release on process death is handled by the OS — no TTL or heartbeat needed.
use crate::config::Config;
use std::fs::OpenOptions;
use std::path::PathBuf;

/// Returns the filesystem path for the advisory lock file of a pipeline session.
///
/// The lock file sits next to the pipeline session markdown file:
/// `{memory_path}/pipelines/{year}/{month}/{day}/{session_id}.lock`
pub fn pipeline_lock_path(session_id: &str, config: &Config) -> PathBuf {
    let uri = crate::sessions::pipeline_session_uri(session_id);
    // uri_to_path appends ".md"; we then switch to ".lock".
    match crate::markdown::uri_to_path(&uri, &config.memory_path) {
        Ok(mut path) => {
            path.set_extension("lock");
            path
        }
        Err(_) => {
            // Fallback: construct path directly from date parts.
            let (year, month, day) = crate::sessions::date_parts_from_session_id(session_id);
            config
                .memory_path
                .join("pipelines")
                .join(format!("{year:04}"))
                .join(format!("{month:02}"))
                .join(format!("{day:02}"))
                .join(format!("{session_id}.lock"))
        }
    }
}

/// Returns `true` if another process currently holds the exclusive lock on this session.
///
/// Uses `fd_lock::RwLock::try_write()`: if the acquire fails (WouldBlock), the lock is held.
/// If the lock file does not exist, the session has never been driven and is not running.
///
/// This is the drop-in replacement for `supervisor_is_running` / `supervisor_status` in
/// contexts where an external driver (not an in-process supervisor) owns the session.
pub(super) fn session_lock_is_held(session_id: &str, config: &Config) -> bool {
    let lock_path = pipeline_lock_path(session_id, config);
    if !lock_path.exists() {
        return false;
    }
    match OpenOptions::new().write(true).open(&lock_path) {
        Ok(file) => {
            let mut rw = fd_lock::RwLock::new(file);
            // If try_write succeeds we immediately drop the guard, releasing the lock.
            rw.try_write().is_err()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn test_config(dir: &std::path::Path) -> Config {
        let memory = dir.join("memory");
        std::fs::create_dir_all(&memory).unwrap();
        Config {
            maenifold_root: dir.to_path_buf(),
            memory_path: memory.clone(),
            assets_path: dir.join("assets"),
            thinking_path: memory.join("thinking"),
            sequential_path: memory.join("thinking").join("sequential"),
            database_path: dir.join("memory.db"),
        }
    }

    // view_pipeline_reports_running_when_lock_is_held
    // rewind_pipeline_rejects_when_lock_is_held
    #[test]
    fn session_lock_is_held_false_when_no_lock_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        // Fake session id that parses to today's date
        let session_id = "pipeline-1700000000000-test";
        assert!(!session_lock_is_held(session_id, &config));
    }

    #[test]
    fn session_lock_is_held_false_after_lock_released() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let session_id = "pipeline-1700000000000-test";
        let lock_path = pipeline_lock_path(session_id, &config);
        std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();

        // Acquire then drop immediately.
        {
            let file = std::fs::File::create(&lock_path).unwrap();
            let mut rw = fd_lock::RwLock::new(file);
            let mut guard = rw.try_write().unwrap();
            guard.write_all(b"test-holder").unwrap();
            // guard dropped here — lock released
        }

        assert!(!session_lock_is_held(session_id, &config));
    }
}
