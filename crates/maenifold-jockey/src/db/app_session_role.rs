use crate::db::with_db;
use crate::types::AppState;
use rusqlite::{params, OptionalExtension};
use serde_json::{Map, Value};

#[derive(Clone, Default)]
pub(crate) struct AppSessionRoleState {
    pub(crate) runtime_kind: Option<String>,
    pub(crate) acp_session_id: Option<String>,
    pub(crate) model_override: Option<String>,
    pub(crate) mode_override: Option<String>,
    pub(crate) config_options_json: Option<String>,
}

fn ensure_app_session_role_row(
    conn: &rusqlite::Connection,
    app_session_id: &str,
    role_name: &str,
    runtime_kind: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_session_roles (
            app_session_id, role_name, runtime_kind
         ) VALUES (?1, ?2, ?3)
         ON CONFLICT(app_session_id, role_name) DO UPDATE SET runtime_kind = excluded.runtime_kind",
        params![app_session_id, role_name, runtime_kind],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn load_app_session_role_state(
    state: &AppState,
    app_session_id: &str,
    role_name: &str,
) -> Result<Option<AppSessionRoleState>, String> {
    if app_session_id.trim().is_empty() {
        return Ok(None);
    }
    with_db(state, |conn| {
        conn.query_row(
            "SELECT runtime_kind, acp_session_id, model_override, mode_override, config_options_json
             FROM app_session_roles
             WHERE app_session_id = ?1 AND role_name = ?2",
            params![app_session_id, role_name],
            |row| {
                Ok(AppSessionRoleState {
                    runtime_kind: row.get(0)?,
                    acp_session_id: row.get(1)?,
                    model_override: row.get(2)?,
                    mode_override: row.get(3)?,
                    config_options_json: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
    })
}

pub(crate) fn save_app_session_role_model_override(
    state: &AppState,
    app_session_id: &str,
    role_name: &str,
    runtime_kind: &str,
    model_override: Option<&str>,
) -> Result<(), String> {
    if app_session_id.trim().is_empty() {
        return Err("app session id required".to_string());
    }
    with_db(state, |conn| {
        ensure_app_session_role_row(conn, app_session_id, role_name, runtime_kind)?;
        conn.execute(
            "UPDATE app_session_roles
             SET model_override = ?1, runtime_kind = ?2
             WHERE app_session_id = ?3 AND role_name = ?4",
            params![model_override, runtime_kind, app_session_id, role_name],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub(crate) fn save_app_session_role_mode_override(
    state: &AppState,
    app_session_id: &str,
    role_name: &str,
    runtime_kind: &str,
    mode_override: Option<&str>,
) -> Result<(), String> {
    if app_session_id.trim().is_empty() {
        return Err("app session id required".to_string());
    }
    with_db(state, |conn| {
        ensure_app_session_role_row(conn, app_session_id, role_name, runtime_kind)?;
        conn.execute(
            "UPDATE app_session_roles
             SET mode_override = ?1, runtime_kind = ?2
             WHERE app_session_id = ?3 AND role_name = ?4",
            params![mode_override, runtime_kind, app_session_id, role_name],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub(crate) fn save_app_session_role_config_option_override(
    state: &AppState,
    app_session_id: &str,
    role_name: &str,
    runtime_kind: &str,
    config_id: &str,
    value: &str,
) -> Result<(), String> {
    if app_session_id.trim().is_empty() {
        return Err("app session id required".to_string());
    }
    if config_id.trim().is_empty() {
        return Err("config id required".to_string());
    }
    with_db(state, |conn| {
        ensure_app_session_role_row(conn, app_session_id, role_name, runtime_kind)?;
        let current_json: Option<String> = conn
            .query_row(
                "SELECT config_options_json
                 FROM app_session_roles
                 WHERE app_session_id = ?1 AND role_name = ?2",
                params![app_session_id, role_name],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .flatten();

        let mut map: Map<String, Value> = current_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        if value.trim().is_empty() {
            map.remove(config_id);
        } else {
            map.insert(config_id.to_string(), Value::String(value.to_string()));
        }

        let next_json = serde_json::to_string(&map).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE app_session_roles
             SET config_options_json = ?1, runtime_kind = ?2
             WHERE app_session_id = ?3 AND role_name = ?4",
            params![next_json, runtime_kind, app_session_id, role_name],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub(crate) fn load_app_session_role_cli_id(
    state: &AppState,
    app_session_id: &str,
    runtime_key: &str,
    role_name: &str,
) -> Option<String> {
    if app_session_id.is_empty() {
        return None;
    }
    load_app_session_role_state(state, app_session_id, role_name)
        .ok()
        .flatten()
        .and_then(|row| {
            if row.runtime_kind.as_deref() == Some(runtime_key) {
                row.acp_session_id
            } else {
                None
            }
        })
}

pub(crate) fn save_app_session_role_cli_id(
    state: &AppState,
    app_session_id: &str,
    runtime_key: &str,
    role_name: &str,
    cli_session_id: &str,
) -> Result<(), String> {
    if app_session_id.is_empty() {
        return Ok(());
    }
    with_db(state, |conn| {
        ensure_app_session_role_row(conn, app_session_id, role_name, runtime_key)?;
        conn.execute(
            "UPDATE app_session_roles
             SET runtime_kind = ?1, acp_session_id = ?2
             WHERE app_session_id = ?3 AND role_name = ?4",
            params![runtime_key, cli_session_id, app_session_id, role_name],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub(crate) fn clear_app_session_role_cli_id(
    state: &AppState,
    app_session_id: &str,
    role_name: &str,
) -> Result<(), String> {
    if app_session_id.trim().is_empty() {
        return Err("app session id required".to_string());
    }
    with_db(state, |conn| {
        conn.execute(
            "UPDATE app_session_roles
             SET acp_session_id = NULL
             WHERE app_session_id = ?1 AND role_name = ?2",
            params![app_session_id, role_name],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}
