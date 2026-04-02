use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::gate_definition::GateDefinition;
use crate::task_file::TaskFileReader;
use crate::task_repository::TaskPathSet;
use crate::task_status::TaskStatusDeriver;

#[derive(Debug)]
pub struct GateCacheDb {
    path: PathBuf,
}

impl GateCacheDb {
    pub fn open(root: &Path) -> Result<Self, GateError> {
        Ok(Self {
            path: root.join(".gate").join("gate.db"),
        })
    }

    pub fn rebuild(
        root: &Path,
        _definition: &GateDefinition,
        tasks: &[TaskPathSet],
    ) -> Result<(), GateError> {
        let db = Self::open(root)?;
        db.reset_file()?;
        let connection = db.connection()?;
        create_schema(&connection)?;

        for task in tasks {
            let document = TaskFileReader::read(&task.path)?;
            let definition =
                GateDefinitionCatalog::resolve_for_task(root, &document.metadata, None)?;
            let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;

            connection.execute(
                "INSERT INTO tasks (file, title, pipeline, status, location, attempts)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    format!("{}.md", task.task_id),
                    document.metadata.title,
                    document.metadata.pipeline,
                    state.state,
                    task.location,
                    i64::from(state.attempts)
                ],
            )?;

            for record in document.records {
                connection.execute(
                    "INSERT INTO work_log (file, stage, agent, agent_model, outcome, notes, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        format!("{}.md", task.task_id),
                        record.stage,
                        record.agent,
                        record.model,
                        record.outcome,
                        record.notes,
                        record.recorded_at
                    ],
                )?;
            }
        }

        Ok(())
    }
    pub fn insert_work_log(&self, entry: &str) -> Result<(), GateError> {
        let connection = self.connection()?;
        ensure_current_schema(&connection)?;
        connection.execute(
            "INSERT INTO work_log (file, stage, agent, notes, created_at)
             VALUES ('sync', 'sync', 'gate', ?1, datetime('now'))",
            params![entry],
        )?;
        Ok(())
    }

    fn connection(&self) -> Result<Connection, GateError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Connection::open(&self.path)?)
    }

    fn reset_file(&self) -> Result<(), GateError> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

fn create_schema(connection: &Connection) -> Result<(), GateError> {
    connection.execute_batch(
        "
        CREATE TABLE tasks (
            file TEXT PRIMARY KEY,
            title TEXT,
            pipeline TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            location TEXT NOT NULL DEFAULT 'todo' CHECK(location IN ('todo','done')),
            attempts INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE work_log (
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
    Ok(())
}

fn ensure_current_schema(connection: &Connection) -> Result<(), GateError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            file TEXT PRIMARY KEY,
            title TEXT,
            pipeline TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            location TEXT NOT NULL DEFAULT 'todo' CHECK(location IN ('todo','done')),
            attempts INTEGER NOT NULL DEFAULT 0,
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
    Ok(())
}
