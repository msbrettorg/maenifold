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

impl TaskFileLock {
    pub fn acquire(root: &Path, task_id: &str) -> Result<Self, GateError> {
        let lock_path = lock_path(root, task_id);
        let lock_dir = lock_path
            .parent()
            .ok_or_else(|| GateError::MissingTaskPath(lock_path.clone()))?;
        fs::create_dir_all(lock_dir)?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(lock_path)?;
        lock_exclusive(&file)?;
        Ok(Self { file })
    }
}

impl Drop for TaskFileLock {
    fn drop(&mut self) {
        let _ = unlock(&self.file);
    }
}

fn lock_path(root: &Path, task_id: &str) -> PathBuf {
    root.join(".gate")
        .join("locks")
        .join(format!("{task_id}.lock"))
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

#[cfg(not(unix))]
fn lock_exclusive(_file: &fs::File) -> Result<(), GateError> {
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
}
