mod support;

use std::path::Path;

use support::{done_task_path, gate_command, read_task, write_bound_task};

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

fn pass(root: &Path, task_id: &str, notes: &str) {
    gate_command(root)
        .args(["pass", task_id, "--root"])
        .arg(root)
        .args(["--agent", "test-agent"])
        .args(["--notes", notes])
        .assert()
        .success();
}
