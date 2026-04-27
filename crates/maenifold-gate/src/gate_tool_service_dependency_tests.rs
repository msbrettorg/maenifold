use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::cli::{GateContext, StatusArgs};
use crate::status_filter::StatusFilter;

use super::super::GateToolService;

#[test]
fn status_enriches_dependency_fields_and_blocked_filter() {
    let root = new_pipeline_root();
    write_frontmatter_task(root.path(), "done", "T-1", "done", "");
    write_frontmatter_task(root.path(), "todo", "T-2", "pending", "depends_on: T-1\n");
    write_frontmatter_task(root.path(), "todo", "T-3", "pending", "depends_on: T-2\n");
    write_frontmatter_task(
        root.path(),
        "todo",
        "T-4",
        "pending",
        "depends_on: T-missing\n",
    );

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
    assert!(rows
        .iter()
        .any(|row| row["taskId"] == "T-3" && row["blockedBy"] == ["T-2"]));
    assert!(rows
        .iter()
        .any(|row| row["taskId"] == "T-4" && row["missingDependencies"] == ["T-missing"]));

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
