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
        let mut tasks = Vec::new();
        let mut seen_task_ids = BTreeMap::new();

        for location in TASK_LOCATIONS {
            let directory = root.join(location);
            if !directory.exists() {
                continue;
            }

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
                let canonical_task_id =
                    match Self::canonical_task_markdown_path(&root, &discovered_task).and_then(
                        |validated_path| {
                            fs::read_to_string(validated_path).map_err(GateError::from)
                        },
                    ) {
                        Ok(text) => TaskMetadataParser::parse_task_id(&text)
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
        if canonical_location_dir.parent() != Some(canonical_root.as_path()) {
            return Err(Self::invalid_gate_task_path(task));
        }

        let metadata = fs::symlink_metadata(&task.path)?;
        if metadata.file_type().is_symlink() {
            return Err(Self::invalid_gate_task_path(task));
        }

        let canonical_task_path = fs::canonicalize(&task.path)?;
        if canonical_task_path.parent() != Some(canonical_location_dir.as_path())
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
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    use tempfile::tempdir;

    use super::{TaskPathSet, TaskRepository};
    use crate::error::GateError;

    #[test]
    fn list_tasks_rejects_duplicate_canonical_task_ids() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        fs::write(
            root.join("todo").join("T-90.25-first.md"),
            "---\nid: T-90.25\ntitle: First duplicate\npipeline: sdd\n---\n# First duplicate\n",
        )
        .expect("write first task");
        fs::write(
            root.join("done").join("T-90.25-second.md"),
            "---\nid: T-90.25\ntitle: Second duplicate\npipeline: sdd\n---\n# Second duplicate\n",
        )
        .expect("write second task");

        let error = TaskRepository::list_tasks(root.to_path_buf()).expect_err("duplicate id");
        match error {
            GateError::DuplicateCanonicalTaskId {
                task_id,
                first_path,
                second_path,
            } => {
                assert_eq!(task_id, "T-90.25");
                assert!(first_path.ends_with("todo/T-90.25-first.md"));
                assert!(second_path.ends_with("done/T-90.25-second.md"));
            }
            other => panic!("expected duplicate canonical task id error, got {other}"),
        }
    }

    #[test]
    fn canonical_task_markdown_path_accepts_task_in_location_corpus() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let task_path = root.join("todo").join("T-1.1.md");
        fs::write(&task_path, "# T-1.1\n").expect("write task");

        let canonical = TaskRepository::canonical_task_markdown_path(
            root,
            &TaskPathSet {
                task_id: "T-1.1".to_string(),
                path: task_path.clone(),
                location: "todo".to_string(),
            },
        )
        .expect("canonical task path");

        assert_eq!(
            canonical,
            fs::canonicalize(task_path).expect("canonicalize task path")
        );
    }

    #[cfg(unix)]
    #[test]
    fn canonical_task_markdown_path_rejects_symlink_escape_outside_root() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let escaped = temp.path().join("escaped.md");
        fs::write(&escaped, "# escaped\n").expect("write escaped file");
        let task_path = root.join("todo").join("T-1.1.md");
        unix_fs::symlink(&escaped, &task_path).expect("create symlink task");

        let error = TaskRepository::canonical_task_markdown_path(
            root,
            &TaskPathSet {
                task_id: "T-1.1".to_string(),
                path: task_path,
                location: "todo".to_string(),
            },
        )
        .expect_err("escaped symlink should be rejected");

        assert!(
            error
                .to_string()
                .contains("resolves outside the gate root corpus")
        );
    }

    #[cfg(unix)]
    #[test]
    fn canonical_task_markdown_path_rejects_symlinked_task_inside_root() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let target = root.join("todo").join("linked-target.md");
        fs::write(&target, "# linked target\n").expect("write linked target");
        let task_path = root.join("todo").join("T-1.1.md");
        unix_fs::symlink(&target, &task_path).expect("create in-root symlink task");

        let error = TaskRepository::canonical_task_markdown_path(
            root,
            &TaskPathSet {
                task_id: "T-1.1".to_string(),
                path: task_path,
                location: "todo".to_string(),
            },
        )
        .expect_err("symlinked task should be rejected");

        assert!(error.to_string().contains("through a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn list_tasks_does_not_parse_frontmatter_from_symlink_target() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let escaped = temp.path().join("escaped.md");
        fs::write(
            &escaped,
            "---\nid: T-999.9\ntitle: Escaped target\npipeline: sdd\n---\n# Escaped target\n",
        )
        .expect("write escaped target");
        let task_path = root.join("todo").join("T-1.1.md");
        unix_fs::symlink(&escaped, &task_path).expect("create task symlink");

        let tasks = TaskRepository::list_tasks(root.to_path_buf()).expect("list tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, "T-1.1");
        assert_eq!(tasks[0].path, task_path);
    }
}
