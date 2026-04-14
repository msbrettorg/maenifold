use super::with_db;
use crate::error::AppError;
use crate::now_ms;
use crate::types::*;
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use uuid::Uuid;

fn validate_role_name(role_name: &str) -> Result<(), String> {
    if role_name.is_empty() {
        return Err(AppError::validation("role name required").to_string());
    }
    if role_name.chars().any(|c| c.is_whitespace()) {
        return Err(AppError::validation("role name cannot contain spaces").to_string());
    }
    if !role_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            AppError::validation("role name only allows letters, numbers, - and _").to_string(),
        );
    }
    Ok(())
}

pub(crate) fn upsert_role(
    state: &AppState,
    role_name: String,
    runtime_kind: String,
    system_prompt: String,
    model: Option<String>,
    mode: Option<String>,
    mcp_servers_json: Option<String>,
    config_options_json: Option<String>,
    config_option_defs_json: Option<String>,
    auto_approve: Option<bool>,
) -> Result<Role, String> {
    let role_name = role_name.trim().to_string();
    validate_role_name(&role_name)?;
    let now = now_ms();
    let existing = with_db(state, |conn| {
        let name_hit: Option<(String, String, Option<String>, Option<String>, Option<String>, Option<bool>)> = conn
            .query_row(
                "SELECT id, role_name, mcp_servers_json, config_options_json, config_option_defs_json, auto_approve
                 FROM roles WHERE lower(role_name) = lower(?1) LIMIT 1",
                params![&role_name],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<bool>>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| AppError::db(e.to_string()).to_string())?;
        if let Some((id, existing_name, mcp, cfg, cfg_defs, approve)) = name_hit {
            if existing_name != role_name {
                return Err(AppError::already_exists(format!(
                    "role name already exists: {}",
                    existing_name
                ))
                .to_string());
            }
            return Ok(Some((id, mcp, cfg, cfg_defs, approve)));
        }
        Ok(None)
    })?;

    let id = existing
        .as_ref()
        .map(|(id, _, _, _, _)| id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mcp = mcp_servers_json
        .or_else(|| existing.as_ref().and_then(|(_, mcp, _, _, _)| mcp.clone()))
        .unwrap_or_else(|| "[]".to_string());
    let cfg = config_options_json
        .or_else(|| existing.as_ref().and_then(|(_, _, cfg, _, _)| cfg.clone()))
        .unwrap_or_else(|| "{}".to_string());
    let cfg_defs = config_option_defs_json
        .or_else(|| {
            existing
                .as_ref()
                .and_then(|(_, _, _, defs, _)| defs.clone())
        })
        .unwrap_or_else(|| "[]".to_string());
    let approve = auto_approve
        .or_else(|| existing.as_ref().and_then(|(_, _, _, _, v)| *v))
        .unwrap_or(true);

    with_db(state, |conn| {
        conn.execute(
            "INSERT INTO roles (id, role_name, runtime_kind, system_prompt, model, mode, mcp_servers_json, config_options_json, config_option_defs_json, auto_approve, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(role_name) DO UPDATE SET
               runtime_kind = excluded.runtime_kind,
               system_prompt = excluded.system_prompt,
               model = excluded.model,
               mode = excluded.mode,
               mcp_servers_json = excluded.mcp_servers_json,
               config_options_json = excluded.config_options_json,
               config_option_defs_json = excluded.config_option_defs_json,
               auto_approve = excluded.auto_approve,
               updated_at = excluded.updated_at",
            params![
                &id,
                &role_name,
                &runtime_kind,
                &system_prompt,
                &model,
                &mode,
                &mcp,
                &cfg,
                &cfg_defs,
                approve,
                now,
                now,
            ],
        )
        .map_err(|e| AppError::db(e.to_string()).to_string())?;
        Ok(())
    })?;

    let role = Role {
        id,
        role_name,
        runtime_kind,
        system_prompt,
        model,
        mode,
        mcp_servers_json: mcp,
        config_options_json: cfg,
        config_option_defs_json: cfg_defs,
        auto_approve: approve,
        created_at: now,
        updated_at: now,
    };
    // Invalidate cache so next load_role re-fetches fresh data.
    state.role_cache.remove(&role.role_name);
    Ok(role)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoleInput {
    pub(crate) role_name: String,
    pub(crate) runtime_kind: String,
    pub(crate) system_prompt: String,
    pub(crate) model: Option<String>,
    pub(crate) mode: Option<String>,
    pub(crate) mcp_servers_json: Option<String>,
    pub(crate) config_options_json: Option<String>,
    pub(crate) config_option_defs_json: Option<String>,
    pub(crate) auto_approve: Option<bool>,
}

pub(crate) async fn upsert_role_cmd(state: &AppState, input: RoleInput) -> Result<Role, String> {
    let role = upsert_role(
        state,
        input.role_name,
        input.runtime_kind,
        input.system_prompt,
        input.model,
        input.mode,
        input.mcp_servers_json,
        input.config_options_json,
        input.config_option_defs_json,
        input.auto_approve,
    )?;
    Ok(role)
}

fn role_from_row(row: &rusqlite::Row) -> rusqlite::Result<Role> {
    Ok(Role {
        id: row.get(0)?,
        role_name: row.get(1)?,
        runtime_kind: row.get(2)?,
        system_prompt: row.get(3)?,
        model: row.get(4)?,
        mode: row.get(5)?,
        mcp_servers_json: row
            .get::<_, Option<String>>(6)?
            .unwrap_or_else(|| "[]".to_string()),
        config_options_json: row
            .get::<_, Option<String>>(7)?
            .unwrap_or_else(|| "{}".to_string()),
        config_option_defs_json: row
            .get::<_, Option<String>>(8)?
            .unwrap_or_else(|| "[]".to_string()),
        auto_approve: row.get::<_, Option<bool>>(9)?.unwrap_or(true),
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub(crate) fn list_all_roles(state: &AppState) -> Result<Vec<Role>, String> {
    with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, role_name, runtime_kind, system_prompt, model, mode, mcp_servers_json, config_options_json, config_option_defs_json, auto_approve, created_at, updated_at
                 FROM roles ORDER BY role_name ASC",
            )
            .map_err(|e| AppError::db(e.to_string()).to_string())?;
        let rows = stmt
            .query_map([], role_from_row)
            .map_err(|e| AppError::db(e.to_string()).to_string())?;
        let mut roles = Vec::new();
        for row in rows {
            roles.push(row.map_err(|e| AppError::db(e.to_string()).to_string())?);
        }
        Ok(roles)
    })
}

pub(crate) fn list_roles(state: &AppState) -> Result<Vec<Role>, String> {
    list_all_roles(state)
}

pub(crate) fn delete_role_internal(state: &AppState, role_name: &str) -> Result<(), String> {
    let role_name = role_name.trim().to_string();
    if role_name.is_empty() {
        return Err(AppError::validation("role name required").to_string());
    }
    with_db(state, |conn| {
        let active_using_count: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM app_sessions WHERE closed_at IS NULL AND active_role = ?1",
                params![&role_name],
                |row| row.get(0),
            )
            .map_err(|e| AppError::db(e.to_string()).to_string())?;
        if active_using_count > 0 {
            return Err(AppError::invalid_input(format!(
                "role is active in {active_using_count} open session(s); switch role first",
            ))
            .to_string());
        }
        let mapped_open_count: i64 = conn
            .query_row(
                "SELECT COUNT(1)
                 FROM app_session_roles r
                 JOIN app_sessions s ON s.id = r.app_session_id
                 WHERE s.closed_at IS NULL AND r.role_name = ?1",
                params![&role_name],
                |row| row.get(0),
            )
            .map_err(|e| AppError::db(e.to_string()).to_string())?;
        if mapped_open_count > 0 {
            return Err(AppError::invalid_input(format!(
                "role \"{role_name}\" is used in {mapped_open_count} open session(s) — close those sessions first, then delete the role",
            ))
            .to_string());
        }
        conn.execute(
            "DELETE FROM roles WHERE role_name = ?1",
            params![&role_name],
        )
        .map_err(|e| AppError::db(e.to_string()).to_string())?;
        conn.execute(
            "DELETE FROM app_session_roles WHERE role_name = ?1",
            params![&role_name],
        )
        .map_err(|e| AppError::db(e.to_string()).to_string())?;
        Ok(())
    })?;
    state.role_cache.remove(&role_name);
    Ok(())
}

pub(crate) fn delete_role_cmd(state: &AppState, role_name: String) -> Result<(), String> {
    delete_role_internal(state, &role_name)
}

pub(crate) fn load_role(state: &AppState, role_name: &str) -> Result<Option<Role>, String> {
    // Check in-memory cache first to avoid a DB round-trip on every assistant_chat call.
    if let Some(cached) = state.role_cache.get(role_name) {
        return Ok(Some((**cached).clone()));
    }
    let result = with_db(state, |conn| {
        conn.query_row(
            "SELECT id, role_name, runtime_kind, system_prompt, model, mode, mcp_servers_json, config_options_json, config_option_defs_json, auto_approve, created_at, updated_at FROM roles WHERE role_name = ?1",
            params![role_name],
            role_from_row,
        ).optional().map_err(|e| AppError::db(e.to_string()).to_string())
    })?;
    if let Some(ref role) = result {
        state
            .role_cache
            .insert(role_name.to_string(), std::sync::Arc::new(role.clone()));
    }
    Ok(result)
}

pub(crate) fn update_role_config_option_defs_if_changed(
    state: &AppState,
    role_name: &str,
    config_option_defs_json: &str,
) -> Result<bool, String> {
    with_db(state, |conn| {
        let existing: Option<String> = conn
            .query_row(
                "SELECT config_option_defs_json FROM roles WHERE role_name = ?1",
                params![role_name],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|e| AppError::db(e.to_string()).to_string())?
            .flatten();

        let changed = match existing {
            Some(current) => {
                let current_v = serde_json::from_str::<serde_json::Value>(&current).ok();
                let next_v =
                    serde_json::from_str::<serde_json::Value>(config_option_defs_json).ok();
                match (current_v, next_v) {
                    (Some(a), Some(b)) => a != b,
                    _ => current != config_option_defs_json,
                }
            }
            None => false,
        };
        if !changed {
            return Ok(false);
        }

        conn.execute(
            "UPDATE roles
             SET config_option_defs_json = ?1, updated_at = ?2
             WHERE role_name = ?3",
            params![config_option_defs_json, now_ms(), role_name],
        )
        .map_err(|e| AppError::db(e.to_string()).to_string())?;
        Ok(true)
    })
    .map(|changed| {
        if changed {
            state.role_cache.remove(role_name);
        }
        changed
    })
}

pub(crate) fn load_role_runtime_kind(state: &AppState, role_name: &str) -> Result<String, String> {
    with_db(state, |conn| {
        let runtime = conn
            .query_row(
                "SELECT runtime_kind FROM roles WHERE role_name = ?1",
                params![role_name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| AppError::db(e.to_string()).to_string())?;
        runtime
            .ok_or_else(|| AppError::not_found(format!("role not found: {role_name}")).to_string())
    })
}

pub(crate) fn resolve_role_runtime(state: &AppState, role_name: &str) -> Result<String, String> {
    load_role_runtime_kind(state, role_name)
}
