use crate::types::*;
use crate::{default_chat_cwd, now_ms};
use super::with_db;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

fn validate_session_title(raw: &str) -> Result<String, String> {
    let title = raw.trim().to_string();
    if title.is_empty() {
        return Err("session name required".to_string());
    }
    if title.chars().any(|c| c.is_whitespace()) {
        return Err("session name cannot contain spaces".to_string());
    }
    Ok(title)
}

fn active_session_title_exists(
    conn: &rusqlite::Connection,
    title: &str,
    exclude_id: Option<&str>,
) -> Result<bool, String> {
    let sql = if exclude_id.is_some() {
        "SELECT 1 FROM app_sessions WHERE lower(title) = lower(?1) AND closed_at IS NULL AND id <> ?2 LIMIT 1"
    } else {
        "SELECT 1 FROM app_sessions WHERE lower(title) = lower(?1) AND closed_at IS NULL LIMIT 1"
    };
    let exists = if let Some(id) = exclude_id {
        conn.query_row(sql, params![title, id], |_row| Ok(()))
            .optional()
            .map_err(|e| e.to_string())?
            .is_some()
    } else {
        conn.query_row(sql, params![title], |_row| Ok(()))
            .optional()
            .map_err(|e| e.to_string())?
            .is_some()
    };
    Ok(exists)
}

