use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::thread;

use tempfile::TempDir;

#[test]
fn concurrent_cli_status_during_rebuild_returns_valid_json() {
    let root = new_pipeline_root();
    for index in 0..75 {
        write_pending_task(root.path(), &format!("T-2.{index}"));
    }
    run_gate(["sync", "--rebuild"], root.path());

    let barrier = Arc::new(Barrier::new(2));
    let rebuild_root = root.path().to_path_buf();
    let rebuild_barrier = Arc::clone(&barrier);
    let rebuild = thread::spawn(move || {
        rebuild_barrier.wait();
        gate_command(&["sync", "--rebuild"], &rebuild_root)
            .output()
            .expect("run gate sync --rebuild")
    });

    barrier.wait();
    for _ in 0..10 {
        let output = gate_command(&["status", "--json"], root.path())
            .output()
            .expect("run gate status --json");
        assert!(
            output.status.success(),
            "status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout)
            .expect("status output should be valid JSON");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("database is locked"),
            "status stderr should not report SQLite lock errors"
        );
    }

    let rebuild_output = rebuild.join().expect("rebuild thread should join");
    assert!(
        rebuild_output.status.success(),
        "rebuild failed: {}",
        String::from_utf8_lossy(&rebuild_output.stderr)
    );
}

fn new_pipeline_root() -> TempDir {
    let temp = tempfile::tempdir().expect("create temp pipeline root");
    fs::create_dir_all(temp.path().join("todo")).expect("create todo");
    fs::create_dir_all(temp.path().join("done")).expect("create done");
    temp
}

fn write_pending_task(root: &Path, task_id: &str) {
    fs::write(
        root.join("todo").join(format!("{task_id}.md")),
        format!(
            "# {task_id} — Auth Login JWT\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. valid login returns a signed JWT\n"
        ),
    )
    .expect("write task");
}

fn run_gate<const N: usize>(args: [&str; N], root: &Path) {
    let output = gate_command(&args, root).output().expect("run gate");
    assert!(
        output.status.success(),
        "gate command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn gate_command(args: &[&str], root: &Path) -> Command {
    let mut command = Command::new(gate_bin());
    command.args(args).arg("--root").arg(root);
    command
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
