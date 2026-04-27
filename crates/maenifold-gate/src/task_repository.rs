use std::path::{Path, PathBuf};

use crate::error::GateError;
use crate::task_file::TaskDocument;

pub use maenifold_gate_core::task_repository::TaskPathSet;

#[derive(Debug, Clone)]
pub struct TaskRepository;

impl TaskRepository {
    pub fn find_task(root: PathBuf, task_id: &str) -> Result<TaskPathSet, GateError> {
        Ok(maenifold_gate_core::task_repository::TaskRepository::find_task(root, task_id)?)
    }

    pub fn list_tasks(root: PathBuf) -> Result<Vec<TaskPathSet>, GateError> {
        Ok(maenifold_gate_core::task_repository::TaskRepository::list_tasks(root)?)
    }

    pub fn move_to_done(root: &Path, task: &TaskPathSet) -> Result<(), GateError> {
        Ok(maenifold_gate_core::task_repository::TaskRepository::move_to_done(root, task)?)
    }

    pub fn move_to_todo(root: &Path, task: &TaskPathSet) -> Result<(), GateError> {
        Ok(maenifold_gate_core::task_repository::TaskRepository::move_to_todo(root, task)?)
    }

    pub fn canonical_task_markdown_path(
        root: &Path,
        task: &TaskPathSet,
    ) -> Result<PathBuf, GateError> {
        Ok(
            maenifold_gate_core::task_repository::TaskRepository::canonical_task_markdown_path(
                root, task,
            )?,
        )
    }

    pub fn read_task_document(root: &Path, task: &TaskPathSet) -> Result<TaskDocument, GateError> {
        Ok(maenifold_gate_core::task_repository::TaskRepository::read_task_document(root, task)?)
    }

    pub fn append_task_section(
        root: &Path,
        task: &TaskPathSet,
        section: &str,
    ) -> Result<(), GateError> {
        Ok(
            maenifold_gate_core::task_repository::TaskRepository::append_task_section(
                root, task, section,
            )?,
        )
    }
}
