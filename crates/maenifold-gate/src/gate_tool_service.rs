use std::fs;

use serde_json::{Value, json};

use crate::append_command::append_task;
use crate::cache_db::GateCacheDb;
use crate::cancel_command::cancel_task;
use crate::create_command::create_task;
use crate::cli::{
    AppendArgs, CancelArgs, CreateArgs, HistoryArgs, OutcomeArgs, StatusArgs, SyncArgs,
    TaskBodyArgs,
};
use crate::error::GateError;
use crate::fail_command::fail_task;
use crate::gate_tool_helpers::{
    ensure_existing_read_model, refresh_read_model_unlocked, status_from_existing_read_model,
    task_status_from_read_model_unlocked,
};
use crate::pass_command::pass_task;
use crate::sync_command::sync_gate_unlocked;
use crate::task_file::GateProjectLock;
use crate::task_repository::TaskRepository;

#[derive(Debug, Clone)]
pub struct GateToolService;

impl GateToolService {
    pub fn status(args: StatusArgs) -> Result<Value, GateError> {
        ensure_existing_read_model(&GateCacheDb::path_for_root(&args.context.root))?;
        let _lock = GateProjectLock::acquire_shared(&args.context.root)?;
        status_from_existing_read_model(&args.context, &args.filter)
    }

    pub fn append(args: AppendArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let context = args.context.clone();
        let _lock = GateProjectLock::acquire_exclusive(&context.root)?;
        append_task(args)?;
        refresh_read_model_unlocked(&context)?;
        task_status_from_read_model_unlocked(&context, &task_id)
    }

    pub fn create(args: CreateArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let context = args.context.clone();
        let _lock = GateProjectLock::acquire_exclusive(&context.root)?;
        create_task(args)?;
        refresh_read_model_unlocked(&context)?;
        task_status_from_read_model_unlocked(&context, &task_id)
    }

    pub fn pass(args: OutcomeArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let context = args.context.clone();
        let _lock = GateProjectLock::acquire_exclusive(&context.root)?;
        pass_task(args)?;
        refresh_read_model_unlocked(&context)?;
        task_status_from_read_model_unlocked(&context, &task_id)
    }

    pub fn fail(args: OutcomeArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let context = args.context.clone();
        let _lock = GateProjectLock::acquire_exclusive(&context.root)?;
        fail_task(args)?;
        refresh_read_model_unlocked(&context)?;
        task_status_from_read_model_unlocked(&context, &task_id)
    }

    pub fn cancel(args: CancelArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let context = args.context.clone();
        let _lock = GateProjectLock::acquire_exclusive(&context.root)?;
        cancel_task(args)?;
        refresh_read_model_unlocked(&context)?;
        task_status_from_read_model_unlocked(&context, &task_id)
    }

    pub fn history(args: HistoryArgs) -> Result<Value, GateError> {
        let root = args.context.root;
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
                        "event": history_event(&record.record_kind, record.outcome.as_deref()),
                        "agent": record.agent,
                        "model": record.model,
                        "notes": record.notes,
                        "timestamp": record.recorded_at,
                    })
                })
                .collect(),
        ))
    }

    pub fn task_body(args: TaskBodyArgs) -> Result<Value, GateError> {
        let root = args.context.root;
        let _lock = GateProjectLock::acquire_shared(&root)?;
        let task = TaskRepository::find_task(root.clone(), &args.task_id)?;
        let path = TaskRepository::canonical_task_markdown_path(&root, &task)?;
        let markdown = fs::read_to_string(&path)?;
        Ok(json!({
            "taskId": task.task_id,
            "path": path,
            "markdown": markdown,
        }))
    }

    pub fn sync(args: SyncArgs) -> Result<Value, GateError> {
        let root = args.context.root.clone();
        let requested_pipeline = args.context.requested_pipeline_name.clone();
        let rebuild = args.rebuild;
        let _lock = GateProjectLock::acquire_exclusive(&root)?;
        sync_gate_unlocked(args)?;
        let task_count = TaskRepository::list_tasks(root.clone())?.len();
        Ok(json!({
            "root": root,
            "pipeline": requested_pipeline,
            "rebuild": rebuild,
            "taskCount": task_count,
        }))
    }
}

fn history_event(record_kind: &str, outcome: Option<&str>) -> String {
    outcome
        .map(|value| value.to_uppercase())
        .unwrap_or_else(|| record_kind.to_string())
}

#[cfg(test)]
#[path = "gate_tool_service_tests.rs"]
mod tests;
