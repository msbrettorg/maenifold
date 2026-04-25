use super::store::validate_schedule_id;
use anyhow::{Context, Result, bail};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct LockGuard {
    path: PathBuf,
}

impl LockGuard {
    /// Try to acquire lock for a task. Returns Ok(guard) if acquired,
    /// or Err with message if another process holds it.
    pub fn try_acquire(locks_dir: &Path, task_id: &str) -> Result<Self> {
        validate_schedule_id(task_id)?;
        fs::create_dir_all(locks_dir).with_context(|| {
            format!("failed to create lock directory '{}'", locks_dir.display())
        })?;

        let lock_path = locks_dir.join(format!("{task_id}.lock"));
        for _ in 0..8 {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    write!(file, "{}", std::process::id()).with_context(|| {
                        format!("failed to write lock '{}'", lock_path.display())
                    })?;
                    file.sync_all().with_context(|| {
                        format!("failed to sync lock file '{}'", lock_path.display())
                    })?;
                    return Ok(Self { path: lock_path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if let Some(pid) = read_lock_pid(&lock_path)? {
                        if is_process_alive(pid) {
                            bail!("task '{task_id}' already running (PID {pid})");
                        }
                    }

                    match fs::remove_file(&lock_path) {
                        Ok(()) => {}
                        Err(remove_error)
                            if remove_error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(remove_error) => {
                            return Err(remove_error).with_context(|| {
                                format!(
                                    "failed to remove stale lock '{}'",
                                    lock_path.display()
                                )
                            });
                        }
                    }
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("failed to create lock '{}'", lock_path.display())
                    });
                }
            }
        }

        bail!("failed to acquire lock '{}'", lock_path.display())
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn read_lock_pid(lock_path: &Path) -> Result<Option<u32>> {
    let content = match fs::read_to_string(lock_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read lock '{}'", lock_path.display()));
        }
    };

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Ok(trimmed.parse::<u32>().ok())
}

#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    let pid = match i32::try_from(pid) {
        Ok(pid) => pid,
        Err(_) => return false,
    };
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || matches!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EPERM))
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    false
}
