use serde_json::{Value, json};

use crate::append_command::append_task;
use crate::cli::{AppendArgs, OutcomeArgs, StatusArgs, SyncArgs};
use crate::error::GateError;
use crate::fail_command::fail_task;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::pass_command::pass_task;
use crate::sync_command::sync_gate;
use crate::task_file::TaskFileReader;
use crate::task_repository::{TaskPathSet, TaskRepository};
use crate::task_status::TaskStatusDeriver;

#[derive(Debug, Clone)]
pub struct GateToolService;

impl GateToolService {
    pub fn status(args: StatusArgs) -> Result<Value, GateError> {
        let tasks = TaskRepository::list_tasks(args.context.root.clone())?;
        let mut values = Vec::with_capacity(tasks.len());

        for task in tasks {
            values.push(render_task_status(
                &args.context.root,
                &task,
                args.context.requested_pipeline_name.as_deref(),
            )?);
        }

        Ok(Value::Array(values))
    }

    pub fn append(args: AppendArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let root = args.context.root.clone();
        let requested_pipeline = args.context.requested_pipeline_name.clone();
        append_task(args)?;
        render_task_status(&root, &TaskRepository::find_task(root.clone(), &task_id)?, requested_pipeline.as_deref())
    }

    pub fn pass(args: OutcomeArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let root = args.context.root.clone();
        let requested_pipeline = args.context.requested_pipeline_name.clone();
        pass_task(args)?;
        render_task_status(&root, &TaskRepository::find_task(root.clone(), &task_id)?, requested_pipeline.as_deref())
    }

    pub fn fail(args: OutcomeArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let root = args.context.root.clone();
        let requested_pipeline = args.context.requested_pipeline_name.clone();
        fail_task(args)?;
        render_task_status(&root, &TaskRepository::find_task(root.clone(), &task_id)?, requested_pipeline.as_deref())
    }

    pub fn sync(args: SyncArgs) -> Result<Value, GateError> {
        let root = args.context.root.clone();
        let requested_pipeline = args.context.requested_pipeline_name.clone();
        let rebuild = args.rebuild;
        sync_gate(args)?;
        let tasks = TaskRepository::list_tasks(root.clone())?;
        Ok(json!({
            "root": root,
            "pipeline": requested_pipeline,
            "rebuild": rebuild,
            "taskCount": tasks.len(),
        }))
    }
}

fn render_task_status(
    root: &std::path::Path,
    task: &TaskPathSet,
    requested_pipeline_name: Option<&str>,
) -> Result<Value, GateError> {
    let document = TaskFileReader::read(&task.path)?;
    let definition = GateDefinitionCatalog::resolve_for_task(root, &document.metadata, requested_pipeline_name)?;
    let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
    Ok(json!({
        "taskId": task.task_id,
        "title": document.metadata.title,
        "pipeline": document.metadata.pipeline,
        "pipelineDigest": document.metadata.pipeline_digest,
        "status": state.state,
        "state": state.state,
        "attempts": state.attempts,
        "location": state.location,
        "path": task.path,
    }))
}
