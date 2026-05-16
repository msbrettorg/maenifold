use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GateError;

use super::{TaskPathSet, TASK_LOCATIONS};

pub(super) fn validate_task_path(
    canonical_root: &Path,
    canonical_location_dir: &Path,
    task: &TaskPathSet,
) -> Result<PathBuf, GateError> {
    if canonical_location_dir.parent() != Some(canonical_root) {
        return Err(invalid_gate_task_path(task));
    }

    let metadata = fs::symlink_metadata(&task.path)?;
    if metadata.file_type().is_symlink() {
        return Err(invalid_gate_task_path(task));
    }

    let canonical_task_path = fs::canonicalize(&task.path)?;
    if canonical_task_path.parent() != Some(canonical_location_dir)
        || canonical_task_path
            .extension()
            .and_then(|value| value.to_str())
            != Some("md")
    {
        return Err(invalid_gate_task_path(task));
    }

    Ok(canonical_task_path)
}

pub(super) fn validate_create_destination(
    root: &Path,
    todo_dir: &Path,
    task: &TaskPathSet,
) -> Result<(), GateError> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical_todo = fs::canonicalize(todo_dir)?;
    if canonical_todo.parent() != Some(canonical_root.as_path()) {
        return Err(invalid_gate_task_path(task));
    }
    if task.path.parent() != Some(todo_dir)
        || task.path.extension().and_then(|value| value.to_str()) != Some("md")
    {
        return Err(invalid_gate_task_path(task));
    }
    Ok(())
}

pub(super) fn validate_destination_directory(
    root: &Path,
    destination_parent: &Path,
    task: &TaskPathSet,
) -> Result<(), GateError> {
    let metadata = fs::symlink_metadata(destination_parent)?;
    if metadata.file_type().is_symlink() {
        return Err(invalid_gate_task_path(task));
    }

    let canonical_root = fs::canonicalize(root)?;
    let canonical_destination_parent = fs::canonicalize(destination_parent)?;
    if !canonical_destination_parent.starts_with(&canonical_root) {
        return Err(invalid_gate_task_path(task));
    }

    Ok(())
}

pub(super) fn invalid_gate_task_path(task: &TaskPathSet) -> GateError {
    GateError::Cli(format!(
        "task '{}' resolves outside the gate root corpus or through a symlink: {}",
        task.task_id,
        task.path.display()
    ))
}

pub(super) fn searched_directories_display(root: &Path) -> String {
    TASK_LOCATIONS
        .iter()
        .map(|location| root.join(location).display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn sync_directory(path: &Path) -> Result<(), GateError> {
    let directory = fs::File::open(path)?;
    directory.sync_all()?;
    Ok(())
}
