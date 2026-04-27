use std::fs;
use std::sync::mpsc;
use std::time::Duration;

use tempfile::tempdir;

use super::{TaskFileLock, TaskFileWriter};

#[test]
fn exclusive_lock_blocks_until_first_guard_is_dropped() {
    let root = tempdir().expect("create temp root");
    let first_lock = TaskFileLock::acquire(root.path(), "T-90.24").expect("acquire first lock");
    let root_path = root.path().to_path_buf();
    let (sender, receiver) = mpsc::channel();

    let handle = std::thread::spawn(move || {
        let _second_lock =
            TaskFileLock::acquire(&root_path, "T-90.24").expect("acquire second lock");
        sender.send(()).expect("notify second lock acquired");
    });

    assert!(receiver.recv_timeout(Duration::from_millis(150)).is_err());
    drop(first_lock);
    receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("second lock should acquire after unlock");
    handle.join().expect("join second lock thread");
}

#[test]
fn append_section_replaces_file_without_leaving_temp_files() {
    let root = tempdir().expect("create temp root");
    let task_path = root.path().join("T-90.24.md");
    fs::write(&task_path, "# T-90.24\n").expect("write initial task");

    TaskFileWriter::append_section(&task_path, "## SWE\n\n- complete").expect("append section");

    let text = fs::read_to_string(&task_path).expect("read task");
    assert!(text.contains("## SWE"));

    let entries = fs::read_dir(root.path())
        .expect("read temp dir")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(entries, vec!["T-90.24.md"]);
}
