mod support;

use std::fs;

use support::{gate_command, new_pipeline_root};

#[test]
fn append_finds_slugged_task_by_frontmatter_id() {
    let root = new_pipeline_root();
    let task_path = root
        .path()
        .join("todo")
        .join("T-90.1-gate-sync-illegal-transition.md");
    fs::write(
        &task_path,
        r#"---
id: T-90.1
title: Gate sync illegal transition
pipeline: sdd
---
# T-90.1 — Gate sync illegal transition

## Acceptance Criteria

1. Append resolves frontmatter id on a slugged filename.
"#,
    )
    .expect("write slugged task");

    gate_command(root.path())
        .args(["append", "T-90.1", "--root"])
        .arg(root.path())
        .args(["--agent", "swe", "--notes", "frontmatter lookup regression"])
        .assert()
        .success();

    let updated = fs::read_to_string(&task_path).expect("read updated task");
    assert!(updated.contains("frontmatter lookup regression"));
}

#[test]
fn append_missing_task_error_reports_directories_without_fake_filename() {
    let root = new_pipeline_root();
    let slugged_path = root
        .path()
        .join("todo")
        .join("T-90.1-gate-sync-illegal-transition.md");
    fs::write(
        &slugged_path,
        r#"---
id: T-90.1
title: Gate sync illegal transition
pipeline: sdd
---
# T-90.1 — Gate sync illegal transition

## Acceptance Criteria

1. Missing task errors should mention searched directories.
"#,
    )
    .expect("write slugged task");

    let assert = gate_command(root.path())
        .args(["append", "T-90.19", "--root"])
        .arg(root.path())
        .args(["--agent", "swe", "--notes", "missing task regression"])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(stderr.contains("task 'T-90.19' not found"), "{stderr}");
    assert!(
        stderr.contains(&root.path().join("todo").display().to_string()),
        "{stderr}"
    );
    assert!(
        stderr.contains(&root.path().join("done").display().to_string()),
        "{stderr}"
    );
    assert!(stderr.contains("frontmatter `id:`"), "{stderr}");
    assert!(
        !stderr.contains(
            &root
                .path()
                .join("todo")
                .join("T-90.19.md")
                .display()
                .to_string()
        ),
        "{stderr}"
    );
}
