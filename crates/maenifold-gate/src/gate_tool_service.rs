use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::append_command::append_task;
use crate::cache_db::GateCacheDb;
use crate::cancel_command::cancel_task;
use crate::cli::{
    AppendArgs, GateContext, HistoryArgs, OutcomeArgs, StatusArgs, SyncArgs, TaskBodyArgs,
};
use crate::error::GateError;
use crate::fail_command::fail_task;
use crate::pass_command::pass_task;
use crate::status_filter::StatusFilter;
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

    pub fn cancel(args: OutcomeArgs) -> Result<Value, GateError> {
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

fn task_status_from_read_model_unlocked(
    context: &GateContext,
    task_id: &str,
) -> Result<Value, GateError> {
    let rows = status_rows(context)?;
    rows.into_iter()
        .find(|row| row["taskId"].as_str() == Some(task_id))
        .ok_or_else(|| GateError::Cli(format!("task '{task_id}' missing from gate DB read model")))
}

fn status_from_existing_read_model(
    context: &GateContext,
    filter: &StatusFilter,
) -> Result<Value, GateError> {
    let rows = status_rows_read_only(context)?
        .into_iter()
        .filter(|row| filter.matches(row))
        .collect();
    Ok(Value::Array(rows))
}

fn status_rows_read_only(context: &GateContext) -> Result<Vec<Value>, GateError> {
    TaskRepository::list_tasks(context.root.clone())?;
    let db_path = GateCacheDb::path_for_root(&context.root);
    ensure_existing_read_model(&db_path)?;
    load_status_rows(context).map_err(|err| status_read_model_error(&db_path, err))
}

fn status_rows(context: &GateContext) -> Result<Vec<Value>, GateError> {
    TaskRepository::list_tasks(context.root.clone())?;
    ensure_read_model_ready(context)?;
    match load_status_rows(context) {
        Ok(rows) => Ok(rows),
        Err(err) if cache_refreshable(&err) => {
            rebuild_read_model_unlocked(context)?;
            load_status_rows(context)
        }
        Err(err) => Err(err),
    }
}

fn load_status_rows(context: &GateContext) -> Result<Vec<Value>, GateError> {
    let db = GateCacheDb::open(&context.root)?;
    db.list_task_status(&context.root, context.requested_pipeline_name.as_deref())
}

fn ensure_read_model_ready(context: &GateContext) -> Result<(), GateError> {
    if !GateCacheDb::path_for_root(&context.root).exists() {
        refresh_read_model_unlocked(context)?;
    }
    Ok(())
}

fn ensure_existing_read_model(db_path: &Path) -> Result<(), GateError> {
    if db_path.exists() {
        Ok(())
    } else {
        Err(GateError::Cli(format!(
            "gate status requires an existing cache DB at {}; run `gate sync` first",
            db_path.display()
        )))
    }
}

fn refresh_read_model_unlocked(context: &GateContext) -> Result<(), GateError> {
    sync_gate_unlocked(SyncArgs {
        context: GateContext {
            root: context.root.clone(),
            requested_pipeline_name: None,
            json_output: false,
        },
        rebuild: false,
    })
}

fn rebuild_read_model_unlocked(context: &GateContext) -> Result<(), GateError> {
    sync_gate_unlocked(SyncArgs {
        context: GateContext {
            root: context.root.clone(),
            requested_pipeline_name: None,
            json_output: false,
        },
        rebuild: true,
    })
}

fn cache_refreshable(err: &GateError) -> bool {
    matches!(err, GateError::Sqlite(_) | GateError::Json(_))
}

fn status_read_model_error(db_path: &Path, err: GateError) -> GateError {
    if cache_refreshable(&err) {
        GateError::Cli(format!(
            "gate status could not read the cache DB at {}; run `gate sync --rebuild` to refresh it ({err})",
            db_path.display()
        ))
    } else {
        err
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
