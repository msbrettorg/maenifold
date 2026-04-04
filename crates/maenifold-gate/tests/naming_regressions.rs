mod support;

use std::fs;
use std::path::Path;

use support::{done_task_path, gate_command, read_task, status_json, write_bound_task};

#[test]
fn status_accepts_legacy_status_frontmatter_and_prefers_state_when_both_exist() {
    let root = support::new_pipeline_root();
    let legacy_task = done_task_path(root.path(), "T-90.12-legacy");
    let canonical_task = done_task_path(root.path(), "T-90.12-canonical");

    fs::write(
        &legacy_task,
        task_with_frontmatter(
            "T-90.12-legacy",
            "Legacy frontmatter state",
            Some("done"),
            None,
        ),
    )
    .expect("write legacy task");
    fs::write(
        &canonical_task,
        task_with_frontmatter(
            "T-90.12-canonical",
            "Canonical frontmatter state",
            Some("pending"),
            Some("done"),
        ),
    )
    .expect("write canonical task");

    sync(root.path());
    let status = status_json(root.path());
    let rows = status.as_array().expect("status array");

    let legacy = rows
        .iter()
        .find(|row| row["taskId"] == "T-90.12-legacy")
        .expect("legacy task present");
    assert_eq!(legacy["state"], "done");

    let canonical = rows
        .iter()
        .find(|row| row["taskId"] == "T-90.12-canonical")
        .expect("canonical task present");
    assert_eq!(canonical["state"], "done");
}

#[test]
fn outcome_sections_use_canonical_team_headings() {
    let root = support::new_pipeline_root();
    let task_id = "T-90.12";
    write_bound_task(root.path(), task_id, "sdd");

    pass(root.path(), task_id, "swe implementation");
    pass(root.path(), task_id, "red review ready");
    pass(root.path(), task_id, "red review approved");
    pass(root.path(), task_id, "blue verification complete");

    let task = read_task(&done_task_path(root.path(), task_id));
    assert!(task.contains("## Red-Team"));
    assert!(task.contains("## Blue-Team"));
    assert!(!task.contains("## RED-Team"));
    assert!(!task.contains("## BLUE-Team"));
}

fn task_with_frontmatter(
    task_id: &str,
    title: &str,
    status: Option<&str>,
    state: Option<&str>,
) -> String {
    let mut frontmatter = vec![
        "---".to_string(),
        format!("id: {task_id}"),
        format!("title: {title}"),
        "pipeline: sdd".to_string(),
    ];
    if let Some(status) = status {
        frontmatter.push(format!("status: {status}"));
    }
    if let Some(state) = state {
        frontmatter.push(format!("state: {state}"));
    }
    frontmatter.push("---".to_string());
    frontmatter.push(String::new());
    frontmatter.push(format!("# {task_id} — {title}"));
    frontmatter.join("\n")
}

fn pass(root: &Path, task_id: &str, notes: &str) {
    gate_command(root)
        .args(["pass", task_id, "--root"])
        .arg(root)
        .args(["--agent", "test-agent"])
        .args(["--notes", notes])
        .assert()
        .success();
}

fn sync(root: &Path) {
    gate_command(root)
        .args(["sync", "--root"])
        .arg(root)
        .assert()
        .success();
}
