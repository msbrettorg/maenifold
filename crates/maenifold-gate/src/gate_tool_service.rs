use std::path::Path;

use serde_json::{Value, json};

use crate::append_command::append_task;
use crate::cache_db::GateCacheDb;
use crate::cli::{AppendArgs, GateContext, OutcomeArgs, StatusArgs, SyncArgs};
use crate::error::GateError;
use crate::fail_command::fail_task;
use crate::pass_command::pass_task;
use crate::sync_command::sync_gate;
use crate::task_repository::TaskRepository;

#[derive(Debug, Clone)]
pub struct GateToolService;

impl GateToolService {
    pub fn status(args: StatusArgs) -> Result<Value, GateError> {
        status_from_existing_read_model(&args.context)
    }

    pub fn append(args: AppendArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let context = args.context.clone();
        append_task(args)?;
        refresh_read_model(&context)?;
        task_status_from_read_model(&context, &task_id)
    }

    pub fn pass(args: OutcomeArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let context = args.context.clone();
        pass_task(args)?;
        refresh_read_model(&context)?;
        task_status_from_read_model(&context, &task_id)
    }

    pub fn fail(args: OutcomeArgs) -> Result<Value, GateError> {
        let task_id = args.task_id.clone();
        let context = args.context.clone();
        fail_task(args)?;
        refresh_read_model(&context)?;
        task_status_from_read_model(&context, &task_id)
    }

    pub fn sync(args: SyncArgs) -> Result<Value, GateError> {
        let root = args.context.root.clone();
        let requested_pipeline = args.context.requested_pipeline_name.clone();
        let rebuild = args.rebuild;
        sync_gate(args)?;
        let task_count = status_rows(&GateContext {
            root: root.clone(),
            requested_pipeline_name: None,
            json_output: false,
        })?
        .len();
        Ok(json!({
            "root": root,
            "pipeline": requested_pipeline,
            "rebuild": rebuild,
            "taskCount": task_count,
        }))
    }
}

fn task_status_from_read_model(context: &GateContext, task_id: &str) -> Result<Value, GateError> {
    let rows = status_rows(context)?;
    rows.into_iter()
        .find(|row| row["taskId"].as_str() == Some(task_id))
        .ok_or_else(|| GateError::Cli(format!("task '{task_id}' missing from gate DB read model")))
}

fn status_from_existing_read_model(context: &GateContext) -> Result<Value, GateError> {
    Ok(Value::Array(status_rows_read_only(context)?))
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
            rebuild_read_model(context)?;
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
        refresh_read_model(context)?;
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

fn refresh_read_model(context: &GateContext) -> Result<(), GateError> {
    sync_gate(SyncArgs {
        context: GateContext {
            root: context.root.clone(),
            requested_pipeline_name: None,
            json_output: false,
        },
        rebuild: false,
    })
}

fn rebuild_read_model(context: &GateContext) -> Result<(), GateError> {
    sync_gate(SyncArgs {
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use crate::cli::{GateContext, OutcomeArgs, StatusArgs};

    use super::GateToolService;

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
        assert_eq!(result["state"], "swe");
    }

    #[test]
    fn status_requires_existing_gate_db_without_creating_repo_files() {
        let root = new_pipeline_root();
        write_pending_task(root.path(), "T-1.1");

        let error = GateToolService::status(StatusArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
        })
        .expect_err("status should fail when the cache DB is missing");

        assert!(error.to_string().contains("run `gate sync` first"));
        assert!(!root.path().join(".gate").exists());
        assert!(!root.path().join(".gitignore").exists());
    }

    #[test]
    fn status_on_corrupt_gate_db_returns_error_without_refreshing_repo() {
        let root = new_pipeline_root();
        write_pending_task(root.path(), "T-1.1");

        let gate_dir = root.path().join(".gate");
        fs::create_dir_all(&gate_dir).expect("create .gate");
        fs::write(gate_dir.join("gate.db"), "not a sqlite database").expect("write corrupt db");

        let error = GateToolService::status(StatusArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
        })
        .expect_err("status should fail on a corrupt cache DB");

        assert!(error.to_string().contains("run `gate sync --rebuild`"));
        assert!(!root.path().join(".gitignore").exists());
        assert!(!gate_dir.join("gate.db-wal").exists());
        assert!(!gate_dir.join("gate.db-shm").exists());
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
        fs::write(
            &path,
            format!(
                "# {task_id} — Auth Login JWT\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. `POST /login` with valid credentials returns a signed JWT\n"
            ),
        )
        .expect("write task");
        path
    }
}
