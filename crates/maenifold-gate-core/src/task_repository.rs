use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::GateError;
use crate::task_file::{TaskDocument, TaskFileReader, TaskFileWriter};
use crate::task_id;
use crate::task_metadata::TaskMetadataParser;

#[path = "task_repository_paths.rs"]
mod task_repository_paths;

use task_repository_paths::{
    searched_directories_display, sync_directory, validate_create_destination,
    validate_destination_directory, validate_task_path,
};

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
    pub fn create_task(
        root: &Path,
        task_id: &str,
        slug: &str,
        markdown: &str,
    ) -> Result<PathBuf, GateError> {
        let todo_dir = root.join("todo");
        if !todo_dir.exists() {
            return Err(GateError::Cli(format!(
                "required todo directory is missing: {}",
                todo_dir.display()
            )));
        }
        if fs::symlink_metadata(&todo_dir)?.file_type().is_symlink() {
            return Err(GateError::Cli(format!(
                "required todo directory must not be a symlink: {}",
                todo_dir.display()
            )));
        }

        Self::reject_duplicate_task_id(root, task_id)?;

        let destination = todo_dir.join(format!("{task_id}-{slug}.md"));
        let task = TaskPathSet {
            task_id: task_id.to_string(),
            path: destination.clone(),
            location: "todo".to_string(),
        };
        validate_create_destination(root, &todo_dir, &task)?;

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    GateError::Cli(format!(
                        "task '{task_id}' already exists in todo: {}",
                        destination.display()
                    ))
                } else {
                    GateError::Io(error)
                }
            })?;
        file.write_all(markdown.as_bytes())?;
        file.sync_all()?;
        sync_directory(&todo_dir)?;
        Ok(destination)
    }

    pub fn find_task(root: PathBuf, task_id: &str) -> Result<TaskPathSet, GateError> {
        for task in Self::list_tasks(root.clone())? {
            if task.task_id == task_id {
                return Ok(task);
            }
        }

        Err(GateError::TaskNotFound {
            task_id: task_id.to_string(),
            searched_directories: searched_directories_display(&root),
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
                let canonical_task_id = match validate_task_path(
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
        let filename = task
            .path
            .file_name()
            .ok_or_else(|| GateError::MissingTaskPath(task.path.clone()))?;
        Self::move_to(root, task, root.join("done").join(filename))
    }

    pub fn move_to_todo(root: &Path, task: &TaskPathSet) -> Result<(), GateError> {
        Self::move_to_todo_with_slug(root, task)
    }

    pub fn move_to_todo_with_slug(root: &Path, task: &TaskPathSet) -> Result<(), GateError> {
        if task.location == "todo" { return Ok(()); }
        Self::move_to(root, task, root.join("todo").join(Self::recover_slug_filename(root, task)?))
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

        validate_task_path(&canonical_root, &canonical_location_dir, task)
    }

    pub fn read_task_document(root: &Path, task: &TaskPathSet) -> Result<TaskDocument, GateError> {
        let path = Self::canonical_task_markdown_path(root, task)?;
        let document = TaskFileReader::read(&path)?;
        if document
            .metadata
            .depends_on
            .iter()
            .any(|dependency| dependency == &task.task_id)
        {
            return Err(GateError::TaskMetadataParse(
                "task depends on itself".to_string(),
            ));
        }
        Ok(document)
    }

    pub fn append_task_section(
        root: &Path,
        task: &TaskPathSet,
        section: &str,
    ) -> Result<(), GateError> {
        let path = Self::canonical_task_markdown_path(root, task)?;
        TaskFileWriter::append_section(&path, section)
    }

    fn move_to(root: &Path, task: &TaskPathSet, destination: PathBuf) -> Result<(), GateError> {
        let source = Self::canonical_task_markdown_path(root, task)?;
        let source_parent = source.parent().ok_or_else(|| GateError::MissingTaskPath(source.clone()))?;
        let destination_parent = destination.parent().ok_or_else(|| GateError::MissingTaskPath(destination.clone()))?;
        fs::create_dir_all(destination_parent)?;
        validate_destination_directory(root, destination_parent, task)?;
        if fs::symlink_metadata(&destination).is_ok() {
            return Err(GateError::Cli(format!(
                "refusing to overwrite existing destination for task '{}': {}",
                task.task_id,
                destination.display()
            )));
        }
        fs::rename(&source, &destination)?;
        sync_directory(source_parent)?;
        sync_directory(destination_parent)?;
        Ok(())
    }

    fn recover_slug_filename(_root: &Path, task: &TaskPathSet) -> Result<String, GateError> {
        task.path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .ok_or_else(|| GateError::MissingTaskPath(task.path.clone()))
    }

    fn reject_duplicate_task_id(root: &Path, task_id: &str) -> Result<(), GateError> {
        for location in TASK_LOCATIONS {
            let directory = root.join(location);
            if !directory.exists() {
                continue;
            }
            for entry in fs::read_dir(&directory)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_file()
                    || path.extension().and_then(|value| value.to_str()) != Some("md")
                {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                if stem == task_id
                    || stem
                        .strip_prefix(task_id)
                        .is_some_and(|rest| rest.starts_with('-'))
                {
                    return Err(GateError::Cli(format!(
                        "task '{task_id}' already exists in {location}: {}",
                        path.display()
                    )));
                }
            }
        }

        match Self::find_task(root.to_path_buf(), task_id) {
            Ok(task) => Err(GateError::Cli(format!(
                "task '{}' already exists in {}: {}",
                task_id,
                task.location,
                task.path.display()
            ))),
            Err(GateError::TaskNotFound { .. }) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
#[path = "task_repository_tests.rs"]
mod tests;
