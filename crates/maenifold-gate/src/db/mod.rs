use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};

mod migration;

const TASK_SELECT: &str =
    "SELECT id, file, title, requirement, status, location, attempts FROM tasks";

pub struct GateDb {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRow {
    pub id: String,
    pub file: String,
    pub title: Option<String>,
    pub requirement: String,
    pub status: String,
    pub location: String,
    pub attempts: i32,
}

pub struct TaskUpsert<'a> {
    pub id: &'a str,
    pub file: &'a str,
    pub title: Option<&'a str>,
    pub requirement: &'a str,
    pub status: &'a str,
    pub location: &'a str,
    pub attempts: i32,
}

pub struct WorkLogEntry<'a> {
    pub file: &'a str,
    pub stage: &'a str,
    pub agent_model: Option<&'a str>,
    pub verdict: Option<&'a str>,
    pub duration_secs: Option<i64>,
    pub notes: Option<&'a str>,
}

/// Extracts task ID from filename.
/// "T-55.1-pipeline-definition-schema.md" → "T-55.1"
/// "some-other-file.md" → "some-other-file"
pub fn extract_task_id(filename: &str) -> String {
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    if stem.is_empty() {
        return filename.to_string();
    }
    if !stem.starts_with("T-") {
        return stem.to_string();
    }
    let bytes = stem.as_bytes();
    let (mut end, mut saw_digit) = (2, false);
    while let Some(&byte) = bytes.get(end) {
        match byte {
            b'0'..=b'9' => saw_digit = true,
            b'.' if saw_digit => {}
            _ => break,
        }
        end += 1;
    }
    // Include trailing alpha suffix (e.g., T-44.5b)
    while let Some(&byte) = bytes.get(end) {
        if byte.is_ascii_alphabetic() {
            end += 1;
        } else {
            break;
        }
    }
    if saw_digit {
        stem[..end].trim_end_matches('.').to_string()
    } else {
        stem.to_string()
    }
}

impl GateDb {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open SQLite database at {}", path.display()))?;
        enable_foreign_keys(&conn)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("failed to enable WAL mode")?;
        let db = Self { conn };
        db.ensure_schema()?;
        Ok(db)
    }

    pub fn open_memory() -> Result<Self> {
        let conn =
            Connection::open_in_memory().context("failed to open in-memory SQLite database")?;
        enable_foreign_keys(&conn)?;
        let db = Self { conn };
        db.ensure_schema()?;
        Ok(db)
    }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn
            .execute_batch(
                // CHECK constraints on status/stage are intentionally omitted to support
                // arbitrary pipeline stage names. Existing databases with old constraints
                // may reject novel stage names — use `gate sync --rebuild` to recreate.
                "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT UNIQUE NOT NULL,
                file TEXT PRIMARY KEY,
                title TEXT,
                requirement TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                location TEXT NOT NULL DEFAULT 'todo' CHECK(location IN ('todo','done')),
                attempts INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
             CREATE TABLE IF NOT EXISTS work_log (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 file TEXT NOT NULL REFERENCES tasks(file),
                 stage TEXT NOT NULL,
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
            .context("failed to ensure gate cache schema")?;
        migration::ensure_task_ids(&self.conn)
    }

    pub fn upsert_task(&self, task: &TaskUpsert<'_>) -> Result<()> {
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT file FROM tasks WHERE id = ?1 AND file != ?2",
                params![task.id, task.file],
                |row| row.get(0),
            )
            .optional()
            .with_context(|| {
                format!(
                    "failed to validate task id '{}' for '{}'",
                    task.id, task.file
                )
            })?;
        if let Some(other_file) = existing {
            bail!(
                "task ID '{}' is already used by '{}', cannot assign to '{}'",
                task.id,
                other_file,
                task.file
            );
        }
        self.conn
            .execute(
                "INSERT INTO tasks
             (id, file, title, requirement, status, location, attempts, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))
             ON CONFLICT(file) DO UPDATE SET
                 id = excluded.id,
                 title = excluded.title,
                 requirement = excluded.requirement,
                 status = excluded.status,
                 location = excluded.location,
                 attempts = excluded.attempts,
                 updated_at = datetime('now')",
                params![
                    task.id,
                    task.file,
                    task.title,
                    task.requirement,
                    task.status,
                    task.location,
                    task.attempts
                ],
            )
            .with_context(|| format!("failed to upsert task '{}'", task.file))?;
        Ok(())
    }

    pub fn insert_work_log(&self, entry: &WorkLogEntry<'_>) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO work_log (file, stage, agent_model, verdict, duration_seconds, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    entry.file,
                    entry.stage,
                    entry.agent_model,
                    entry.verdict,
                    entry.duration_secs,
                    entry.notes
                ],
            )
            .with_context(|| format!("failed to insert work log for '{}'", entry.file))?;
        Ok(())
    }

    pub fn increment_attempts(&self, file: &str) -> Result<()> {
        ensure_changed(
            self.conn.execute(
                "UPDATE tasks SET attempts = attempts + 1, updated_at = datetime('now') WHERE file = ?1",
                [file],
            ),
            format!("failed to increment attempts for '{file}'"),
            format!("task '{file}' not found"),
        )
    }

    pub fn get_task(&self, file: &str) -> Result<Option<TaskRow>> {
        self.conn
            .query_row(
                &format!("{TASK_SELECT} WHERE file = ?1"),
                [file],
                map_task_row,
            )
            .optional()
            .with_context(|| format!("failed to fetch task '{file}'"))
    }

    pub fn get_task_by_id(&self, id: &str) -> Result<Option<TaskRow>> {
        self.conn
            .query_row(&format!("{TASK_SELECT} WHERE id = ?1"), [id], map_task_row)
            .optional()
            .with_context(|| format!("failed to fetch task id '{id}'"))
    }

    pub fn resolve_id_to_file(&self, id: &str) -> Result<Option<String>> {
        self.conn
            .query_row("SELECT file FROM tasks WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .optional()
            .with_context(|| format!("failed to resolve task id '{id}'"))
    }

    pub fn get_all_tasks(&self) -> Result<Vec<TaskRow>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{TASK_SELECT} ORDER BY file"))
            .context("failed to prepare task list query")?;
        stmt.query_map([], map_task_row)
            .context("failed to query tasks")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to decode task rows")
    }

    pub fn update_task_status(&self, file: &str, status: &str, location: &str) -> Result<()> {
        let changed = self
            .conn
            .execute(
                "UPDATE tasks
                 SET status = ?1, location = ?2, updated_at = datetime('now')
                 WHERE file = ?3",
                params![status, location, file],
            )
            .with_context(|| format!("failed to update task '{file}'"))?;
        if changed == 0 {
            bail!("unknown task file '{file}'");
        }
        Ok(())
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

fn ensure_changed(
    changed: rusqlite::Result<usize>,
    context: String,
    missing: String,
) -> Result<()> {
    match changed.with_context(|| context)? {
        0 => bail!(missing),
        _ => Ok(()),
    }
}

pub fn map_task_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        id: row.get(0)?,
        file: row.get(1)?,
        title: row.get(2)?,
        requirement: row.get(3)?,
        status: row.get(4)?,
        location: row.get(5)?,
        attempts: row.get(6)?,
    })
}

#[cfg(test)]
#[path = "../db_tests.rs"]
mod tests;
