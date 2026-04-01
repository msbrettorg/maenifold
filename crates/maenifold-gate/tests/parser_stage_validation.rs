use std::fs;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use maenifold_gate::{
    TaskLocation,
    db::{GateDb, extract_task_id},
};
use tempfile::{TempDir, tempdir};

fn setup_root() -> TempDir {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("todo")).unwrap();
    fs::create_dir_all(dir.path().join("done")).unwrap();
    dir
}

fn gate_command() -> Command {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_gate") {
        Command::new(path)
    } else {
        cargo_bin_cmd!("gate")
    }
}

fn task_file_name(n: u32) -> String {
    format!("T-199.{n}-parser-stage-validation.md")
}

fn write_task(root: &std::path::Path, location: TaskLocation, file: &str, content: &str) {
    fs::write(root.join(location.as_str()).join(file), content).unwrap();
}

#[test]
fn parser_warns_on_unknown_section_stage_during_sync() {
    let root = setup_root();
    let file = task_file_name(1);
    let content = "\
---\n\
id: T-199.1\n\
title: Parser Stage Validation\n\
status: pending\n\
pipeline: gsd\n\
---\n\
\n\
# T-199.1 — Parser Stage Validation\n\
\n\
**Requirement**: FR-199\n\
\n\
## SWE Notes\n\
\n\
Wrong pipeline stage.\n";
    write_task(root.path(), TaskLocation::Todo, &file, content);

    let assert = gate_command()
        .args(["sync", "--rebuild"])
        .arg(root.path())
        .assert()
        .success();
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("section heading 'SWE Notes'"),
        "expected invalid section-stage warning in stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("pipeline 'gsd'"),
        "expected pipeline name in stderr, got: {stderr}"
    );
    assert!(
        GateDb::open(&root.path().join("gate.db"))
            .unwrap()
            .get_task_by_id(&extract_task_id(&file))
            .unwrap()
            .is_none()
    );
}
