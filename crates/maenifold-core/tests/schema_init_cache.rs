use maenifold_core::{Config, GraphDatabase};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier, mpsc};
use std::thread;
use std::time::{Duration, Instant};

fn make_config(root: &Path, database_path: PathBuf) -> Config {
    let memory_path = root.join("memory");
    let assets_path = root.join("assets");
    let thinking_path = memory_path.join("thinking");
    let sequential_path = thinking_path.join("sequential");

    Config {
        maenifold_root: root.to_path_buf(),
        memory_path,
        assets_path,
        thinking_path,
        sequential_path,
        database_path,
        mcp_allowed_hosts: Vec::new(),
        mcp_allowed_origins: Vec::new(),
        settings: Default::default(),
    }
}

fn table_exists(database_path: &Path, table_name: &str) -> bool {
    let connection = Connection::open(database_path).expect("open sqlite database");
    let mut statement = connection
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1")
        .expect("prepare sqlite_master query");
    let count: i64 = statement
        .query_row([table_name], |row| row.get(0))
        .expect("query sqlite_master");
    count == 1
}

#[test]
fn init_is_idempotent_for_same_path() {
    let root = tempfile::tempdir().expect("create temp dir");
    let config = make_config(root.path(), root.path().join("memory.db"));

    let first = GraphDatabase::initialize_database(&config);
    assert!(first.is_ok(), "first init failed: {first:?}");

    let second = GraphDatabase::initialize_database(&config);
    assert!(second.is_ok(), "second init failed: {second:?}");
}

#[test]
fn init_caches_per_database_path() {
    let root = tempfile::tempdir().expect("create temp dir");
    let first_config = make_config(root.path(), root.path().join("first.db"));
    let second_config = make_config(root.path(), root.path().join("second.db"));

    let first = GraphDatabase::initialize_database(&first_config);
    assert!(first.is_ok(), "first path init failed: {first:?}");

    let second = GraphDatabase::initialize_database(&second_config);
    assert!(second.is_ok(), "second path init failed: {second:?}");

    assert!(
        table_exists(&first_config.database_path, "file_content"),
        "file_content missing from first database"
    );
    assert!(
        table_exists(&second_config.database_path, "file_content"),
        "file_content missing from second database"
    );
}

#[test]
fn concurrent_init_does_not_wedge() {
    let root = tempfile::tempdir().expect("create temp dir");
    let config = Arc::new(make_config(root.path(), root.path().join("memory.db")));
    let barrier = Arc::new(Barrier::new(11));
    let (sender, receiver) = mpsc::channel();
    let mut handles = Vec::new();

    for _ in 0..10 {
        let config = Arc::clone(&config);
        let barrier = Arc::clone(&barrier);
        let sender = sender.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            let result = GraphDatabase::initialize_database(&config);
            sender.send(result).expect("send init result");
        }));
    }
    drop(sender);

    let started_at = Instant::now();
    barrier.wait();

    for _ in 0..10 {
        let remaining = Duration::from_secs(5)
            .checked_sub(started_at.elapsed())
            .unwrap_or(Duration::ZERO);
        assert!(
            remaining > Duration::ZERO,
            "concurrent init exceeded 5 seconds"
        );

        let result = receiver
            .recv_timeout(remaining)
            .expect("receive init result before timeout");
        assert!(result.is_ok(), "concurrent init failed: {result:?}");
    }

    for handle in handles {
        handle.join().expect("join init thread");
    }
}
