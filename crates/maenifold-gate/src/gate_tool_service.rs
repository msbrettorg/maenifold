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
        render_task_status(
            &root,
            &TaskRepository::find_task(root.clone(), &task_id)?,
            requested_pipeline.as_deref(),
        )
    }

    pub fn pass(args: OutcomeArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let root = args.context.root.clone();
        let requested_pipeline = args.context.requested_pipeline_name.clone();
        pass_task(args)?;
        render_task_status(
            &root,
            &TaskRepository::find_task(root.clone(), &task_id)?,
            requested_pipeline.as_deref(),
        )
    }

    pub fn fail(args: OutcomeArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let root = args.context.root.clone();
        let requested_pipeline = args.context.requested_pipeline_name.clone();
        fail_task(args)?;
        render_task_status(
            &root,
            &TaskRepository::find_task(root.clone(), &task_id)?,
            requested_pipeline.as_deref(),
        )
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
    let definition =
        GateDefinitionCatalog::resolve_for_task(root, &document.metadata, requested_pipeline_name)?;
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use sha2::{Digest, Sha256};
    use tempfile::TempDir;

    use crate::cli::{GateContext, OutcomeArgs, StatusArgs};

    use super::GateToolService;

    #[test]
    fn mcp_status_tool_returns_task_array() {
        let root = new_pipeline_root();
        write_pending_task(root.path(), "T-1.1");

        let result = GateToolService::status(StatusArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: true,
            },
        })
        .expect("status should succeed");

        let rows = result.as_array().expect("status should return array");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["taskId"], "T-1.1");
        assert_eq!(rows[0]["state"], "pending");
    }

    #[test]
    fn mcp_pass_tool_advances_bound_task() {
        let root = new_pipeline_root();
        write_pending_task(root.path(), "T-1.1");

        let result = GateToolService::pass(OutcomeArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            task_id: "T-1.1".to_string(),
            agent: "swe".to_string(),
            notes: "dispatch".to_string(),
            model: None,
        })
        .expect("pass should succeed");

        assert_eq!(result["taskId"], "T-1.1");
        assert_eq!(result["state"], "red");
    }

    fn new_pipeline_root() -> TempDir {
        let temp = tempfile::tempdir().expect("create temp pipeline root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");
        temp
    }

    fn write_pending_task(root: &Path, task_id: &str) -> PathBuf {
        let path = root.join("todo").join(format!("{task_id}.md"));
        let digest = authoritative_digest("sdd");
        fs::write(
            &path,
            format!(
                "# {task_id} — Auth Login JWT\n\n**Pipeline**: sdd\n**Pipeline Digest**: {digest}\n\n## Acceptance Criteria\n\n1. `POST /login` with valid credentials returns a signed JWT\n"
            ),
        )
        .expect("write task");
        path
    }

    fn authoritative_digest(name: &str) -> String {
        let bytes = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(format!("{name}.json")),
        )
        .expect("read bundled asset");
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }
}
