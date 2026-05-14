//! Pause-all state for schedules.
//!
//! Single-process daemon; no inter-process locking. If concurrency is added later,
//! switch to filesystem advisory locks (e.g., `fs2::FileExt::lock_exclusive`).

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseState {
    pub paused_at: DateTime<Utc>,
    pub prior_enabled: BTreeMap<String, bool>,
}

#[derive(Debug, Clone)]
pub enum CreateOutcome {
    Created,
    AlreadyExists(PauseState),
}

pub fn path(maenifold_root: &Path) -> PathBuf {
    maenifold_root.join("schedules-pause-state.json")
}

pub fn try_create(maenifold_root: &Path, state: &PauseState) -> Result<CreateOutcome> {
    let state_path = path(maenifold_root);
    let parent = state_path
        .parent()
        .context("pause state path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create directory '{}'", parent.display()))?;
    let payload = serde_json::to_vec_pretty(state).context("failed to serialize pause state")?;
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&state_path)
    {
        Ok(mut file) => {
            file.write_all(&payload).with_context(|| {
                format!("failed to write pause state '{}'", state_path.display())
            })?;
            file.sync_all().with_context(|| {
                format!("failed to sync pause state '{}'", state_path.display())
            })?;
            Ok(CreateOutcome::Created)
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(CreateOutcome::AlreadyExists(
            read(maenifold_root)?.unwrap_or(PauseState {
                paused_at: Utc::now(),
                prior_enabled: BTreeMap::new(),
            }),
        )),
        Err(error) => Err(error)
            .with_context(|| format!("failed to create pause state '{}'", state_path.display())),
    }
}

pub fn read(maenifold_root: &Path) -> Result<Option<PauseState>> {
    let state_path = path(maenifold_root);
    match fs::read(&state_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse pause state '{}'", state_path.display()))
            .map(Some),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error)
            .with_context(|| format!("failed to read pause state '{}'", state_path.display())),
    }
}

pub fn clear(maenifold_root: &Path) -> Result<()> {
    let state_path = path(maenifold_root);
    match fs::remove_file(&state_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to remove pause state '{}'", state_path.display())),
    }
}

pub fn is_paused(maenifold_root: &Path) -> Result<bool> {
    Ok(read(maenifold_root)?.is_some())
}
