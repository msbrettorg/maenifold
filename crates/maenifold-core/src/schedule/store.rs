use super::ScheduledTask;
use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

pub struct ScheduleStore {
    base_dir: PathBuf,
}

impl ScheduleStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn create(&self, task: &ScheduledTask) -> Result<()> {
        validate_schedule_id(&task.id)?;
        if self.exists(&task.id) {
            bail!("scheduled task '{}' already exists", task.id);
        }

        self.ensure_task_dirs(&task.id)?;
        self.write_task(task)
    }

    pub fn read(&self, id: &str) -> Result<ScheduledTask> {
        validate_schedule_id(id)?;
        let path = self.task_path(id);
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read scheduled task '{}'", path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse scheduled task '{}'", path.display()))
    }

    pub fn update(&self, task: &ScheduledTask) -> Result<()> {
        validate_schedule_id(&task.id)?;
        if !self.exists(&task.id) {
            bail!("scheduled task '{}' does not exist", task.id);
        }

        self.ensure_task_dirs(&task.id)?;
        let mut updated = task.clone();
        updated.updated_at = Utc::now();
        self.write_task(&updated)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        validate_schedule_id(id)?;
        let path = self.task_path(id);
        if path.exists() {
            fs::remove_file(&path).with_context(|| {
                format!("failed to remove scheduled task '{}'", path.display())
            })?;
        }

        let task_dir = self.task_dir(id);
        if task_dir.exists() {
            fs::remove_dir_all(&task_dir).with_context(|| {
                format!("failed to remove scheduled task directory '{}'", task_dir.display())
            })?;
        }

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<ScheduledTask>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut paths = fs::read_dir(&self.base_dir)
            .with_context(|| format!("failed to read schedule directory '{}'", self.base_dir.display()))?
            .filter_map(|entry| entry.ok().map(|value| value.path()))
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        paths.sort();

        let mut tasks = Vec::new();
        for path in paths {
            match fs::read(&path)
                .with_context(|| format!("failed to read scheduled task '{}'", path.display()))
                .and_then(|bytes| {
                    serde_json::from_slice::<ScheduledTask>(&bytes).with_context(|| {
                        format!("failed to parse scheduled task '{}'", path.display())
                    })
                }) {
                Ok(task) => tasks.push(task),
                Err(error) => warn!(path = %path.display(), error = %error, "skipping malformed scheduled task"),
            }
        }

        Ok(tasks)
    }

    pub fn exists(&self, id: &str) -> bool {
        validate_schedule_id(id).is_ok() && self.task_path(id).exists()
    }

    fn write_task(&self, task: &ScheduledTask) -> Result<()> {
        fs::create_dir_all(&self.base_dir).with_context(|| {
            format!("failed to create schedule directory '{}'", self.base_dir.display())
        })?;
        let payload = serde_json::to_vec_pretty(task)
            .with_context(|| format!("failed to serialize scheduled task '{}'", task.id))?;
        write_atomically(&self.task_path(&task.id), &payload)
    }

    fn ensure_task_dirs(&self, id: &str) -> Result<()> {
        fs::create_dir_all(self.task_dir(id).join("logs"))?;
        fs::create_dir_all(self.task_dir(id).join("runs"))?;
        Ok(())
    }

    fn task_path(&self, id: &str) -> PathBuf {
        self.base_dir.join(format!("{id}.json"))
    }

    fn task_dir(&self, id: &str) -> PathBuf {
        self.base_dir.join(id)
    }
}

fn validate_schedule_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 64 {
        bail!("schedule id must be between 1 and 64 characters");
    }
    if !id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        bail!("schedule id must contain only alphanumeric characters or hyphens");
    }
    Ok(())
}

fn write_atomically(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("scheduled task path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create directory '{}'", parent.display()))?;

    for _ in 0..8 {
        let temp_path = create_sidecar_path(path, "schedule-tmp");
        let backup_path = create_sidecar_path(path, "schedule-prev");
        let result = (|| -> std::io::Result<()> {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)?;
            file.write_all(content)?;
            file.sync_all()?;
            drop(file);

            let existing_moved = if path.exists() {
                fs::rename(path, &backup_path)?;
                true
            } else {
                false
            };

            if let Err(error) = fs::rename(&temp_path, path) {
                if existing_moved {
                    let _ = fs::rename(&backup_path, path);
                    sync_directory(parent);
                }
                return Err(error);
            }

            sync_directory(parent);

            if existing_moved {
                fs::remove_file(&backup_path)?;
                sync_directory(parent);
            }

            Ok(())
        })();

        let _ = fs::remove_file(&temp_path);
        let _ = fs::remove_file(&backup_path);

        match result {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }

    bail!("failed to allocate atomic temp file for {}", path.display())
}

fn create_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("schedule.json");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.{suffix}-{nonce}"))
}

fn sync_directory(path: &Path) {
    let _ = fs::File::open(path).and_then(|file| file.sync_all());
}
