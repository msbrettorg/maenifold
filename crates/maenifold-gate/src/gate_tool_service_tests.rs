use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::cli::{GateContext, HistoryArgs, OutcomeArgs, StatusArgs, TaskBodyArgs};
use crate::status_filter::StatusFilter;

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
        filter: StatusFilter {
            state: None,
            ready_only: false,
            include_done: false,
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
        filter: StatusFilter {
            state: None,
            ready_only: false,
            include_done: false,
        },
    })
    .expect_err("status should fail on a corrupt cache DB");

    assert!(error.to_string().contains("run `gate sync --rebuild`"));
    assert!(!root.path().join(".gitignore").exists());
    assert!(!gate_dir.join("gate.db-wal").exists());
    assert!(!gate_dir.join("gate.db-shm").exists());
}

#[test]
fn task_body_returns_raw_markdown_for_task() {
    let root = new_pipeline_root();
    let task_path = write_pending_task(root.path(), "T-1.1");

    let result = GateToolService::task_body(TaskBodyArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        task_id: "T-1.1".to_string(),
    })
    .expect("task body should succeed");

    assert_eq!(result["taskId"], "T-1.1");
    assert_eq!(
        result["path"],
        fs::canonicalize(task_path)
            .expect("canonicalize task path")
            .to_string_lossy()
            .as_ref()
    );
    assert!(
        result["markdown"]
            .as_str()
            .expect("markdown string")
            .contains("## Acceptance Criteria")
    );
}

#[cfg(unix)]
#[test]
fn task_body_rejects_symlink_escape_outside_gate_root() {
    let root = new_pipeline_root();
    let escaped = root.path().join("..").join("escaped.md");
    fs::write(&escaped, "# escaped\n").expect("write escaped file");
    let task_path = root.path().join("todo").join("T-1.1.md");
    unix_fs::symlink(&escaped, &task_path).expect("create task symlink");

    let error = GateToolService::task_body(TaskBodyArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        task_id: "T-1.1".to_string(),
    })
    .expect_err("symlink escape should fail");

    assert!(
        error
            .to_string()
            .contains("resolves outside the gate root corpus")
    );
}

#[cfg(unix)]
#[test]
fn history_rejects_symlinked_task_path() {
    let root = new_pipeline_root();
    let target = root.path().join("todo").join("history-target.md");
    fs::write(
        &target,
        "# T-1.2 — History target\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. history should reject symlinks\n",
    )
    .expect("write history target");
    let task_path = root.path().join("todo").join("T-1.2.md");
    unix_fs::symlink(&target, &task_path).expect("create history symlink");

    let error = GateToolService::history(HistoryArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        task_id: "T-1.2".to_string(),
    })
    .expect_err("symlinked history should fail");

    assert!(error.to_string().contains("through a symlink"));
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
