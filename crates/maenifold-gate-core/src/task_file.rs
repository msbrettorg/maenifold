use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::GateError;
use crate::task_metadata::{TaskMetadata, TaskMetadataParser};
use crate::task_sections::{TaskSectionParser, TaskStageRecord};
use crate::task_status::TaskDocumentLike;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct TaskDocument {
    pub metadata: TaskMetadata,
    pub records: Vec<TaskStageRecord>,
}

impl TaskDocumentLike for TaskDocument {
    type Record = TaskStageRecord;

    fn metadata_state(&self) -> Option<&str> {
        self.metadata.state.as_deref()
    }

    fn records(&self) -> &[Self::Record] {
        &self.records
    }
}

#[derive(Debug, Clone)]
pub struct TaskFileReader;

impl TaskFileReader {
    pub fn read(path: &Path) -> Result<TaskDocument, GateError> {
        let text = fs::read_to_string(path)?;
        let lines: Vec<&str> = text.lines().collect();
        let metadata = TaskMetadataParser::parse_with_path(&lines, path)?;
        let records = TaskSectionParser::parse(&lines)?;
        Ok(TaskDocument { metadata, records })
    }
}

#[derive(Debug, Clone)]
pub struct TaskFileWriter;

impl TaskFileWriter {
    pub fn append_section(path: &Path, section: &str) -> Result<(), GateError> {
        let mut text = fs::read_to_string(path)?;
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
        text.push_str(section.trim_end());
        text.push('\n');
        write_atomically(path, &text)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct TaskFileLock {
    _lock: fd_lock::RwLock<fs::File>,
}

#[derive(Debug)]
pub struct GateProjectLock {
    _lock: fd_lock::RwLock<fs::File>,
}

impl TaskFileLock {
    pub fn acquire(root: &Path, task_id: &str) -> Result<Self, GateError> {
        let lock_path = lock_path(root, task_id);
        let lock_dir = lock_path
            .parent()
            .ok_or_else(|| GateError::MissingTaskPath(lock_path.clone()))?;
        fs::create_dir_all(lock_dir)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)?;
        let mut lock = fd_lock::RwLock::new(file);
        let guard = lock.write()?;
        // The public gate API returns an owned lock guard, so the OS advisory lock
        // must stay held after this constructor returns. `fd-lock` exposes a
        // borrow-based guard; forgetting it keeps the lock held, and dropping
        // `_lock` later closes the file descriptor/handle and releases it.
        std::mem::forget(guard);
        Ok(Self { _lock: lock })
    }
}

impl GateProjectLock {
    pub fn acquire_shared(root: &Path) -> Result<Self, GateError> {
        let file = open_project_lock(root)?;
        let lock = fd_lock::RwLock::new(file);
        let guard = lock.read()?;
        // Keep the shared project lock held for this owned guard's lifetime.
        // Dropping `_lock` closes the file descriptor/handle and releases it.
        std::mem::forget(guard);
        Ok(Self { _lock: lock })
    }

    pub fn acquire_exclusive(root: &Path) -> Result<Self, GateError> {
        let file = open_project_lock(root)?;
        let mut lock = fd_lock::RwLock::new(file);
        let guard = lock.write()?;
        // Keep the exclusive project lock held for this owned guard's lifetime.
        // Dropping `_lock` closes the file descriptor/handle and releases it.
        std::mem::forget(guard);
        Ok(Self { _lock: lock })
    }
}

fn lock_path(root: &Path, task_id: &str) -> PathBuf {
    root.join(".gate")
        .join("locks")
        .join(format!("{task_id}.lock"))
}

fn project_lock_path(root: &Path) -> PathBuf {
    root.join(".gate").join("gate.lock")
}

fn open_project_lock(root: &Path) -> Result<fs::File, GateError> {
    let lock_path = project_lock_path(root);
    let lock_dir = lock_path
        .parent()
        .ok_or_else(|| GateError::MissingTaskPath(lock_path.clone()))?;
    fs::create_dir_all(lock_dir).map_err(|error| {
        GateError::Cli(format!(
            "cannot create gate lock directory '{}': {error}; ensure the project root is writable or pass --root <writable-path>",
            lock_dir.display()
        ))
    })?;
    Ok(OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?)
}

fn write_atomically(path: &Path, text: &str) -> Result<(), GateError> {
    let parent = path
        .parent()
        .ok_or_else(|| GateError::MissingTaskPath(path.to_path_buf()))?;
    for _ in 0..8 {
        let temp_path = create_temp_path(path);
        let result = (|| -> Result<(), GateError> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp_path, path)?;
            sync_directory(parent)?;
            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }

        match result {
            Ok(()) => return Ok(()),
            Err(GateError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                continue;
            }
            Err(error) => return Err(error),
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        format!(
            "failed to allocate atomic write temp file for {}",
            path.display()
        ),
    )
    .into())
}

fn create_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("task.md");
    let unique = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(
            ".{file_name}.gate-tmp-{}-{timestamp}-{unique}",
            std::process::id(),
        ))
}

fn sync_directory(path: &Path) -> Result<(), GateError> {
    let directory = fs::File::open(path)?;
    directory.sync_all()?;
    Ok(())
}

#[cfg(test)]
#[path = "task_file_tests.rs"]
mod tests;
