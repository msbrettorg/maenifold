use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::GateError;
use crate::task_metadata::{TaskMetadata, TaskMetadataParser};
use crate::task_sections::{TaskSectionParser, TaskStageRecord};

/// Validate a task ID to prevent path traversal attacks.
///
/// Rejects empty strings, path separators (`/`, `\`), parent directory
/// references (`..`), current-directory references (`.`), and strings
/// that look like absolute paths.
pub fn validate_task_id(task_id: &str) -> Result<(), GateError> {
    if task_id.is_empty() {
        return Err(GateError::Cli(
            "taskId must not be empty".to_string(),
        ));
    }
    if task_id == "." || task_id == ".." {
        return Err(GateError::Cli(format!(
            "taskId '{task_id}' is a relative path component and is not allowed"
        )));
    }
    if task_id.contains('/') || task_id.contains('\\') {
        return Err(GateError::Cli(format!(
            "taskId '{task_id}' contains path separators and is not allowed"
        )));
    }
    if task_id.contains("..") {
        return Err(GateError::Cli(format!(
            "taskId '{task_id}' contains '..' and is not allowed"
        )));
    }
    // Absolute path check (Unix and Windows)
    if task_id.starts_with('/') || (task_id.len() >= 2 && task_id.as_bytes()[1] == b':') {
        return Err(GateError::Cli(format!(
            "taskId '{task_id}' looks like an absolute path and is not allowed"
        )));
    }
    Ok(())
}

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct TaskDocument {
    pub metadata: TaskMetadata,
    pub records: Vec<TaskStageRecord>,
}

#[derive(Debug, Clone)]
pub struct TaskFileReader;

impl TaskFileReader {
    pub fn read(path: &Path) -> Result<TaskDocument, GateError> {
        let text = fs::read_to_string(path)?;
        let lines: Vec<&str> = text.lines().collect();
        let metadata = TaskMetadataParser::parse(&lines)?;
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
    file: fs::File,
}

#[derive(Debug)]
pub struct GateProjectLock {
    file: fs::File,
}

impl TaskFileLock {
    pub fn acquire(root: &Path, task_id: &str) -> Result<Self, GateError> {
        validate_task_id(task_id)?;
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
        lock_exclusive(&file)?;
        Ok(Self { file })
    }
}

impl GateProjectLock {
    /// Acquire a shared lock for read operations. If the `.gate` directory
    /// does not exist (i.e. no tasks have been written yet), returns `None`
    /// instead of creating the directory — read operations should not
    /// produce side-effects on the filesystem.
    pub fn acquire_shared_if_exists(root: &Path) -> Result<Option<Self>, GateError> {
        let lock_path = project_lock_path(root);
        if !lock_path
            .parent()
            .map(|p| p.exists())
            .unwrap_or(false)
        {
            return Ok(None);
        }
        if !lock_path.exists() {
            return Ok(None);
        }
        let file = OpenOptions::new()
            .read(true)
            .open(&lock_path)?;
        lock_shared(&file)?;
        Ok(Some(Self { file }))
    }

    #[allow(dead_code)] // Retained for non-MCP callers that need guaranteed locking
    pub fn acquire_shared(root: &Path) -> Result<Self, GateError> {
        let file = open_project_lock(root)?;
        lock_shared(&file)?;
        Ok(Self { file })
    }

    pub fn acquire_exclusive(root: &Path) -> Result<Self, GateError> {
        let file = open_project_lock(root)?;
        lock_exclusive(&file)?;
        Ok(Self { file })
    }
}

impl Drop for TaskFileLock {
    fn drop(&mut self) {
        let _ = unlock(&self.file);
    }
}

impl Drop for GateProjectLock {
    fn drop(&mut self) {
        let _ = unlock(&self.file);
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

#[cfg(unix)]
fn lock_exclusive(file: &fs::File) -> Result<(), GateError> {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().into())
    }
}

#[cfg(unix)]
fn lock_shared(file: &fs::File) -> Result<(), GateError> {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().into())
    }
}

#[cfg(not(unix))]
fn lock_exclusive(_file: &fs::File) -> Result<(), GateError> {
    Ok(())
}

#[cfg(not(unix))]
fn lock_shared(_file: &fs::File) -> Result<(), GateError> {
    Ok(())
}

#[cfg(unix)]
fn unlock(file: &fs::File) -> std::io::Result<()> {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn unlock(_file: &fs::File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;
    use std::time::Duration;

    use tempfile::tempdir;

    use super::{TaskFileLock, TaskFileWriter};

    #[test]
    fn exclusive_lock_blocks_until_first_guard_is_dropped() {
        let root = tempdir().expect("create temp root");
        let first_lock = TaskFileLock::acquire(root.path(), "T-90.24").expect("acquire first lock");
        let root_path = root.path().to_path_buf();
        let (sender, receiver) = mpsc::channel();

        let handle = std::thread::spawn(move || {
            let _second_lock =
                TaskFileLock::acquire(&root_path, "T-90.24").expect("acquire second lock");
            sender.send(()).expect("notify second lock acquired");
        });

        assert!(receiver.recv_timeout(Duration::from_millis(150)).is_err());
        drop(first_lock);
        receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("second lock should acquire after unlock");
        handle.join().expect("join second lock thread");
    }

    #[test]
    fn append_section_replaces_file_without_leaving_temp_files() {
        let root = tempdir().expect("create temp root");
        let task_path = root.path().join("T-90.24.md");
        fs::write(&task_path, "# T-90.24\n").expect("write initial task");

        TaskFileWriter::append_section(&task_path, "## SWE\n\n- complete").expect("append section");

        let text = fs::read_to_string(&task_path).expect("read task");
        assert!(text.contains("## SWE"));

        let entries = fs::read_dir(root.path())
            .expect("read temp dir")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(entries, vec!["T-90.24.md"]);
    }

    #[test]
    fn validate_task_id_rejects_path_traversal() {
        use super::validate_task_id;

        // Empty
        assert!(validate_task_id("").is_err());

        // Dot components
        assert!(validate_task_id(".").is_err());
        assert!(validate_task_id("..").is_err());

        // Path separators
        assert!(validate_task_id("../etc/passwd").is_err());
        assert!(validate_task_id("foo/bar").is_err());
        assert!(validate_task_id("foo\\bar").is_err());

        // Absolute paths
        assert!(validate_task_id("/etc/passwd").is_err());
        assert!(validate_task_id("C:Windows").is_err());

        // Embedded ..
        assert!(validate_task_id("T-1..2").is_err());

        // Valid task IDs
        assert!(validate_task_id("T-90.24").is_ok());
        assert!(validate_task_id("T-237.1a2").is_ok());
        assert!(validate_task_id("my-task").is_ok());
    }

    #[test]
    fn lock_acquire_rejects_traversal_task_id() {
        let root = tempdir().expect("create temp root");
        let err = TaskFileLock::acquire(root.path(), "../escape")
            .expect_err("traversal task id should fail");
        assert!(err.to_string().contains("path separators"));
    }
}
