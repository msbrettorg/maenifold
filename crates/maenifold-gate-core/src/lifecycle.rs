use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::error::GateError;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::status_filter::StatusFilter;
use crate::task_file::{GateProjectLock, TaskFileLock};
use crate::task_repository::TaskRepository;

#[path = "lifecycle_helpers.rs"]
mod lifecycle_helpers;
pub use lifecycle_helpers::resolve_definition;
use lifecycle_helpers::{
    active_task_state, apply_outcome, canonical_root, status_for_document, status_row,
};

#[derive(Debug, Clone)]
pub struct GateContext {
    pub root: PathBuf,
    pub requested_pipeline_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StatusArgs {
    pub context: GateContext,
    pub filter: StatusFilter,
}

#[derive(Debug, Clone)]
pub struct SyncArgs {
    pub context: GateContext,
    pub rebuild: bool,
}

#[derive(Debug, Clone)]
pub struct TaskArgs {
    pub context: GateContext,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct StageNoteArgs {
    pub context: GateContext,
    pub task_id: String,
    pub agent: String,
    pub notes: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GateLifecycle;

impl GateLifecycle {
    pub fn status(args: StatusArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _lock = GateProjectLock::acquire_shared(&root)?;
        let mut rows = Vec::new();
        for task in TaskRepository::list_tasks(root.clone())? {
            // Skip done/ tasks early when not requested — avoids replaying
            // incomplete stage records for legacy/manually-moved tasks.
            if task.location == "done" && !args.filter.include_done {
                continue;
            }
            let row = match status_row(
                &root,
                &task,
                args.context.requested_pipeline_name.as_deref(),
            ) {
                Ok(row) => row,
                Err(e) => json!({
                    "taskId": task.task_id,
                    "canonicalTaskId": task.task_id,
                    "title": format!("[error] {}", task.task_id),
                    "pipeline": "unknown",
                    "state": "error",
                    "stages": [],
                    "attempts": 0,
                    "location": task.location,
                    "path": task.path.display().to_string(),
                    "linked": false,
                    "issue": format!("{e}"),
                }),
            };
            if args.filter.matches(&row) {
                rows.push(row);
            }
        }
        Ok(Value::Array(rows))
    }

    pub fn sync(args: SyncArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _lock = GateProjectLock::acquire_exclusive(&root)?;
        let gate_dir = root.join(".gate");
        if !gate_dir.exists() {
            std::fs::create_dir_all(&gate_dir).map_err(|e| {
                GateError::Io(std::io::Error::new(
                    e.kind(),
                    format!("failed to create .gate/: {e}"),
                ))
            })?;
        }

        let gitignore_path = root.join(".gitignore");
        let needs_entry = if gitignore_path.exists() {
            let content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
            !content
                .lines()
                .any(|line| line.trim() == ".gate/" || line.trim() == ".gate")
        } else {
            true
        };
        if needs_entry {
            use std::io::Write;

            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&gitignore_path)
                .map_err(|e| {
                    GateError::Io(std::io::Error::new(
                        e.kind(),
                        format!("failed to open .gitignore: {e}"),
                    ))
                })?;
            writeln!(file, "\n.gate/").map_err(|e| {
                GateError::Io(std::io::Error::new(
                    e.kind(),
                    format!("failed to write .gitignore: {e}"),
                ))
            })?;
        }
        Ok(json!({
            "root": root,
            "pipeline": args.context.requested_pipeline_name,
            "rebuild": args.rebuild,
            "taskCount": TaskRepository::list_tasks(root)?.len(),
            "synced": true
        }))
    }

    pub fn history(args: TaskArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _lock = GateProjectLock::acquire_shared(&root)?;
        let task = TaskRepository::find_task(root.clone(), &args.task_id)?;
        let document = TaskRepository::read_task_document(&root, &task)?;
        Ok(Value::Array(
            document
                .records
                .into_iter()
                .map(|record| {
                    json!({
                        "stage": record.stage,
                        "event": record.outcome.map(|value| value.to_uppercase()).unwrap_or(record.record_kind),
                        "agent": record.agent,
                        "model": record.model,
                        "notes": record.notes,
                        "timestamp": record.recorded_at
                    })
                })
                .collect(),
        ))
    }

    pub fn task_body(args: TaskArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _lock = GateProjectLock::acquire_shared(&root)?;
        let task = TaskRepository::find_task(root.clone(), &args.task_id)?;
        let path = TaskRepository::canonical_task_markdown_path(&root, &task)?;
        Ok(json!({
            "taskId": task.task_id,
            "path": path,
            "markdown": fs::read_to_string(&path)?
        }))
    }

    pub fn append(args: StageNoteArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _project_lock = GateProjectLock::acquire_exclusive(&root)?;
        let _task_lock = TaskFileLock::acquire(&root, &args.task_id)?;
        let task = TaskRepository::find_task(root.clone(), &args.task_id)?;
        let (document, definition, state) = active_task_state(
            &root,
            &task,
            args.context.requested_pipeline_name.as_deref(),
        )?;
        if definition
            .find_state(&state.state)
            .ok_or_else(|| GateError::UnknownState(state.state.clone()))?
            .terminal
        {
            return Err(GateError::Cli(format!(
                "task '{}' is already terminal; append is not allowed on completed tasks",
                args.task_id
            )));
        }
        let acting_stage = StateMachine::acting_stage_name(&state.state, &definition)?;
        let section = CanonicalSectionFactory::build_append_section(
            &acting_stage,
            &args.agent,
            &args.notes,
            args.model.as_deref(),
        );
        TaskRepository::append_task_section(&root, &task, &section)?;
        status_for_document(&root, &task, &document.metadata.pipeline)
    }

    pub fn pass(args: StageNoteArgs) -> Result<Value, GateError> {
        apply_outcome(args, "pass")
    }

    pub fn fail(args: StageNoteArgs) -> Result<Value, GateError> {
        apply_outcome(args, "fail")
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
