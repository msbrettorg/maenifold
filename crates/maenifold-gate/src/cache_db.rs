use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, Transaction, params};
use serde_json::{Value, json};

use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::task_file::TaskFileReader;
use crate::task_id;
use crate::task_metadata::is_linked_session_id;
use crate::task_repository::TaskPathSet;
use crate::task_status::TaskStatusDeriver;

#[derive(Debug)]
pub struct GateCacheDb {
    path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheRefreshMode {
    Incremental,
    Rebuild,
}

#[derive(Debug)]
struct CachedTaskStatusRow {
    file: String,
    task_id: String,
    canonical_task_id: Option<String>,
    title: String,
    pipeline: Option<String>,
    status: String,
    location: String,
    attempts: u32,
    stages_json: String,
    linked: bool,
    issue: String,
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
            fs::create_dir_all(parent)?;
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

impl CachedTaskStatusRow {
    fn for_task(root: &Path, task: &TaskPathSet, cache_key: &str) -> Self {
        Self::load_success(root, task, cache_key)
            .unwrap_or_else(|err| Self::error(task, cache_key, err.to_string()))
    }

    fn load_success(root: &Path, task: &TaskPathSet, cache_key: &str) -> Result<Self, GateError> {
        let document = TaskFileReader::read(&task.path)?;
        let definition = GateDefinitionCatalog::resolve_for_task(root, &document.metadata, None)?;
        let effective = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
        let stages_json = serde_json::to_string(&definition.ordered_stage_ids()?)?;

        Ok(Self {
            file: cache_key.to_string(),
            task_id: task.task_id.clone(),
            canonical_task_id: document.metadata.id,
            title: document.metadata.title,
            pipeline: Some(document.metadata.pipeline),
            status: effective.state,
            location: task.location.clone(),
            attempts: effective.attempts,
            stages_json,
            linked: is_linked_session_id(document.metadata.session_id.as_deref()),
            issue: String::new(),
        })
    }

    fn error(task: &TaskPathSet, cache_key: &str, issue: String) -> Self {
        Self {
            file: cache_key.to_string(),
            task_id: task.task_id.clone(),
            canonical_task_id: None,
            title: task.task_id.clone(),
            pipeline: None,
            status: "error".to_string(),
            location: task.location.clone(),
            attempts: 0,
            stages_json: "[]".to_string(),
            linked: false,
            issue,
        }
    }

    fn into_status_value(
        self,
        root: &Path,
        requested_pipeline_name: Option<&str>,
    ) -> Result<Value, GateError> {
        if self.issue.is_empty()
            && let Some(requested) = requested_pipeline_name
            && self.pipeline.as_deref() != Some(requested)
        {
            return Ok(pipeline_mismatch_row(
                root,
                &self.file,
                &self.task_id,
                &self.location,
                self.linked,
                requested,
                self.pipeline.as_deref(),
            ));
        }

        if !self.issue.is_empty() {
            return Ok(error_status_row(
                root,
                &self.file,
                &self.task_id,
                &self.location,
                self.linked,
                &self.issue,
            ));
        }

        let stages: Vec<String> = serde_json::from_str(&self.stages_json)?;
        Ok(json!({
            "taskId": self.task_id,
            "canonicalTaskId": self.canonical_task_id,
            "title": self.title,
            "pipeline": self.pipeline,
            "state": self.status,
            "stages": stages,
            "attempts": self.attempts,
            "location": self.location,
            "path": root.join(&self.file),
            "linked": self.linked,
            "issue": self.issue,
        }))
    }
}

fn cache_task_key(root: &Path, task_path: &Path) -> Result<String, GateError> {
    let relative = task_path.strip_prefix(root).map_err(|_| {
        GateError::Cli(format!(
            "task path {} is not under root {}",
            task_path.display(),
            root.display()
        ))
    })?;
    Ok(relative.to_string_lossy().into_owned())
}

fn upsert_task_row(
    transaction: &Transaction<'_>,
    row: &CachedTaskStatusRow,
) -> Result<(), GateError> {
    transaction.execute(
        "INSERT INTO tasks (
            file, task_id, canonical_task_id, title, pipeline, status, location, attempts, stages_json, linked, issue
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(file) DO UPDATE SET
            task_id = excluded.task_id,
            canonical_task_id = excluded.canonical_task_id,
            title = excluded.title,
            pipeline = excluded.pipeline,
            status = excluded.status,
            location = excluded.location,
            attempts = excluded.attempts,
            stages_json = excluded.stages_json,
            linked = excluded.linked,
            issue = excluded.issue,
            updated_at = datetime('now')",
        params![
            &row.file,
            &row.task_id,
            row.canonical_task_id.as_deref(),
            &row.title,
            row.pipeline.as_deref(),
            &row.status,
            &row.location,
            row.attempts,
            &row.stages_json,
            row.linked,
            &row.issue,
        ],
    )?;
    Ok(())
}

fn delete_missing_task_rows(
    transaction: &Transaction<'_>,
    cached_files: &BTreeSet<String>,
) -> Result<(), GateError> {
    let existing_files = {
        let mut statement = transaction.prepare("SELECT file FROM tasks")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }
        files
    };

