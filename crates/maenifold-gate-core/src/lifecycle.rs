use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use crate::error::GateError;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::status_filter::StatusFilter;
use crate::task_file::{GateProjectLock, TaskFileLock};
use crate::task_repository::TaskRepository;

#[path = "lifecycle_create.rs"]
mod lifecycle_create;
#[path = "lifecycle_helpers.rs"]
mod lifecycle_helpers;
#[path = "lifecycle_status_output.rs"]
mod lifecycle_status_output;
use lifecycle_create::create_task;
pub use lifecycle_helpers::resolve_definition;
use lifecycle_helpers::{
    active_task_state, apply_outcome, canonical_root, status_for_document, status_row,
};
use lifecycle_status_output::{error_status_row, status_summary_row, write_gate_db_snapshot};

#[derive(Debug, Clone)]
pub struct GateContext {
    pub root: PathBuf,
    pub requested_pipeline_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StatusArgs {
    pub context: GateContext,
    pub filter: StatusFilter,
    pub summary: bool,
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
pub struct CreateTaskArgs {
    pub context: GateContext,
    pub task_id: String,
    pub title: String,
    pub body: Option<String>,
    pub priority: Option<String>,
    pub sprint: Option<u64>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StageNoteArgs {
    pub context: GateContext,
    pub task_id: String,
    pub agent: String,
    pub notes: String,
    pub model: Option<String>,
    pub expected_stage: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GateLifecycle;

static CREATE_TASK_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^T-(\d+)\.\d+$").expect("gate create task id regex"));

impl GateLifecycle {
    pub fn create(args: CreateTaskArgs) -> Result<Value, GateError> {
        create_task(args)
    }

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
                Err(e) => error_status_row(&root, &task, e),
            };
            if args.filter.matches(&row) {
                rows.push(if args.summary {
                    status_summary_row(&row)
                } else {
                    row
                });
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
        let tasks = TaskRepository::list_tasks(root.clone())?;
        write_gate_db_snapshot(&root, &gate_dir, &tasks)?;
        let task_count = tasks.len();
        Ok(json!({
            "root": root,
            "pipeline": args.context.requested_pipeline_name,
            "rebuild": args.rebuild,
            "taskCount": task_count,
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
