use super::{GateDb, TaskRow, TaskUpsert, WorkLogEntry, extract_task_id};
use rusqlite::Connection;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn extract_id_from_standard_name() {
    assert_eq!(extract_task_id("T-55.1-foo.md"), "T-55.1");
}

#[test]
fn extract_id_from_dotless_name() {
    assert_eq!(extract_task_id("T-55-foo.md"), "T-55");
}

#[test]
fn extract_id_from_non_task_name() {
    assert_eq!(extract_task_id("some-file.md"), "some-file");
}

#[test]
fn extract_id_empty_string() {
    assert_eq!(extract_task_id(""), "");
}

#[test]
fn extract_id_dot_md_only() {
    assert_eq!(extract_task_id(".md"), ".md");
}

macro_rules! task_upsert {
    ($id:expr, $file:expr, $title:expr, $requirement:expr, $status:expr, $location:expr, $attempts:expr $(,)?) => {
        TaskUpsert {
            id: $id,
            file: $file,
            title: $title,
            requirement: $requirement,
            status: $status,
            location: $location,
            attempts: $attempts,
        }
    };
}

macro_rules! work_log {
    ($file:expr, $stage:expr, $agent_model:expr, $verdict:expr, $duration_secs:expr, $notes:expr $(,)?) => {
        WorkLogEntry {
            file: $file,
            stage: $stage,
            agent_model: $agent_model,
            verdict: $verdict,
            duration_secs: $duration_secs,
            notes: $notes,
        }
    };
}

#[test]
fn extract_id_no_extension() {
    assert_eq!(extract_task_id("T-55.1-foo"), "T-55.1");
}

#[test]
fn extract_id_triple_version() {
    assert_eq!(extract_task_id("T-1.2.3-foo.md"), "T-1.2.3");
}

#[test]
fn extract_task_id_trailing_alpha() {
    assert_eq!(extract_task_id("T-44.5b-foo.md"), "T-44.5b");
    assert_eq!(extract_task_id("T-44.5bc-foo.md"), "T-44.5bc");
    assert_eq!(extract_task_id("T-44.5-foo.md"), "T-44.5");
}

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

#[test]
fn upsert_task_inserts_and_updates() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "b",
        "b.md",
        Some("First"),
        "FR-41",
        "pending",
        "todo",
        0,
    ))
    .unwrap();
    db.upsert_task(&task_upsert!(
        "b",
        "b.md",
        Some("Updated"),
        "FR-41",
        "red",
        "todo",
        2,
    ))
    .unwrap();
    assert_eq!(
        db.get_task("b.md").unwrap(),
        Some(TaskRow {
            id: "b".into(),
            file: "b.md".into(),
            title: Some("Updated".into()),
            requirement: "FR-41".into(),
            status: "red".into(),
            location: "todo".into(),
            attempts: 2,
        })
    );
}

