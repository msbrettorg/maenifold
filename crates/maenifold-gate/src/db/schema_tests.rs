use super::GateDb;
use rusqlite::Connection;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn ensure_schema_is_idempotent() {
    GateDb::open_memory().unwrap().ensure_schema().unwrap();
}

#[test]
fn open_memory_enables_foreign_keys() {
    let db = GateDb::open_memory().unwrap();
    assert_eq!(
        db.conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn open_handles_existing_schema_variants() {
    let path = temp_db_path("schema-variant");
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE requirements (id TEXT PRIMARY KEY);
             CREATE TABLE tasks (
                 file TEXT PRIMARY KEY,
                 title TEXT,
                 requirement TEXT NOT NULL REFERENCES requirements(id),
                 status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','red','blue','done','stuck')),
                 location TEXT NOT NULL DEFAULT 'todo' CHECK(location IN ('todo','done')),
                 attempts INTEGER NOT NULL DEFAULT 0,
                 created_at TEXT NOT NULL DEFAULT (datetime('now')),
                 updated_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE work_log (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 file TEXT NOT NULL REFERENCES tasks(file),
                 stage TEXT NOT NULL CHECK(stage IN ('swe','red','blue')),
                 agent_model TEXT, verdict TEXT, duration_seconds INTEGER, notes TEXT,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );",
        )
        .unwrap();
    let db = GateDb::open(&path).unwrap();
    assert!(db.get_all_tasks().unwrap().is_empty());
    fs::remove_file(path).unwrap();
}

fn temp_db_path(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("maenifold-gate-{label}-{unique}.db"))
}
