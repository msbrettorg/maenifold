use std::fs;
use std::path::PathBuf;

use crate::error::GateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPathSet {
    pub task_id: String,
    pub path: PathBuf,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct TaskRepository;

impl TaskRepository {
    pub fn find_task(root: PathBuf, task_id: &str) -> Result<TaskPathSet, GateError> {
        for location in ["todo", "done"] {
            let path = root.join(location).join(format!("{task_id}.md"));
            if path.exists() {
                return Ok(TaskPathSet {
                    task_id: task_id.to_string(),
                    path,
                    location: location.to_string(),
                });
            }
        }

        Err(GateError::MissingTaskPath(
            root.join("todo").join(format!("{task_id}.md")),
        ))
    }

    pub fn list_tasks(root: PathBuf) -> Result<Vec<TaskPathSet>, GateError> {
        let mut tasks = Vec::new();

        for location in ["todo", "done"] {
            let directory = root.join(location);
            if !directory.exists() {
                continue;
            }

            for entry in fs::read_dir(&directory)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) != Some("md") {
                    continue;
                }

                let Some(task_id) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                tasks.push(TaskPathSet {
                    task_id: task_id.to_string(),
                    path,
                    location: location.to_string(),
                });
            }
        }

        tasks.sort_by(|left, right| left.task_id.cmp(&right.task_id));
        Ok(tasks)
    }

    pub fn move_to_done(task: &TaskPathSet) -> Result<(), GateError> {
        if task.location == "done" {
            return Ok(());
        }

        let parent = task
            .path
            .parent()
            .ok_or_else(|| GateError::MissingTaskPath(task.path.clone()))?;
        let root = parent
            .parent()
            .ok_or_else(|| GateError::MissingTaskPath(task.path.clone()))?;
        let destination = root.join("done").join(format!("{}.md", task.task_id));
        fs::rename(&task.path, destination)?;
        Ok(())
    }
}
