use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GateError;
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

            for entry in std::fs::read_dir(&directory)
                .map_err(GateError::Io)?
            {
                let entry = entry.map_err(GateError::Io)?;
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("md") {
                    continue;
                }

                let Some(task_id) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                if !task_id.starts_with("T-") {
                    continue;
                }
                let canonical_task_id = match fs::read_to_string(&path) {
                    Ok(text) => TaskMetadataParser::parse_task_id(&text)
                        .unwrap_or_else(|| task_id.to_string()),
                    Err(_) => task_id.to_string(),
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
                    path,
                    location: location.to_string(),
                });
            }
        }

        tasks.sort_by(|left, right| task_id::natural_cmp(&left.task_id, &right.task_id));
        Ok(tasks)
    }

    pub fn move_to_done(task: &TaskPathSet) -> Result<(), GateError> {
        if task.location == "done" {
            return Ok(());
        }

        let destination = Self::destination_path(task, "done")?;
        fs::rename(&task.path, destination)?;
        Ok(())
    }

    pub fn move_to_todo(task: &TaskPathSet) -> Result<(), GateError> {
        if task.location == "todo" {
            return Ok(());
        }

        let destination = Self::destination_path(task, "todo")?;
        fs::rename(&task.path, destination)?;
        Ok(())
    }

    fn destination_path(task: &TaskPathSet, location: &str) -> Result<PathBuf, GateError> {
        let location_directory = task
            .path
            .ancestors()
            .find(|path| {
                path.file_name().and_then(|value| value.to_str()) == Some(task.location.as_str())
            })
            .ok_or_else(|| GateError::MissingTaskPath(task.path.clone()))?;
        let root = location_directory
            .parent()
            .ok_or_else(|| GateError::MissingTaskPath(task.path.clone()))?;
        Ok(root.join(location).join(format!("{}.md", task.task_id)))
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

    use tempfile::tempdir;

    use super::TaskRepository;
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
}
