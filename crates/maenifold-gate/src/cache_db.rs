use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, Transaction};
use serde_json::Value;

use crate::cache_task_row::*;
use crate::error::GateError;
use crate::task_id;
use crate::task_repository::TaskPathSet;

#[derive(Debug)]
pub struct GateCacheDb {
    path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheRefreshMode {
    Incremental,
    Rebuild,
}

impl GateCacheDb {
    pub fn open(root: &Path) -> Result<Self, GateError> {
        Ok(Self {
            path: Self::path_for_root(root),
        })
    }

    pub fn path_for_root(root: &Path) -> PathBuf {
        root.join(".gate").join("gate.db")
    }

    pub fn sync(root: &Path, tasks: &[TaskPathSet]) -> Result<(), GateError> {
        Self::refresh(root, tasks, CacheRefreshMode::Incremental)
    }

    pub fn rebuild(root: &Path, tasks: &[TaskPathSet]) -> Result<(), GateError> {
        Self::refresh(root, tasks, CacheRefreshMode::Rebuild)
    }

    fn refresh(
        root: &Path,
        tasks: &[TaskPathSet],
        mode: CacheRefreshMode,
    ) -> Result<(), GateError> {
        let db = Self::open(root)?;
        let mut connection = db.connection()?;
        let transaction = connection.transaction()?;
        ensure_schema(&transaction)?;
        if mode == CacheRefreshMode::Rebuild {
            clear_task_rows(&transaction)?;
        }
        let mut issues = Vec::new();
        let mut cached_files = BTreeSet::new();

        for task in tasks {
            let cache_key = cache_task_key(root, &task.path)?;
            cached_files.insert(cache_key.clone());
            let row = CachedTaskStatusRow::for_task(root, task, &cache_key);
            if !row.issue.is_empty() {
                issues.push((row.file.clone(), row.issue.clone()));
            }
            upsert_task_row(&transaction, &row)?;
        }

        if mode == CacheRefreshMode::Incremental {
            delete_missing_task_rows(&transaction, &cached_files)?;
        }

        transaction.commit()?;
        if !issues.is_empty() {
            return Err(GateError::Cli(format_task_issues(root, &issues)));
        }
        Ok(())
    }

    pub fn list_task_status(
        &self,
        root: &Path,
        requested_pipeline_name: Option<&str>,
    ) -> Result<Vec<Value>, GateError> {
        let connection = self.read_only_connection()?;
        let mut statement = connection.prepare(
            "SELECT file, task_id, canonical_task_id, title, pipeline, status, location, attempts, stages_json, linked, issue
             FROM tasks
             ORDER BY task_id",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(CachedTaskStatusRow {
                file: row.get(0)?,
                task_id: row.get(1)?,
                canonical_task_id: row.get(2)?,
                title: row.get(3)?,
                pipeline: row.get(4)?,
                status: row.get(5)?,
                location: row.get(6)?,
                attempts: row.get(7)?,
                stages_json: row.get(8)?,
                linked: row.get(9)?,
                issue: row.get(10)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?.into_status_value(root, requested_pipeline_name)?);
        }
        results.sort_by(|left, right| {
            task_id::natural_cmp(
                left["taskId"].as_str().unwrap_or_default(),
                right["taskId"].as_str().unwrap_or_default(),
            )
        });
        validate_unique_cached_task_ids(&results)?;
        Ok(results)
    }

    fn connection(&self) -> Result<Connection, GateError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                GateError::Cli(format!(
                    "cannot create gate cache directory '{}': {error}; ensure the project root is writable or pass --root <writable-path>",
                    parent.display()
                ))
            })?;
        }
        Self::migrate_if_stale(&self.path)?;
        let connection = Connection::open(&self.path)?;
        ensure_wal_mode(&connection, &self.path)?;
        Ok(connection)
    }

    /// gate.db is a pure cache derived from markdown task files.
    /// Additive schema updates must preserve the existing SQLite file; only
    /// pre-v2 incompatible caches are discarded and recreated.
    fn migrate_if_stale(path: &Path) -> Result<(), GateError> {
        const MIN_COMPATIBLE_SCHEMA_VERSION: i32 = 2;
        if !path.exists() {
            return Ok(());
        }
        let conn = Connection::open(path)?;
        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap_or(0);
        drop(conn);
        if version > 0 && version < MIN_COMPATIBLE_SCHEMA_VERSION {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn read_only_connection(&self) -> Result<Connection, GateError> {
        if !self.path.exists() {
            return Err(GateError::Cli(format!(
                "gate cache DB not found at {}; run `gate sync` first",
                self.path.display()
            )));
        }
        Ok(Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?)
    }
}

pub(crate) fn ensure_schema(transaction: &Transaction<'_>) -> Result<(), GateError> {
    transaction.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            file TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            canonical_task_id TEXT,
            title TEXT NOT NULL,
            pipeline TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            location TEXT NOT NULL DEFAULT 'todo' CHECK(location IN ('todo','done')),
            attempts INTEGER NOT NULL DEFAULT 0,
            stages_json TEXT NOT NULL DEFAULT '[]',
            linked INTEGER NOT NULL DEFAULT 0,
            issue TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS work_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file TEXT NOT NULL,
            stage TEXT NOT NULL,
            agent TEXT NOT NULL,
            agent_model TEXT,
            outcome TEXT,
            duration_seconds INTEGER,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;
    ensure_tasks_linked_column(transaction)?;
    transaction.execute_batch("PRAGMA user_version = 3;")?;
    Ok(())
}

fn ensure_tasks_linked_column(transaction: &Transaction<'_>) -> Result<(), GateError> {
    let mut statement = transaction.prepare("PRAGMA table_info(tasks)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_linked = false;
    for column in columns {
        if column? == "linked" {
            has_linked = true;
            break;
        }
    }
    drop(statement);

    if !has_linked {
        transaction.execute(
            "ALTER TABLE tasks ADD COLUMN linked INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    Ok(())
}

fn ensure_wal_mode(connection: &Connection, path: &Path) -> Result<(), GateError> {
    let journal_mode: String =
        connection.query_row("PRAGMA journal_mode = WAL;", [], |row| row.get(0))?;
    if journal_mode.eq_ignore_ascii_case("wal") {
        Ok(())
    } else {
        Err(GateError::Cli(format!(
            "failed to enable SQLite WAL mode for {}: SQLite reported journal_mode={journal_mode}",
            path.display()
        )))
    }
}

#[cfg(test)]
#[path = "cache_db_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cache_db_sync_tests.rs"]
mod sync_tests;
