use crate::now_ms;
use crate::types::*;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;
use super::with_db;

fn validate_workflow_name(raw: &str) -> Result<String, String> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err("workflow name required".to_string());
    }
    if name.chars().any(|c| c.is_whitespace()) {
        return Err("workflow name cannot contain spaces".to_string());
    }
    Ok(name)
}

pub(crate) fn create_workflow_internal(
    state: &AppState,
    name: String,
    steps: Vec<String>,
) -> Result<Workflow, String> {
    let name = validate_workflow_name(&name)?;
    if steps.is_empty() {
        return Err("workflow steps cannot be empty".to_string());
    }
    let now = now_ms();
    let workflow = Workflow {
        id: Uuid::new_v4().to_string(),
        name,
        steps,
        created_at: now,
        updated_at: now,
    };
    let steps_json = serde_json::to_string(&workflow.steps).map_err(|e| e.to_string())?;
    with_db(state, |conn| {
        let exists = conn
            .query_row(
                "SELECT 1 FROM workflows WHERE lower(name) = lower(?1) LIMIT 1",
                params![&workflow.name],
                |_row| Ok(()),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .is_some();
        if exists {
            return Err(format!("workflow name already exists: {}", workflow.name));
        }
        conn.execute(
            "INSERT INTO workflows (id, name, steps_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &workflow.id,
                &workflow.name,
                &steps_json,
                workflow.created_at,
                workflow.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(workflow)
}

pub(crate) fn create_workflow(
    state: &AppState,
    name: String,
    steps: Vec<String>,
) -> Result<Workflow, String> {
    create_workflow_internal(state, name, steps)
}

pub(crate) fn list_workflows(state: &AppState) -> Result<Vec<Workflow>, String> {
    with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, steps_json, created_at, updated_at
                 FROM workflows ORDER BY updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let steps_json: String = row.get(2)?;
                let steps = serde_json::from_str::<Vec<String>>(&steps_json).unwrap_or_default();
                Ok(Workflow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    steps,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut workflows = Vec::new();
        for row in rows {
            workflows.push(row.map_err(|e| e.to_string())?);
        }
        Ok(workflows)
    })
}

pub(crate) fn list_workflows_internal(state: &AppState) -> Result<Vec<Workflow>, String> {
    with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, steps_json, created_at, updated_at
                 FROM workflows ORDER BY updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let steps_json: String = row.get(2)?;
                let steps = serde_json::from_str::<Vec<String>>(&steps_json).unwrap_or_default();
                Ok(Workflow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    steps,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut workflows = Vec::new();
        for row in rows {
            workflows.push(row.map_err(|e| e.to_string())?);
        }
        Ok(workflows)
    })
}

pub(crate) fn update_workflow_internal(
    state: &AppState,
    workflow_id: &str,
    name: Option<String>,
    steps: Option<Vec<String>>,
) -> Result<Workflow, String> {
    let now = now_ms();
    with_db(state, |conn| {
        if let Some(ref n) = name {
            let n = validate_workflow_name(n)?;
            let conflict = conn
                .query_row(
                    "SELECT 1 FROM workflows WHERE lower(name) = lower(?1) AND id <> ?2 LIMIT 1",
                    params![&n, workflow_id],
                    |_| Ok(()),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .is_some();
            if conflict {
                return Err(format!("workflow name already exists: {n}"));
            }
            conn.execute(
                "UPDATE workflows SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![n, now, workflow_id],
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some(ref s) = steps {
            if s.is_empty() {
                return Err("workflow steps cannot be empty".to_string());
            }
            let steps_json = serde_json::to_string(s).map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE workflows SET steps_json = ?1, updated_at = ?2 WHERE id = ?3",
                params![steps_json, now, workflow_id],
            )
            .map_err(|e| e.to_string())?;
        }
        conn.query_row(
            "SELECT id, name, steps_json, created_at, updated_at FROM workflows WHERE id = ?1",
            params![workflow_id],
            |row| {
                let steps_json: String = row.get(2)?;
                let steps = serde_json::from_str::<Vec<String>>(&steps_json).unwrap_or_default();
                Ok(Workflow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    steps,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.to_string())
    })
}

pub(crate) fn delete_workflow_internal(state: &AppState, workflow_id: &str) -> Result<(), String> {
    with_db(state, |conn| {
        let deleted = conn
            .execute("DELETE FROM workflows WHERE id = ?1", params![workflow_id])
            .map_err(|e| e.to_string())?;
        if deleted == 0 {
            return Err(format!("workflow not found: {workflow_id}"));
        }
        Ok(())
    })
}

pub(crate) fn delete_workflow(state: &AppState, id: String) -> Result<(), String> {
    delete_workflow_internal(state, &id)
}

pub(crate) fn load_workflow(state: &AppState, workflow_id: &str) -> Result<Workflow, String> {
    with_db(state, |conn| {
        conn.query_row(
            "SELECT id, name, steps_json, created_at, updated_at FROM workflows WHERE id = ?1",
            params![workflow_id],
            |row| {
                let steps_json: String = row.get(2)?;
                let steps = serde_json::from_str::<Vec<String>>(&steps_json).unwrap_or_default();
                Ok(Workflow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    steps,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.to_string())
    })
}
