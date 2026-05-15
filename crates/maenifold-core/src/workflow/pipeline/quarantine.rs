use crate::config::{Config, QuarantineConfig};
use crate::error::{MaenifoldError, Result};
use crate::markdown::uri_to_path;
use crate::sessions::{date_parts_from_session_id, pipeline_session_uri, workflow_session_uri};
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use chrono::{NaiveDate, Utc};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreReport {
    pub restored_files: Vec<String>,
    pub skipped_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineCleanupReport {
    pub deleted_files: Vec<String>,
}

pub(super) fn require_quarantine_config(config: &Config) -> Result<QuarantineConfig> {
    config
        .quarantine_config()
        .ok_or_else(|| MaenifoldError::Message("quarantine retention not configured".to_string()))
}

pub(super) fn quarantine_file(uri: &str, config: &Config, session_id: &str) -> Result<String> {
    require_quarantine_config(config)?;
    let path = uri_to_path(uri, &config.memory_path)?;
    let absolute_path = path.canonicalize().unwrap_or(path);
    let name = absolute_path
        .file_name()
        .ok_or_else(|| MaenifoldError::Message(format!("missing file name: {uri}")))?;
    let destination_dir = trash_session_dir(config, session_id);
    fs::create_dir_all(&destination_dir)?;
    let destination = collision_free_path(&destination_dir.join(name));
    move_file(&absolute_path, &destination)?;
    mirror_quarantine_file(config, session_id, name, &destination)?;
    Ok(absolute_path.to_string_lossy().to_string())
}

pub fn restore_pipeline_from_quarantine(
    context: &ToolContext,
    session_id: &str,
    trash_date: &str,
) -> Result<RestoreReport> {
    let trash_dir = context
        .config
        .maenifold_root
        .join(".trash")
        .join(trash_date)
        .join(session_id);
    let mut restored_files = Vec::new();
    let mut skipped_files = 0usize;
    if !trash_dir.exists() {
        return Ok(RestoreReport {
            restored_files,
            skipped_files,
        });
    }
    for entry in fs::read_dir(&trash_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let source = entry.path();
        let destination = restore_destination(&context.config, &source)?;
        if destination.exists() {
            skipped_files += 1;
            continue;
        }
        let parent = destination.parent().ok_or_else(|| {
            MaenifoldError::Message(format!("missing restore parent: {}", destination.display()))
        })?;
        fs::create_dir_all(parent)?;
        move_file(&source, &destination)?;
        restored_files.push(destination.to_string_lossy().to_string());
    }
    Ok(RestoreReport {
        restored_files,
        skipped_files,
    })
}

pub fn run_quarantine_cleanup(config: &Config) -> Result<QuarantineCleanupReport> {
    if config.settings.quarantine_retention_days.is_none() {
        return Err(MaenifoldError::Message(
            "quarantine retention not configured".to_string(),
        ));
    }
    let quarantine = require_quarantine_config(config)?;
    let trash_root = config.maenifold_root.join(".trash");
    let mut deleted_files = Vec::new();
    if !trash_root.exists() {
        return Ok(QuarantineCleanupReport { deleted_files });
    }
    let today = Utc::now().date_naive();
    for entry in fs::read_dir(&trash_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(date) = NaiveDate::parse_from_str(&name, "%Y-%m-%d") else {
            continue;
        };
        if today.signed_duration_since(date).num_days() <= i64::from(quarantine.retention_days) {
            continue;
        }
        collect_files(&entry.path(), &mut deleted_files)?;
        fs::remove_dir_all(entry.path())?;
    }
    Ok(QuarantineCleanupReport { deleted_files })
}

fn trash_session_dir(config: &Config, session_id: &str) -> PathBuf {
    let (year, month, day) = date_parts_from_session_id(session_id);
    config
        .maenifold_root
        .join(".trash")
        .join(format!("{year:04}-{month:02}-{day:02}"))
        .join(session_id)
}

fn visible_quarantine_day_dir(config: &Config, session_id: &str) -> PathBuf {
    let (year, month, day) = date_parts_from_session_id(session_id);
    config
        .memory_path
        .join("pipelines")
        .join(".quarantine")
        .join(format!("{year:04}"))
        .join(format!("{month:02}"))
        .join(format!("{day:02}"))
}

fn mirror_quarantine_file(
    config: &Config,
    session_id: &str,
    name: &std::ffi::OsStr,
    source: &Path,
) -> Result<()> {
    let visible_dir = visible_quarantine_day_dir(config, session_id);
    fs::create_dir_all(&visible_dir)?;
    let visible_destination = collision_free_path(&visible_dir.join(name));
    fs::copy(source, visible_destination)?;
    Ok(())
}

fn collision_free_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    for suffix in 1u32.. {
        let candidate = path.with_file_name(format!(
            "{}.{suffix}",
            path.file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default()
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

fn move_file(source: &Path, destination: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, destination)?;
            fs::remove_file(source)?;
            Ok(())
        }
    }
}

fn restore_destination(config: &Config, source: &Path) -> Result<PathBuf> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MaenifoldError::NonUtf8Path(source.to_path_buf()))?;
    let original_name = strip_numeric_suffix(name);
    let session_id = original_name.strip_suffix(".md").unwrap_or(original_name);
    let uri = if session_id.starts_with("pipeline-") {
        pipeline_session_uri(session_id)
    } else if session_id.starts_with("workflow-") {
        workflow_session_uri(session_id)
    } else {
        sequential_session_uri(session_id)
    };
    uri_to_path(&uri, &config.memory_path)
}

fn strip_numeric_suffix(name: &str) -> &str {
    let Some((base, suffix)) = name.rsplit_once('.') else {
        return name;
    };
    if suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        base
    } else {
        name
    }
}

fn collect_files(dir: &Path, deleted_files: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_files(&path, deleted_files)?;
        } else {
            deleted_files.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}
