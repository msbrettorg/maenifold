use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn repo_task_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for location in ["todo", "done"] {
        let directory = root.join(location);
        for entry in fs::read_dir(&directory).expect("read task directory") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|value| value.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            if stem.starts_with("T-") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

fn repo_fixture_root(root: &Path) -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temp repo root");
    for location in ["todo", "done"] {
        let source = root.join(location);
        let destination = temp.path().join(location);
        fs::create_dir_all(&destination).expect("create task directory");
        for entry in fs::read_dir(&source).expect("read task directory") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|value| value.to_str()) != Some("md") {
                continue;
            }
            fs::copy(
                &path,
                destination.join(path.file_name().expect("task file name")),
            )
            .expect("copy task file");
        }
    }
    temp
}

fn gate_status(root: &Path) -> Value {
    let assets_dir = tempfile::tempdir().expect("temp assets dir");
    let fixture_root = repo_fixture_root(root);
    cargo_bin_cmd!("gate")
        .env("MAENIFOLD_GATE_ASSETS_DIR", assets_dir.path())
        .args(["sync", "--root"])
        .arg(fixture_root.path())
        .arg("--rebuild")
        .assert()
        .success();
    let output = cargo_bin_cmd!("gate")
        .env("MAENIFOLD_GATE_ASSETS_DIR", assets_dir.path())
        .args(["status", "--root"])
        .arg(fixture_root.path())
        .arg("--all")
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("valid status json")
}

#[test]
fn status_loads_every_shipped_repo_task() {
    let root = workspace_root();
    let status = gate_status(&root);
    let rows = status.as_array().expect("status array");
    let repo_paths = repo_task_paths(&root);

    assert_eq!(
        rows.len(),
        repo_paths.len(),
        "status should include every shipped T-*.md task file in todo/ and done/"
    );
}

#[test]
fn status_uses_canonical_frontmatter_id_for_repo_task() {
    let root = workspace_root();
    let status = gate_status(&root);
    let rows = status.as_array().expect("status array");

    let task = rows
        .iter()
        .find(|row| row["taskId"] == "T-69.1")
        .expect("expected canonical T-69.1 task id from frontmatter");

    assert!(
        task["path"]
            .as_str()
            .expect("path string")
            .ends_with("done/T-69.1.md")
    );
}

#[test]
fn status_respects_frontmatter_done_state_for_repo_tasks() {
    let root = workspace_root();
    let status = gate_status(&root);
    let rows = status.as_array().expect("status array");

    let task = rows
        .iter()
        .find(|row| row["taskId"] == "T-89.12")
        .expect("expected done task T-89.12");

    assert_eq!(task["pipeline"], "sdd");
    assert_eq!(task["state"], "done");
    assert_eq!(task["location"], "done");
    assert_eq!(task["issue"], "");
}

#[test]
fn sync_rebuild_succeeds_for_shipped_repo_tasks() {
    let root = workspace_root();
    let assets_dir = tempfile::tempdir().expect("temp assets dir");

    cargo_bin_cmd!("gate")
        .env("MAENIFOLD_GATE_ASSETS_DIR", assets_dir.path())
        .args(["sync", "--root"])
        .arg(&root)
        .arg("--rebuild")
        .assert()
        .success();
}