    for file in existing_files {
        if !cached_files.contains(&file) {
            transaction.execute("DELETE FROM tasks WHERE file = ?1", params![file])?;
        }
    }

    Ok(())
}

fn clear_task_rows(transaction: &Transaction<'_>) -> Result<(), GateError> {
    transaction.execute("DELETE FROM tasks", [])?;
    Ok(())
}

fn error_status_row(
    root: &Path,
    file: &str,
    task_id: &str,
    location: &str,
    linked: bool,
    issue: &str,
) -> Value {
    json!({
        "taskId": task_id,
        "title": task_id,
        "pipeline": Value::Null,
        "state": "error",
        "stages": [],
        "attempts": 0,
        "location": location,
        "path": root.join(file),
        "linked": linked,
        "issue": issue,
    })
}

#[allow(clippy::too_many_arguments)]
fn pipeline_mismatch_row(
    root: &Path,
    file: &str,
    task_id: &str,
    location: &str,
    linked: bool,
    requested: &str,
    bound: Option<&str>,
) -> Value {
    error_status_row(
        root,
        file,
        task_id,
        location,
        linked,
        &GateError::PipelineOverrideMismatch {
            requested: requested.to_string(),
            bound: bound.unwrap_or("unbound").to_string(),
        }
        .to_string(),
    )
}

fn ensure_schema(transaction: &Transaction<'_>) -> Result<(), GateError> {
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

fn format_task_issues(root: &Path, issues: &[(String, String)]) -> String {
    let details = issues
        .iter()
        .map(|(file, issue)| format!("- {}: {issue}", root.join(file).display()))
        .collect::<Vec<_>>()
        .join("\n");
    format!("gate sync found task issues:\n{details}")
}

fn validate_unique_cached_task_ids(rows: &[Value]) -> Result<(), GateError> {
    let mut seen_task_ids: BTreeMap<String, PathBuf> = BTreeMap::new();
    for row in rows {
        let Some(task_id) = row["taskId"].as_str() else {
            continue;
        };
        let Some(path) = row["path"].as_str() else {
            continue;
        };
        let path = PathBuf::from(path);
        if let Some(previous_path) = seen_task_ids.insert(task_id.to_string(), path.clone()) {
            return Err(GateError::DuplicateCanonicalTaskId {
                task_id: task_id.to_string(),
                first_path: previous_path,
                second_path: path,
            });
        }
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
mod tests {
    use std::fs;

    use rusqlite::params;
    use tempfile::tempdir;

    use super::{GateCacheDb, ensure_schema};
    use crate::error::GateError;
    use crate::task_repository::TaskPathSet;

    #[test]
    fn gate_cache_connections_enable_wal_mode() {
        let temp = tempfile::tempdir().expect("create temp root");
        let db = GateCacheDb::open(temp.path()).expect("open cache db");
        let connection = db.connection().expect("open sqlite connection");
        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .expect("read journal mode");
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    }

    #[test]
    fn rebuild_preserves_work_log_history() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        let db = GateCacheDb::open(root).expect("open cache db");
        GateCacheDb::rebuild(root, &[]).expect("create cache db schema");
        let connection = db.connection().expect("open sqlite connection");
        connection
            .execute(
                "INSERT INTO work_log (file, stage, agent, notes, created_at)
                 VALUES ('sync', 'sync', 'gate', 'before rebuild', datetime('now'))",
                [],
            )
            .expect("insert work log entry");

        GateCacheDb::rebuild(root, &[]).expect("rebuild cache db");

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM work_log", [], |row| row.get(0))
            .expect("count work log rows");
        assert_eq!(count, 1);
    }

    #[test]
    fn incremental_sync_preserves_task_row_identity_while_rebuild_replaces_it() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        let task_path = root.join("todo").join("T-90.15.md");
        fs::write(
            &task_path,
            "# T-90.15 — Cache refresh\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. sync refreshes read model\n",
        )
        .expect("write task");

        let task = TaskPathSet {
            task_id: "T-90.15".to_string(),
            path: task_path.clone(),
            location: "todo".to_string(),
        };

        GateCacheDb::rebuild(root, &[task.clone()]).expect("seed cache");
        let db = GateCacheDb::open(root).expect("open cache db");
        let connection = db.connection().expect("open sqlite connection");
        let (original_title, original_created_at): (String, String) = connection
            .query_row(
                "SELECT title, created_at FROM tasks WHERE file = 'todo/T-90.15.md'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read seeded row");

        connection
            .execute(
                "UPDATE tasks SET title = 'stale title', created_at = '2000-01-01 00:00:00'
                 WHERE file = 'todo/T-90.15.md'",
                [],
            )
            .expect("mutate cached row");

        GateCacheDb::sync(root, &[task.clone()]).expect("incremental sync");
        let incremental = connection
            .query_row(
                "SELECT title, created_at FROM tasks WHERE file = 'todo/T-90.15.md'",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("read synced row");
        assert_eq!(incremental.0, original_title);
        assert_eq!(incremental.1, "2000-01-01 00:00:00");

        GateCacheDb::rebuild(root, &[task]).expect("rebuild cache");
        let rebuilt = connection
            .query_row(
                "SELECT title, created_at FROM tasks WHERE file = 'todo/T-90.15.md'",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("read rebuilt row");
        assert_eq!(rebuilt.0, original_title);
        assert_ne!(rebuilt.1, "2000-01-01 00:00:00");
        assert!(rebuilt.1 >= original_created_at);
    }

    #[test]
    fn rebuild_reports_task_issue_details_after_caching_rows() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        let task_path = root.join("todo").join("T-90.26.md");
        fs::write(
            &task_path,
            "---\nid: [\npipeline: sdd\n---\n# Broken frontmatter\n",
        )
        .expect("write malformed task");

        let error = GateCacheDb::rebuild(
            root,
            &[TaskPathSet {
                task_id: "T-90.26".to_string(),
                path: task_path,
                location: "todo".to_string(),
            }],
        )
        .expect_err("malformed task should be surfaced");
        let message = error.to_string();
        assert!(message.contains("gate sync found task issues"));
        assert!(message.contains("T-90.26.md"));
        assert!(message.contains("invalid task frontmatter"));

        let rows = GateCacheDb::open(root)
            .expect("open db")
            .list_task_status(root, None)
            .expect("load cached status");
        assert_eq!(rows[0]["state"], "error");
        assert!(
            rows[0]["issue"]
                .as_str()
                .expect("issue string")
                .contains("invalid task frontmatter")
        );
        assert_eq!(rows[0]["linked"], false);
    }

    #[test]
    fn rebuild_caches_linked_status_leniently() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");

        let linked_path = root.join("todo").join("T-90.27.md");
        fs::write(
            &linked_path,
            "---\ntitle: Linked task\npipeline: sdd\nsessionId: session-1775916753112-57359\n---\n# Linked task\n",
        )
        .expect("write linked task");

        let malformed_path = root.join("todo").join("T-90.28.md");
        fs::write(
            &malformed_path,
            "---\ntitle: Malformed linkage\npipeline: sdd\nsessionId:\n  - malformed\n---\n# Malformed linkage\n",
        )
        .expect("write malformed task");

        GateCacheDb::rebuild(
            root,
            &[
                TaskPathSet {
                    task_id: "T-90.27".to_string(),
                    path: linked_path,
                    location: "todo".to_string(),
                },
                TaskPathSet {
                    task_id: "T-90.28".to_string(),
                    path: malformed_path,
                    location: "todo".to_string(),
                },
            ],
        )
        .expect("rebuild cache db");

        let rows = GateCacheDb::open(root)
            .expect("open db")
            .list_task_status(root, None)
            .expect("load cached status");
        assert_eq!(rows[0]["linked"], true);
        assert_eq!(rows[1]["linked"], false);
    }

    #[test]
    fn list_task_status_rejects_duplicate_cached_task_ids() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        let db = GateCacheDb::open(root).expect("open cache db");
        let mut connection = db.connection().expect("open sqlite connection");
        let transaction = connection.transaction().expect("create transaction");
        ensure_schema(&transaction).expect("ensure schema");
        transaction
            .execute(
                "INSERT INTO tasks (
                    file, task_id, canonical_task_id, title, pipeline, status, location, attempts, stages_json, issue
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "todo/T-90.25-a.md",
                    "T-90.25",
                    "T-90.25",
                    "First duplicate",
                    "sdd",
                    "pending",
                    "todo",
                    0,
                    "[]",
                    ""
                ],
            )
            .expect("insert first task row");
        transaction
            .execute(
                "INSERT INTO tasks (
                    file, task_id, canonical_task_id, title, pipeline, status, location, attempts, stages_json, issue
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "done/T-90.25-b.md",
                    "T-90.25",
                    "T-90.25",
                    "Second duplicate",
                    "sdd",
                    "done",
                    "done",
                    0,
                    "[]",
                    ""
                ],
            )
            .expect("insert second task row");
        transaction.commit().expect("commit rows");

        let error = db
            .list_task_status(root, None)
            .expect_err("duplicate cached ids should fail");
        match error {
            GateError::DuplicateCanonicalTaskId {
                task_id,
                first_path,
                second_path,
            } => {
                assert_eq!(task_id, "T-90.25");
                assert!(first_path.ends_with("todo/T-90.25-a.md"));
                assert!(second_path.ends_with("done/T-90.25-b.md"));
            }
            other => panic!("expected duplicate canonical task id error, got {other}"),
        }
    }
}
