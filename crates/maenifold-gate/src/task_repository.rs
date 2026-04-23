use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GateError;
use crate::task_file::{TaskDocument, TaskFileReader, TaskFileWriter};
use crate::task_id;
use crate::task_metadata::TaskMetadataParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPathSet {
    pub task_id: String,
    pub path: PathBuf,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct TaskRepository;

const TASK_LOCATIONS: [&str; 2] = ["todo", "done"];

impl TaskRepository {
    pub fn find_task(root: PathBuf, task_id: &str) -> Result<TaskPathSet, GateError> {
        for task in Self::list_tasks(root.clone())? {
            if task.task_id == task_id {
                return Ok(task);
            }
        }

        Err(GateError::TaskNotFound {
            task_id: task_id.to_string(),
            searched_directories: Self::searched_directories_display(&root),
        })
    }

    pub fn list_tasks(root: PathBuf) -> Result<Vec<TaskPathSet>, GateError> {
        let canonical_root = fs::canonicalize(&root)?;
        let mut tasks = Vec::new();
        let mut seen_task_ids = BTreeMap::new();

        for location in TASK_LOCATIONS {
            let directory = root.join(location);
            if !directory.exists() {
                continue;
            }
            let canonical_location = fs::canonicalize(&directory)?;

            for entry in std::fs::read_dir(&directory).map_err(GateError::Io)? {
                let entry = entry.map_err(GateError::Io)?;
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("md") {
                    continue;
                }

                let Some(file_task_id) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                if !file_task_id.starts_with("T-") {
                    continue;
                }

                let discovered_task = TaskPathSet {
                    task_id: file_task_id.to_string(),
                    path: path.clone(),
                    location: location.to_string(),
                };
                let canonical_task_id = match Self::validate_task_path(
                    &canonical_root,
                    &canonical_location,
                    &discovered_task,
                )
                .and_then(|validated_path| {
                    fs::read_to_string(validated_path).map_err(GateError::from)
                }) {
                    Ok(text) => TaskMetadataParser::parse_task_id(&text)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| file_task_id.to_string()),
                    Err(_) => file_task_id.to_string(),
                };
                if let Some(previous_path) =
                    seen_task_ids.insert(canonical_task_id.clone(), path.clone())
                {
                    return Err(GateError::DuplicateCanonicalTaskId {
                        task_id: canonical_task_id,
                        first_path: previous_path,
                        second_path: path,
                    });
                }
                tasks.push(TaskPathSet {
                    task_id: canonical_task_id,
                    ..discovered_task
                });
            }
        }

        tasks.sort_by(|left, right| task_id::natural_cmp(&left.task_id, &right.task_id));
        Ok(tasks)
    }

    pub fn move_to_done(root: &Path, task: &TaskPathSet) -> Result<(), GateError> {
        if task.location == "done" {
            return Ok(());
        }

        Self::move_to_location(root, task, "done")
    }

    pub fn move_to_todo(root: &Path, task: &TaskPathSet) -> Result<(), GateError> {
        if task.location == "todo" {
            return Ok(());
        }

        Self::move_to_location(root, task, "todo")
    }

    pub fn canonical_task_markdown_path(
        root: &Path,
        task: &TaskPathSet,
    ) -> Result<PathBuf, GateError> {
        if !TASK_LOCATIONS.contains(&task.location.as_str()) {
            return Err(GateError::Cli(format!(
                "task '{}' has unsupported location '{}'",
                task.task_id, task.location
            )));
        }

        let canonical_root = fs::canonicalize(root)?;
        let location_dir = root.join(&task.location);
        let canonical_location_dir = fs::canonicalize(&location_dir)?;

        Self::validate_task_path(&canonical_root, &canonical_location_dir, task)
    }

    fn validate_task_path(
        canonical_root: &Path,
        canonical_location_dir: &Path,
        task: &TaskPathSet,
    ) -> Result<PathBuf, GateError> {
        if canonical_location_dir.parent() != Some(canonical_root) {
            return Err(Self::invalid_gate_task_path(task));
        }

        let metadata = fs::symlink_metadata(&task.path)?;
        if metadata.file_type().is_symlink() {
            return Err(Self::invalid_gate_task_path(task));
        }

        let canonical_task_path = fs::canonicalize(&task.path)?;
        if canonical_task_path.parent() != Some(canonical_location_dir)
            || canonical_task_path
                .extension()
                .and_then(|value| value.to_str())
                != Some("md")
        {
            return Err(Self::invalid_gate_task_path(task));
        }

        Ok(canonical_task_path)
    }

    pub fn read_task_document(root: &Path, task: &TaskPathSet) -> Result<TaskDocument, GateError> {
        let path = Self::canonical_task_markdown_path(root, task)?;
        TaskFileReader::read(&path)
    }

    pub fn append_task_section(
        root: &Path,
        task: &TaskPathSet,
        section: &str,
    ) -> Result<(), GateError> {
        let path = Self::canonical_task_markdown_path(root, task)?;
        TaskFileWriter::append_section(&path, section)
    }

    fn move_to_location(root: &Path, task: &TaskPathSet, location: &str) -> Result<(), GateError> {
        let source = Self::canonical_task_markdown_path(root, task)?;
        let destination = root.join(location).join(format!("{}.md", task.task_id));
        fs::rename(source, destination)?;
        Ok(())
    }

    fn invalid_gate_task_path(task: &TaskPathSet) -> GateError {
        GateError::Cli(format!(
            "task '{}' resolves outside the gate root corpus or through a symlink: {}",
            task.task_id,
            task.path.display()
        ))
    }

    fn searched_directories_display(root: &Path) -> String {
        TASK_LOCATIONS
            .iter()
            .map(|location| root.join(location).display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
#[path = "task_repository_tests.rs"]
mod tests;
