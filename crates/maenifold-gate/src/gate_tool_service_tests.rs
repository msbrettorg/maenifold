use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::thread;

use tempfile::TempDir;

use crate::cli::{GateContext, HistoryArgs, OutcomeArgs, StatusArgs, TaskBodyArgs};
use crate::status_filter::StatusFilter;
use crate::sync_command::sync_gate;

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
            blocked_only: false,
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
            blocked_only: false,
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

#[test]
fn concurrent_mcp_fail_and_cli_pass_do_not_record_same_stage_conflict() {
    let root = new_pipeline_root();
    write_blue_team_task(root.path(), "T-3.1");
    sync_gate(crate::cli::SyncArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        rebuild: true,
    })
    .expect("initial sync should succeed");

    let barrier = Arc::new(Barrier::new(2));
    let cli_root = root.path().to_path_buf();
    let cli_barrier = Arc::clone(&barrier);
    let cli_pass = thread::spawn(move || {
        cli_barrier.wait();
        Command::new(gate_bin())
            .args(["pass", "T-3.1", "--root"])
            .arg(&cli_root)
            .args(["--agent", "cli", "--notes", "cli pass"])
            .output()
            .expect("run gate pass")
    });

    barrier.wait();
    let _ = GateToolService::fail(OutcomeArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        task_id: "T-3.1".to_string(),
        agent: "mcp".to_string(),
        notes: "mcp fail".to_string(),
        model: None,
    });
    let _ = cli_pass.join().expect("cli pass thread should join");

    let task_text = fs::read_to_string(root.path().join("todo").join("T-3.1.md"))
        .or_else(|_| fs::read_to_string(root.path().join("done").join("T-3.1.md")))
        .expect("task should remain in todo or done");
    let blue_pass = task_text.matches("- Stage ID: blue-team").count();
    assert_eq!(
        blue_pass, 1,
        "only one blue-team outcome should be recorded: {task_text}"
    );
}

#[test]
fn sync_rebuild_recovers_terminal_todo_task_after_crash_window() {
    let root = new_pipeline_root();
    let path = root.path().join("todo").join("T-4.1.md");
    fs::write(
        &path,
        "# T-4.1 — Crash recovery\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. recover terminal task\n\n## Pending\n\n- Stage ID: pending\n- Record Kind: outcome\n- Agent: pm\n- Recorded At: 2026-01-01T00:00:00Z\n- Outcome: PASS\n- Notes: dispatch\n\n## SWE\n\n- Stage ID: swe\n- Record Kind: outcome\n- Agent: swe\n- Recorded At: 2026-01-01T00:00:01Z\n- Outcome: PASS\n- Notes: built\n\n## Red-Team\n\n- Stage ID: red-team\n- Record Kind: outcome\n- Agent: red\n- Recorded At: 2026-01-01T00:00:02Z\n- Outcome: PASS\n- Notes: audited\n\n## Blue-Team\n\n- Stage ID: blue-team\n- Record Kind: outcome\n- Agent: blue\n- Recorded At: 2026-01-01T00:00:03Z\n- Outcome: PASS\n- Notes: verified\n",
    )
    .expect("write terminal task stranded in todo");

    sync_gate(crate::cli::SyncArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        rebuild: true,
    })
    .expect("rebuild should recover stranded terminal task");

    assert!(!path.exists(), "stranded terminal task should leave todo/");
    assert!(
        root.path().join("done").join("T-4.1.md").exists(),
        "terminal task should be recovered into done/"
    );
}

#[test]
fn status_enriches_dependency_fields_and_blocked_filter() {
    let root = new_pipeline_root();
    write_frontmatter_task(root.path(), "done", "T-1", "done", "");
    write_frontmatter_task(root.path(), "todo", "T-2", "pending", "depends_on: T-1\n");
    write_frontmatter_task(root.path(), "todo", "T-3", "pending", "depends_on: T-2\n");
    write_frontmatter_task(root.path(), "todo", "T-4", "pending", "depends_on: T-missing\n");

    GateToolService::sync(crate::cli::SyncArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        rebuild: true,
    })
    .expect("sync should succeed");

    let status = GateToolService::status(StatusArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        filter: StatusFilter {
            state: None,
            ready_only: false,
            blocked_only: true,
            include_done: false,
        },
    })
    .expect("status should succeed");
    let rows = status.as_array().expect("status array");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row["taskId"] == "T-3" && row["blockedBy"] == ["T-2"]));
    assert!(rows.iter().any(
        |row| row["taskId"] == "T-4" && row["missingDependencies"] == ["T-missing"]
    ));

    let ready = GateToolService::status(StatusArgs {
        context: GateContext {
            root: root.path().to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        filter: StatusFilter {
            state: None,
            ready_only: true,
            blocked_only: false,
            include_done: false,
        },
    })
    .expect("ready status should succeed");
    let ready_rows = ready.as_array().expect("ready array");
    assert_eq!(ready_rows.len(), 1);
    assert_eq!(ready_rows[0]["taskId"], "T-2");
    assert_eq!(ready_rows[0]["dependsOn"], ["T-1"]);
}

fn new_pipeline_root() -> TempDir {
    let temp = tempfile::tempdir().expect("create temp pipeline root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");
    fs::create_dir_all(root.join("done")).expect("create done");
    temp
}

fn gate_bin() -> PathBuf {
    std::env::current_exe()
        .expect("current test binary path")
        .parent()
        .expect("deps directory")
        .parent()
        .expect("debug directory")
        .join("gate")
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

fn write_blue_team_task(root: &Path, task_id: &str) -> PathBuf {
    let path = root.join("todo").join(format!("{task_id}.md"));
    fs::write(
        &path,
        format!(
            "# {task_id} — Ready for final pass\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. final pass moves to done\n\n## Pending\n\n- Stage ID: pending\n- Record Kind: outcome\n- Agent: pm\n- Recorded At: 2026-01-01T00:00:00Z\n- Outcome: PASS\n- Notes: dispatch\n\n## SWE\n\n- Stage ID: swe\n- Record Kind: outcome\n- Agent: swe\n- Recorded At: 2026-01-01T00:00:01Z\n- Outcome: PASS\n- Notes: built\n\n## Red-Team\n\n- Stage ID: red-team\n- Record Kind: outcome\n- Agent: red\n- Recorded At: 2026-01-01T00:00:02Z\n- Outcome: PASS\n- Notes: audited\n"
        ),
    )
    .expect("write blue-team task");
    path
}

fn write_frontmatter_task(
    root: &Path,
    location: &str,
    task_id: &str,
    state: &str,
    extra_frontmatter: &str,
) -> PathBuf {
    let path = root.join(location).join(format!("{task_id}.md"));
    fs::write(
        &path,
        format!(
            "---\nid: {task_id}\ntitle: {task_id}\npipeline: sdd\nstate: {state}\n{extra_frontmatter}---\n# {task_id}\n"
        ),
    )
    .expect("write frontmatter task");
    path
}
