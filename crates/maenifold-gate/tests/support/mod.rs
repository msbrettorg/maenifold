#![allow(dead_code)]
#![allow(deprecated)]

use assert_cmd::Command;
use serde_json::Value;
use sha2::{Digest, Sha256};
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

pub fn authoritative_digest(name: &str) -> String {
    let bytes = fs::read(bundled_asset_path(name))
        .unwrap_or_else(|error| panic!("failed to read bundled asset for digest: {error}"));
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn runtime_assets_dir(root: &Path) -> PathBuf {
    root.join(".gate-assets")
}

pub fn new_pipeline_root() -> TempDir {
    let temp = tempfile::tempdir().expect("create temp pipeline root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");
    fs::create_dir_all(root.join("done")).expect("create done");
    fs::write(
        root.join("PRD.md"),
        "# Product Requirements Document\n\n## Functional Requirements\n\n- **FR-1**: System SHALL provide user authentication via JWT tokens\n",
    )
    .expect("write PRD");
    fs::write(
        root.join("RTM.md"),
        "# Requirements Traceability Matrix\n\n| Requirement | Component | Test Case | Type |\n|-------------|-----------|-----------|------|\n| FR-1 | src/auth/ | TC-1: login returns JWT | integration |\n",
    )
    .expect("write RTM");
    temp
}

pub fn write_pending_task(root: &Path, task_id: &str) -> PathBuf {
    write_bound_task(root, task_id, "sdd", &authoritative_digest("sdd"))
}

pub fn write_bound_task(
    root: &Path,
    task_id: &str,
    pipeline: &str,
    pipeline_digest: &str,
) -> PathBuf {
    let path = root.join("todo").join(format!("{task_id}.md"));
    fs::write(
        &path,
        format!(
            "# {task_id} — Auth Login JWT\n\n**Requirement**: FR-1\n**Test Cases**: TC-1\n**Component**: src/auth/\n**Pipeline**: {pipeline}\n**Pipeline Digest**: {pipeline_digest}\n\n## Acceptance Criteria\n\n1. `POST /login` with valid credentials returns a signed JWT\n"
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
            "# {task_id} — Auth Login JWT\n\n**Requirement**: FR-1\n**Test Cases**: TC-1\n**Component**: src/auth/\n\n## Acceptance Criteria\n\n1. `POST /login` with valid credentials returns a signed JWT\n"
        ),
    )
    .expect("write legacy task");
    path
}

pub fn write_task_without_pipeline_digest(root: &Path, task_id: &str, pipeline: &str) -> PathBuf {
    let path = root.join("todo").join(format!("{task_id}.md"));
    fs::write(
        &path,
        format!(
            "# {task_id} — Auth Login JWT\n\n**Requirement**: FR-1\n**Test Cases**: TC-1\n**Component**: src/auth/\n**Pipeline**: {pipeline}\n\n## Acceptance Criteria\n\n1. `POST /login` with valid credentials returns a signed JWT\n"
        ),
    )
    .expect("write task without digest");
    path
}

pub fn write_task_with_mismatched_digest(root: &Path, task_id: &str, pipeline: &str) -> PathBuf {
    write_bound_task(
        root,
        task_id,
        pipeline,
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
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
    command.env("MAENIFOLD_GATE_ASSETS_DIR", runtime_assets_dir(root));
    command
}

pub fn status_json(root: &Path) -> Value {
    let output = gate_command(root)
        .args(["status", "--root"])
        .arg(root)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("status output should be valid JSON")
}
