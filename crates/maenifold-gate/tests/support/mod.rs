#![allow(dead_code)]
#![allow(deprecated)]

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn bundled_asset_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(format!("{name}.json"))
}

pub fn read_authoritative_asset(name: &str) -> Value {
    let path = bundled_asset_path(name);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read bundled asset {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse bundled asset {}: {error}", path.display()))
}

pub fn runtime_assets_dir(root: &Path) -> PathBuf {
    root.join(".gate-maenifold-root")
}

pub fn gate_db_path(root: &Path) -> PathBuf {
    root.join(".gate").join("gate.db")
}

pub fn gitignore_path(root: &Path) -> PathBuf {
    root.join(".gitignore")
}

pub fn new_pipeline_root() -> TempDir {
    let temp = tempfile::tempdir().expect("create temp pipeline root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");
    fs::create_dir_all(root.join("done")).expect("create done");
    temp
}

pub fn write_pending_task(root: &Path, task_id: &str) -> PathBuf {
    write_bound_task(root, task_id, "sdd")
}

pub fn write_bound_task(root: &Path, task_id: &str, pipeline: &str) -> PathBuf {
    let path = root.join("todo").join(format!("{task_id}.md"));
    fs::write(
        &path,
        format!(
            "# {task_id} — Auth Login JWT\n\n**Pipeline**: {pipeline}\n\n## Acceptance Criteria\n\n1. `POST /login` with valid credentials returns a signed JWT\n"
        ),
    )
    .expect("write task");
    path
}

pub fn write_legacy_task_without_pipeline_binding(root: &Path, task_id: &str) -> PathBuf {
    let path = root.join("todo").join(format!("{task_id}.md"));
    fs::write(
        &path,
        format!(
            "# {task_id} — Auth Login JWT\n\n## Acceptance Criteria\n\n1. `POST /login` with valid credentials returns a signed JWT\n"
        ),
    )
    .expect("write legacy task");
    path
}

pub fn task_path(root: &Path, task_id: &str) -> PathBuf {
    root.join("todo").join(format!("{task_id}.md"))
}

pub fn done_task_path(root: &Path, task_id: &str) -> PathBuf {
    root.join("done").join(format!("{task_id}.md"))
}

pub fn read_task(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read task {}: {error}", path.display()))
}

pub fn gate_command(root: &Path) -> Command {
    let mut command =
        Command::cargo_bin("gate").expect("gate binary should be present for integration tests");
    command.env("MAENIFOLD_ROOT", runtime_assets_dir(root));
    command
}

pub fn sync_gate(root: &Path) {
    gate_command(root)
        .args(["sync", "--root"])
        .arg(root)
        .arg("--rebuild")
        .assert()
        .success();
}

pub fn status_json(root: &Path) -> Value {
    let output = gate_command(root)
        .args(["status", "--root"])
        .arg(root)
        .arg("--all")
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("status output should be valid JSON")
}
