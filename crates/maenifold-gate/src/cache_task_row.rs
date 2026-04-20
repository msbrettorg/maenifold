use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rusqlite::{Transaction, params};
use serde_json::{Value, json};

use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::task_metadata::is_linked_session_id;
use crate::task_repository::{TaskPathSet, TaskRepository};
use crate::task_status::TaskStatusDeriver;

#[derive(Debug)]
pub(crate) struct CachedTaskStatusRow {
    pub(crate) file: String,
    pub(crate) task_id: String,
    pub(crate) canonical_task_id: Option<String>,
    pub(crate) title: String,
    pub(crate) pipeline: Option<String>,
    pub(crate) status: String,
    pub(crate) location: String,
    pub(crate) attempts: u32,
    pub(crate) stages_json: String,
    pub(crate) linked: bool,
    pub(crate) issue: String,
}

impl CachedTaskStatusRow {
    pub(crate) fn for_task(root: &Path, task: &TaskPathSet, cache_key: &str) -> Self {
        Self::load_success(root, task, cache_key)
            .unwrap_or_else(|err| Self::error(task, cache_key, err.to_string()))
    }

    pub(crate) fn load_success(
        root: &Path,
        task: &TaskPathSet,
        cache_key: &str,
    ) -> Result<Self, GateError> {
        let document = TaskRepository::read_task_document(root, task)?;
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

    pub(crate) fn error(task: &TaskPathSet, cache_key: &str, issue: String) -> Self {
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

    pub(crate) fn into_status_value(
        self,
        root: &Path,
        requested_pipeline_name: Option<&str>,
    ) -> Result<Value, GateError> {
        let ctx = ErrorRowContext {
            root,
            file: &self.file,
            task_id: &self.task_id,
            location: &self.location,
            linked: self.linked,
        };
        if self.issue.is_empty()
            && let Some(requested) = requested_pipeline_name
            && self.pipeline.as_deref() != Some(requested)
        {
            return Ok(pipeline_mismatch_row(&ctx, requested, self.pipeline.as_deref()));
        }

        if !self.issue.is_empty() {
            return Ok(error_status_row(&ctx, &self.issue));
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

pub(crate) fn cache_task_key(root: &Path, task_path: &Path) -> Result<String, GateError> {
    let relative = task_path.strip_prefix(root).map_err(|_| {
        GateError::Cli(format!(
            "task path {} is not under root {}",
            task_path.display(),
            root.display()
        ))
    })?;
    Ok(relative.to_string_lossy().into_owned())
}

pub(crate) fn upsert_task_row(
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

pub(crate) fn delete_missing_task_rows(
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

pub(crate) fn clear_task_rows(transaction: &Transaction<'_>) -> Result<(), GateError> {
    transaction.execute("DELETE FROM tasks", [])?;
    Ok(())
}

pub(crate) struct ErrorRowContext<'a> {
    pub root: &'a Path,
    pub file: &'a str,
    pub task_id: &'a str,
    pub location: &'a str,
    pub linked: bool,
}

pub(crate) fn error_status_row(ctx: &ErrorRowContext<'_>, issue: &str) -> Value {
    json!({
        "taskId": ctx.task_id,
        "title": ctx.task_id,
        "pipeline": Value::Null,
        "state": "error",
        "stages": [],
        "attempts": 0,
        "location": ctx.location,
        "path": ctx.root.join(ctx.file),
        "linked": ctx.linked,
        "issue": issue,
    })
}

pub(crate) fn pipeline_mismatch_row(
    ctx: &ErrorRowContext<'_>,
    requested: &str,
    bound: Option<&str>,
) -> Value {
    error_status_row(
        ctx,
        &GateError::PipelineOverrideMismatch {
            requested: requested.to_string(),
            bound: bound.unwrap_or("unbound").to_string(),
        }
        .to_string(),
    )
}

pub(crate) fn format_task_issues(root: &Path, issues: &[(String, String)]) -> String {
    let details = issues
        .iter()
        .map(|(file, issue)| format!("- {}: {issue}", root.join(file).display()))
        .collect::<Vec<_>>()
        .join("\n");
    format!("gate sync found task issues:\n{details}")
}

pub(crate) fn validate_unique_cached_task_ids(rows: &[Value]) -> Result<(), GateError> {
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