#[test]
fn upsert_rejects_id_collision() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "T-55.1",
        "T-55.1-foo.md",
        Some("First"),
        "FR-41",
        "pending",
        "todo",
        0,
    ))
    .unwrap();

    let err = db
        .upsert_task(&task_upsert!(
            "T-55.1",
            "T-55.1-bar.md",
            Some("Second"),
            "FR-42",
            "red",
            "todo",
            1,
        ))
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "task ID 'T-55.1' is already used by 'T-55.1-foo.md', cannot assign to 'T-55.1-bar.md'"
    );
    assert_eq!(
        db.get_task("T-55.1-foo.md").unwrap(),
        Some(TaskRow {
            id: "T-55.1".into(),
            file: "T-55.1-foo.md".into(),
            title: Some("First".into()),
            requirement: "FR-41".into(),
            status: "pending".into(),
            location: "todo".into(),
            attempts: 0,
        })
    );
    assert_eq!(db.get_task("T-55.1-bar.md").unwrap(), None);
}
#[test]
fn insert_work_log_appends_entries() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "a", "a.md", None, "FR-41", "pending", "todo", 0,
    ))
    .unwrap();
    db.insert_work_log(&work_log!(
        "a.md",
        "swe",
        Some("gpt-5.4"),
        None,
        Some(3),
        Some("first"),
    ))
    .unwrap();
    db.insert_work_log(&work_log!(
        "a.md",
        "red",
        Some("gpt-5.4"),
        Some("PASS"),
        Some(4),
        Some("second"),
    ))
    .unwrap();
    assert_eq!(
        db.conn
            .query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM work_log WHERE file = 'a.md'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
        2
    );
}
#[test]
fn get_task_handles_missing_and_existing() {
    let db = GateDb::open_memory().unwrap();
    assert_eq!(db.get_task("missing.md").unwrap(), None);
    db.upsert_task(&task_upsert!(
        "a", "a.md", None, "FR-41", "pending", "todo", 0,
    ))
    .unwrap();
    assert!(db.get_task("a.md").unwrap().is_some());
}
#[test]
fn get_task_by_id_returns_none_for_missing() {
    assert_eq!(
        GateDb::open_memory()
            .unwrap()
            .get_task_by_id("T-55.2")
            .unwrap(),
        None
    );
}
#[test]
fn get_task_by_id_finds_inserted_task() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "T-55.2",
        "T-55.2-db-id-column.md",
        None,
        "FR-42",
        "pending",
        "todo",
        0,
    ))
    .unwrap();
    assert_eq!(
        db.get_task_by_id("T-55.2").unwrap().map(|row| row.file),
        Some("T-55.2-db-id-column.md".into())
    );
}
#[test]
fn resolve_id_to_file_works() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "T-55.2",
        "T-55.2-db-id-column.md",
        None,
        "FR-42",
        "pending",
        "todo",
        0,
    ))
    .unwrap();
    assert_eq!(
        db.resolve_id_to_file("T-55.2").unwrap(),
        Some("T-55.2-db-id-column.md".into())
    );
}
#[test]
fn get_all_tasks_returns_sorted_rows() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "b", "b.md", None, "FR-41", "pending", "todo", 0,
    ))
    .unwrap();
    db.upsert_task(&task_upsert!(
        "a",
        "a.md",
        Some("Alpha"),
        "FR-42",
        "done",
        "done",
        1,
    ))
    .unwrap();
    assert_eq!(
        db.get_all_tasks()
            .unwrap()
            .into_iter()
            .map(|row| row.file)
            .collect::<Vec<_>>(),
        vec!["a.md", "b.md"]
    );
}
#[test]
fn delete_all_tasks_clears_tasks_and_logs() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "a", "a.md", None, "FR-41", "pending", "todo", 0,
    ))
    .unwrap();
    db.insert_work_log(&work_log!("a.md", "swe", None, None, None, None))
        .unwrap();
    assert_eq!(db.delete_all_tasks().unwrap(), 2);
    assert!(db.get_all_tasks().unwrap().is_empty());
    assert_eq!(
        db.conn
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM work_log", [], |r| r.get(0))
            .unwrap(),
        0
    );
}
#[test]
fn update_task_status_changes_status() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "a", "a.md", None, "FR-41", "pending", "todo", 0,
    ))
    .unwrap();
    db.update_task_status("a.md", "red", "todo").unwrap();
    assert_eq!(db.get_task("a.md").unwrap().unwrap().status, "red");
}
#[test]
fn increment_attempts_bumps_counter() {
    let db = GateDb::open_memory().unwrap();
    db.upsert_task(&task_upsert!(
        "a", "a.md", None, "FR-41", "pending", "todo", 2,
    ))
    .unwrap();
    db.increment_attempts("a.md").unwrap();
    assert_eq!(db.get_task("a.md").unwrap().unwrap().attempts, 3);
}

fn temp_db_path(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("maenifold-gate-{label}-{unique}.db"))
}
