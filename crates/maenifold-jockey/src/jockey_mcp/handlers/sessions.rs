use serde_json::{json, Value};

use crate::db::app_session::{close_app_session_internal, create_app_session_internal};
use crate::db::with_db;
use crate::types::AppState;

pub(crate) fn list_sessions(state: &AppState, params: Value) -> Result<Value, String> {
    let limit = params.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);
    let sessions = with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, title, active_role, runtime_kind, cwd, created_at, last_active_at \
                 FROM app_sessions WHERE closed_at IS NULL ORDER BY last_active_at DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<Value> = stmt
            .query_map(rusqlite::params![limit], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, Option<String>>(1)?,
                    "activeRole": row.get::<_, Option<String>>(2)?,
                    "runtimeKind": row.get::<_, Option<String>>(3)?,
                    "cwd": row.get::<_, Option<String>>(4)?,
                    "createdAt": row.get::<_, i64>(5)?,
                    "lastActiveAt": row.get::<_, i64>(6)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    })?;
    Ok(json!(sessions))
}

pub(crate) fn get_session(state: &AppState, params: Value) -> Result<Value, String> {
    let id = params
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("id is required")?;
    let session = with_db(state, |conn| {
        conn.query_row(
            "SELECT id, title, active_role, runtime_kind, cwd, created_at, last_active_at \
             FROM app_sessions WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, Option<String>>(1)?,
                    "activeRole": row.get::<_, Option<String>>(2)?,
                    "runtimeKind": row.get::<_, Option<String>>(3)?,
                    "cwd": row.get::<_, Option<String>>(4)?,
                    "createdAt": row.get::<_, i64>(5)?,
                    "lastActiveAt": row.get::<_, i64>(6)?,
                }))
            },
        )
        .map_err(|e| e.to_string())
    })?;
    Ok(session)
}

pub(crate) fn update_session(state: &AppState, params: Value) -> Result<Value, String> {
    let id = params
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("id is required")?;
    let now = crate::now_ms();
    with_db(state, |conn| {
        if let Some(title) = params.get("title").and_then(|v| v.as_str()) {
            conn.execute(
                "UPDATE app_sessions SET title = ?1, last_active_at = ?2 WHERE id = ?3",
                rusqlite::params![title, now, id],
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some(role) = params.get("activeRole").and_then(|v| v.as_str()) {
            conn.execute(
                "UPDATE app_sessions SET active_role = ?1, last_active_at = ?2 WHERE id = ?3",
                rusqlite::params![role, now, id],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })?;
    Ok(json!(format!("Session '{id}' updated")))
}

pub(crate) fn create_session(state: &AppState, params: Value) -> Result<Value, String> {
    let title = params.get("title").and_then(|v| v.as_str());
    let session = create_app_session_internal(state, title)?;
    Ok(json!({
        "id": session.id,
        "title": session.title,
        "activeRole": session.active_role,
    }))
}

pub(crate) fn close_session(state: &AppState, params: Value) -> Result<Value, String> {
    let id = params
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("id is required")?;
    close_app_session_internal(state, id)?;
    Ok(json!(format!("Session '{id}' closed")))
}

pub(crate) fn get_session_history(state: &AppState, params: Value) -> Result<Value, String> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or("sessionId is required")?;
    let limit = params.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);
    let mut messages = with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT role_name, content, created_at FROM app_session_messages \
                 WHERE session_id = ?1 ORDER BY id DESC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<Value> = stmt
            .query_map(rusqlite::params![session_id, limit], |row| {
                Ok(json!({
                    "roleName": row.get::<_, String>(0)?,
                    "content": row.get::<_, String>(1)?,
                    "createdAt": row.get::<_, i64>(2)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    })?;
    messages.reverse();
    Ok(json!(messages))
}
