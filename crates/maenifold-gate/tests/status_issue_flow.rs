mod support;

use std::fs;

use predicates::prelude::*;
use support::{gate_command, new_pipeline_root};

#[test]
fn sync_fails_on_duplicate_canonical_task_ids_with_both_paths() {
    let root = new_pipeline_root();
    fs::write(
        root.path().join("todo").join("T-90.25-first.md"),
        "---\nid: T-90.25\ntitle: First duplicate\npipeline: sdd\n---\n# First duplicate\n",
    )
    .expect("write first task");
    fs::write(
        root.path().join("done").join("T-90.25-second.md"),
        "---\nid: T-90.25\ntitle: Second duplicate\npipeline: sdd\n---\n# Second duplicate\n",
    )
    .expect("write second task");

    gate_command(root.path())
        .args(["sync", "--root"])
        .arg(root.path())
        .arg("--rebuild")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "duplicate canonical task id 'T-90.25'",
        ))
        .stderr(predicate::str::contains("todo/T-90.25-first.md"))
        .stderr(predicate::str::contains("done/T-90.25-second.md"));
}

#[test]
fn sync_and_plain_status_surface_parse_issue_details() {
    let root = new_pipeline_root();
    fs::write(
        root.path().join("todo").join("T-90.26.md"),
        "---\nid: [\npipeline: sdd\n---\n# Broken frontmatter\n",
    )
    .expect("write malformed task");

    gate_command(root.path())
        .args(["sync", "--root"])
        .arg(root.path())
        .arg("--rebuild")
        .assert()
        .failure()
        .stderr(predicate::str::contains("gate sync found task issues"))
        .stderr(predicate::str::contains("T-90.26.md"))
        .stderr(predicate::str::contains("invalid task frontmatter"));

    gate_command(root.path())
        .args(["status", "--root"])
        .arg(root.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("T-90.26 - error"))
        .stdout(predicate::str::contains("T-90.26.md"))
        .stdout(predicate::str::contains("invalid task frontmatter"));
}
