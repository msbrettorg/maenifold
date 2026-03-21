use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
pub struct GateDb {
    conn: Connection,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRow {
    pub file: String,
    pub title: Option<String>,
    pub requirement: String,
    pub status: String,
    pub location: String,
    pub attempts: i32,
}
impl GateDb {
    pub fn open(path: &Path) -> Result<GateDb> {
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open SQLite database at {}", path.display()))?;
        enable_foreign_keys(&conn)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("failed to enable WAL mode")?;
        let db = GateDb { conn };
        db.ensure_schema()?;
        Ok(db)
    }
    pub fn open_memory() -> Result<GateDb> {
        let conn =
            Connection::open_in_memory().context("failed to open in-memory SQLite database")?;
        enable_foreign_keys(&conn)?;
        let db = GateDb { conn };
        db.ensure_schema()?;
        Ok(db)
    }
    pub fn ensure_schema(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS tasks (
                    file TEXT PRIMARY KEY,
                    title TEXT,
                    requirement TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','red','blue','done','stuck')),
                    location TEXT NOT NULL DEFAULT 'todo' CHECK(location IN ('todo','done')),
                    attempts INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE TABLE IF NOT EXISTS work_log (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file TEXT NOT NULL REFERENCES tasks(file),
                    stage TEXT NOT NULL CHECK(stage IN ('swe','red','blue')),
                    agent_model TEXT,
                    verdict TEXT,
                    duration_seconds INTEGER,
                    notes TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
                CREATE INDEX IF NOT EXISTS idx_tasks_requirement ON tasks(requirement);
                CREATE INDEX IF NOT EXISTS idx_work_log_file ON work_log(file);",
            )
            .context("failed to ensure gate cache schema")
    }
    pub fn upsert_task(
        &self,
        file: &str,
        title: Option<&str>,
        requirement: &str,
        status: &str,
        location: &str,
        attempts: i32,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO tasks
                 (file, title, requirement, status, location, attempts, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
                params![file, title, requirement, status, location, attempts],
            )
            .with_context(|| format!("failed to upsert task '{file}'"))?;
        Ok(())
    }
    pub fn insert_work_log(
        &self,
        file: &str,
        stage: &str,
        agent_model: Option<&str>,
        verdict: Option<&str>,
        duration_secs: Option<i64>,
        notes: Option<&str>,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO work_log
                 (file, stage, agent_model, verdict, duration_seconds, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![file, stage, agent_model, verdict, duration_secs, notes],
            )
            .with_context(|| format!("failed to insert work log for '{file}'"))?;
        Ok(())
    }
    pub fn get_task(&self, file: &str) -> Result<Option<TaskRow>> {
        self.conn
            .query_row(
                "SELECT file, title, requirement, status, location, attempts
                 FROM tasks WHERE file = ?1",
                [file],
                map_task_row,
            )
            .optional()
            .with_context(|| format!("failed to fetch task '{file}'"))
    }
    pub fn get_all_tasks(&self) -> Result<Vec<TaskRow>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT file, title, requirement, status, location, attempts
                 FROM tasks ORDER BY file",
            )
            .context("failed to prepare task list query")?;
        let rows = stmt
            .query_map([], map_task_row)
            .context("failed to query tasks")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to decode task rows")?;
        Ok(rows)
    }
    pub fn delete_all_tasks(&self) -> Result<usize> {
        let logs = self
            .conn
            .execute("DELETE FROM work_log", [])
            .context("failed to delete work logs")?;
        let tasks = self
            .conn
            .execute("DELETE FROM tasks", [])
            .context("failed to delete tasks")?;
        Ok(tasks + logs)
    }
}
fn enable_foreign_keys(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("failed to enable foreign key enforcement")
}
pub fn map_task_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        file: row.get(0)?,
        title: row.get(1)?,
        requirement: row.get(2)?,
        status: row.get(3)?,
        location: row.get(4)?,
        attempts: row.get(5)?,
    })
}
#[cfg(test)]
pub mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use rusqlite::Connection;

    use super::{GateDb, TaskRow};
    #[test]
    fn ensure_schema_is_idempotent() {
        let db = GateDb::open_memory().unwrap();
        db.ensure_schema().unwrap();
    }
    #[test]
    fn open_memory_enables_foreign_keys() {
        let db = GateDb::open_memory().unwrap();
        let enabled = db
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .unwrap();
        assert_eq!(enabled, 1);
    }
    #[test]
    fn open_handles_existing_schema_variants() {
        let path = temp_db_path("schema-variant");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE requirements (id TEXT PRIMARY KEY);
             CREATE TABLE tasks (
                 file TEXT PRIMARY KEY,
                 title TEXT,
                 requirement TEXT NOT NULL REFERENCES requirements(id),
                 status TEXT NOT NULL DEFAULT 'pending'
                     CHECK(status IN ('pending','red','blue','done','stuck')),
                 location TEXT NOT NULL DEFAULT 'todo'
                     CHECK(location IN ('todo','done')),
                 attempts INTEGER NOT NULL DEFAULT 0,
                 created_at TEXT NOT NULL DEFAULT (datetime('now')),
                 updated_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE work_log (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 file TEXT NOT NULL REFERENCES tasks(file),
                 stage TEXT NOT NULL CHECK(stage IN ('swe','red','blue')),
                 agent_model TEXT,
                 verdict TEXT,
                 duration_seconds INTEGER,
                 notes TEXT,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );",
        )
        .unwrap();
        drop(conn);
        let db = GateDb::open(&path).unwrap();
        assert!(db.get_all_tasks().unwrap().is_empty());
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn upsert_task_inserts_and_updates() {
        let db = GateDb::open_memory().unwrap();
        db.upsert_task("b.md", Some("First"), "FR-41", "pending", "todo", 0)
            .unwrap();
        db.upsert_task("b.md", Some("Updated"), "FR-41", "red", "todo", 2)
            .unwrap();
        assert_eq!(
            db.get_task("b.md").unwrap(),
            Some(TaskRow {
                file: "b.md".into(),
                title: Some("Updated".into()),
                requirement: "FR-41".into(),
                status: "red".into(),
                location: "todo".into(),
                attempts: 2
            })
        );
    }
    #[test]
    fn insert_work_log_appends_entries() {
        let db = GateDb::open_memory().unwrap();
        db.upsert_task("a.md", None, "FR-41", "pending", "todo", 0)
            .unwrap();
        db.insert_work_log("a.md", "swe", Some("gpt-5.4"), None, Some(3), Some("first"))
            .unwrap();
        db.insert_work_log(
            "a.md",
            "red",
            Some("gpt-5.4"),
            Some("PASS"),
            Some(4),
            Some("second"),
        )
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
        db.upsert_task("a.md", None, "FR-41", "pending", "todo", 0)
            .unwrap();
        assert!(db.get_task("a.md").unwrap().is_some());
    }
    #[test]
    fn get_all_tasks_returns_sorted_rows() {
        let db = GateDb::open_memory().unwrap();
        db.upsert_task("b.md", None, "FR-41", "pending", "todo", 0)
            .unwrap();
        db.upsert_task("a.md", Some("Alpha"), "FR-42", "done", "done", 1)
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
        db.upsert_task("a.md", None, "FR-41", "pending", "todo", 0)
            .unwrap();
        db.insert_work_log("a.md", "swe", None, None, None, None)
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
    fn temp_db_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("maenifold-gate-{label}-{unique}.db"))
    }
}
