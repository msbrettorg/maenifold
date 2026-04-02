use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::gate_definition::GateDefinition;
use crate::prd_parser::PrdParser;
use crate::rtm_parser::RtmParser;
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
            path: root.join("gate.db"),
        })
    }

    pub fn rebuild(
        root: &Path,
        _definition: &GateDefinition,
        tasks: &[TaskPathSet],
    ) -> Result<(), GateError> {
        let db = Self::open(root)?;
        let connection = Connection::open(&db.path)?;
        initialize_schema(&connection)?;
        clear_tables(&connection)?;
        insert_requirements(&connection, root)?;
        insert_traceability(&connection, root)?;

        for task in tasks {
            let document = TaskFileReader::read(&task.path)?;
            let definition =
                GateDefinitionCatalog::resolve_for_task(root, &document.metadata, None)?;
            let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;

            connection.execute(
                "INSERT INTO tasks (file, title, requirement, status, location, attempts)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    format!("{}.md", task.task_id),
                    document.metadata.title,
                    document.metadata.requirement,
                    state.state,
                    task.location,
                    i64::from(state.attempts)
                ],
            )?;

            for record in document.records {
                connection.execute(
                    "INSERT INTO work_log (file, stage, agent_model, outcome, notes, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        format!("{}.md", task.task_id),
                        record.stage,
                        record.model,
                        record.outcome,
                        record.record_kind,
                        record.recorded_at
                    ],
                )?;
            }
        }

        Ok(())
    }

    pub fn upsert_task(&self, task: &TaskPathSet) -> Result<(), GateError> {
        let connection = Connection::open(&self.path)?;
        initialize_schema(&connection)?;
        connection.execute(
            "INSERT INTO tasks (file, location) VALUES (?1, ?2)
             ON CONFLICT(file) DO UPDATE SET location = excluded.location, updated_at = datetime('now')",
            params![format!("{}.md", task.task_id), task.location],
        )?;
        Ok(())
    }

    pub fn insert_work_log(&self, entry: &str) -> Result<(), GateError> {
        let connection = Connection::open(&self.path)?;
        initialize_schema(&connection)?;
        connection.execute(
            "INSERT INTO work_log (file, stage, notes, created_at)
             VALUES ('sync', 'sync', ?1, datetime('now'))",
            params![entry],
        )?;
        Ok(())
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), GateError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS requirements (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL CHECK(type IN ('FR','NFR')),
            description TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS traceability (
            test_case TEXT PRIMARY KEY,
            requirement TEXT NOT NULL REFERENCES requirements(id),
            component TEXT NOT NULL,
            test_type TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS tasks (
            file TEXT PRIMARY KEY,
            title TEXT,
            requirement TEXT NOT NULL REFERENCES requirements(id),
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

fn clear_tables(connection: &Connection) -> Result<(), GateError> {
    connection.execute("DELETE FROM work_log", [])?;
    connection.execute("DELETE FROM tasks", [])?;
    connection.execute("DELETE FROM traceability", [])?;
    connection.execute("DELETE FROM requirements", [])?;
    Ok(())
}

fn insert_requirements(connection: &Connection, root: &Path) -> Result<(), GateError> {
    for requirement in PrdParser::parse(&root.join("PRD.md"))? {
        let kind = if requirement.id.starts_with("NFR-") {
            "NFR"
        } else {
            "FR"
        };
        connection.execute(
            "INSERT INTO requirements (id, type, description) VALUES (?1, ?2, ?3)",
            params![requirement.id, kind, requirement.description],
        )?;
    }
    Ok(())
}

fn insert_traceability(connection: &Connection, root: &Path) -> Result<(), GateError> {
    for row in RtmParser::parse(&root.join("RTM.md"))? {
        connection.execute(
            "INSERT INTO traceability (test_case, requirement, component, test_type)
             VALUES (?1, ?2, ?3, ?4)",
            params![row.test_case, row.requirement, row.component, row.test_type],
        )?;
    }
    Ok(())
}