pub(crate) fn get_app_session_cwd(state: &AppState, session_id: &str) -> Option<String> {
    with_db(state, |conn| {
        conn.query_row(
            "SELECT cwd FROM app_sessions WHERE id = ?1",
            params![session_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())
    })
    .ok()
    .flatten()
    .flatten()
}

pub(crate) fn set_app_session_cwd(
    state: &AppState,
    session_id: &str,
    cwd: &str,
) -> Result<(), String> {
    let now = crate::now_ms();
    with_db(state, |conn| {
        conn.execute(
            "UPDATE app_sessions SET cwd = ?1, last_active_at = ?2 WHERE id = ?3",
            params![cwd, now, session_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn query_sessions(conn: &rusqlite::Connection, sql: &str) -> Result<Vec<AppSession>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mut sessions: Vec<AppSession> = stmt
        .query_map([], |row| {
            Ok(AppSession {
                id: row.get(0)?,
                title: row.get(1)?,
                active_role: row.get(2)?,
                runtime_kind: row.get(3)?,
                cwd: row.get(4)?,
                messages: Vec::new(),
                created_at: row.get(5)?,
                last_active_at: row.get(6)?,
                closed_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut msg_stmt = conn
        .prepare(
            "SELECT role_name, content, created_at
             FROM app_session_messages
             WHERE session_id = ?1
             ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;

    for session in sessions.iter_mut() {
        let rows = msg_stmt
            .query_map(params![&session.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut messages = Vec::new();
        for row in rows {
            let (role_name, content, at) = row.map_err(|e| e.to_string())?;
            messages.push(serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "roleName": role_name,
                "text": content,
                "at": at
            }));
        }
        session.messages = messages;
    }

    Ok(sessions)
}

pub(crate) fn list_app_sessions(state: &AppState) -> Result<Vec<AppSession>, String> {
    with_db(state, |conn| {
        query_sessions(
            conn,
            "SELECT id, title, active_role, runtime_kind, cwd, created_at, last_active_at, closed_at
             FROM app_sessions WHERE closed_at IS NULL ORDER BY created_at ASC LIMIT 50",
        )
    })
}

pub(crate) fn list_closed_app_sessions(
    state: &AppState,
) -> Result<Vec<AppSession>, String> {
    with_db(state, |conn| {
        query_sessions(
            conn,
            "SELECT id, title, active_role, runtime_kind, cwd, created_at, last_active_at, closed_at
             FROM app_sessions WHERE closed_at IS NOT NULL ORDER BY closed_at DESC LIMIT 200",
        )
    })
}

pub(crate) fn append_app_message(
    state: &AppState,
    session_id: String,
    role_name: String,
    content: String,
) -> Result<(), String> {
    let now = now_ms();
    with_db(state, |conn| {
        conn.execute(
            "INSERT INTO app_session_messages (session_id, role_name, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![&session_id, &role_name, &content, now],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE app_sessions SET last_active_at = ?1 WHERE id = ?2",
            params![now, &session_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub(crate) fn create_app_session_internal(
    state: &AppState,
    title: Option<&str>,
) -> Result<AppSession, String> {
    let now = now_ms();
    let title = validate_session_title(title.unwrap_or("Session_1"))?;
    let session = AppSession {
        id: Uuid::new_v4().to_string(),
        title,
        active_role: "Jockey".to_string(),
        runtime_kind: None,
        cwd: Some(default_chat_cwd()),
        messages: Vec::new(),
        created_at: now,
        last_active_at: now,
        closed_at: None,
    };
    with_db(state, |conn| {
        if active_session_title_exists(conn, &session.title, None)? {
            return Err(format!("session name already exists: {}", session.title));
        }
        conn.execute(
            "INSERT INTO app_sessions (id, title, active_role, runtime_kind, cwd, created_at, last_active_at, closed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![
                &session.id,
                &session.title,
                &session.active_role,
                &session.runtime_kind,
                &session.cwd,
                session.created_at,
                session.last_active_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(session)
}

pub(crate) fn close_app_session_internal(state: &AppState, id: &str) -> Result<(), String> {
    let now = now_ms();
    with_db(state, |conn| {
        let changed = conn
            .execute(
                "UPDATE app_sessions SET closed_at = ?1 WHERE id = ?2 AND closed_at IS NULL",
                params![now, id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err(format!("session not found or already closed: {id}"));
        }
        Ok(())
    })
}

pub(crate) fn create_app_session(
    state: &AppState,
    title: Option<String>,
) -> Result<AppSession, String> {
    create_app_session_internal(state, title.as_deref())
}

pub(crate) fn update_app_session(
    state: &AppState,
    id: String,
    update: AppSessionUpdate,
) -> Result<(), String> {
    with_db(state, |conn| {
        if let Some(ref title) = update.title {
            let title = validate_session_title(title)?;
            if active_session_title_exists(conn, &title, Some(&id))? {
                return Err(format!("session name already exists: {}", title));
            }
            conn.execute(
                "UPDATE app_sessions SET title = ?1 WHERE id = ?2",
                params![title, &id],
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some(ref role) = update.active_role {
            conn.execute(
                "UPDATE app_sessions SET active_role = ?1 WHERE id = ?2",
                params![role, &id],
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some(runtime) = update.runtime_kind {
            conn.execute(
                "UPDATE app_sessions SET runtime_kind = ?1 WHERE id = ?2",
                params![runtime, &id],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}

pub(crate) fn delete_app_session(state: &AppState, id: String) -> Result<(), String> {
    let now = now_ms();
    with_db(state, |conn| {
        conn.execute(
            "UPDATE app_sessions SET closed_at = ?1 WHERE id = ?2",
            params![now, &id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub(crate) fn reopen_app_session(
    state: &AppState,
    id: String,
) -> Result<AppSession, String> {
    let now = now_ms();
    with_db(state, |conn| {
        let title: String = conn
            .query_row(
                "SELECT title FROM app_sessions WHERE id = ?1",
                params![&id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if active_session_title_exists(conn, &title, Some(&id))? {
            return Err(format!("session name already exists: {}", title));
        }
        conn.execute(
            "UPDATE app_sessions SET closed_at = NULL, last_active_at = ?1 WHERE id = ?2",
            params![now, &id],
        )
        .map_err(|e| e.to_string())?;
        let session = conn
            .query_row(
                "SELECT id, title, active_role, runtime_kind, cwd, created_at, last_active_at, closed_at
                 FROM app_sessions WHERE id = ?1",
                params![&id],
                |row| {
                    Ok(AppSession {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        active_role: row.get(2)?,
                        runtime_kind: row.get(3)?,
                        cwd: row.get(4)?,
                        messages: Vec::new(),
                        created_at: row.get(5)?,
                        last_active_at: row.get(6)?,
                        closed_at: row.get(7)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;
        Ok(session)
    })
}
