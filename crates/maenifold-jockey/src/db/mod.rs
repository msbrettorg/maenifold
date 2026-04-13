pub(crate) mod app_session;
pub(crate) mod app_session_role;
pub(crate) mod context;
pub(crate) mod global_mcp;
pub(crate) mod pool;
pub(crate) mod role;
pub(crate) mod session;
pub(crate) mod session_context;
pub(crate) mod skill;
pub(crate) mod workflow;

pub(crate) use pool::DbPool;

use crate::types::*;
use rusqlite::Connection;
use serde_json::{json, Value};

fn is_safe_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    column_def: &str,
) -> Result<(), String> {
    if !is_safe_identifier(table) {
        return Err(format!("invalid table name: {table}"));
    }
    if !is_safe_identifier(column) {
        return Err(format!("invalid column name: {column}"));
    }
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?;
    let mut exists = false;
    for row in rows {
        let name = row.map_err(|e| e.to_string())?;
        if name == column {
            exists = true;
            break;
        }
    }
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {column_def}"),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS roles (
          id TEXT PRIMARY KEY,
          role_name TEXT NOT NULL UNIQUE,
          runtime_kind TEXT NOT NULL,
          system_prompt TEXT NOT NULL,
          model TEXT,
          mode TEXT,
          mcp_servers_json TEXT DEFAULT '[]',
          config_options_json TEXT DEFAULT '{}',
          config_option_defs_json TEXT DEFAULT '[]',
          auto_approve INTEGER DEFAULT 1,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS workflows (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          steps_json TEXT NOT NULL,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sessions (
          id TEXT PRIMARY KEY,
          workflow_id TEXT NOT NULL,
          status TEXT NOT NULL,
          initial_prompt TEXT NOT NULL,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS session_events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          session_id TEXT NOT NULL,
          event_type TEXT NOT NULL,
          role_name TEXT,
          payload TEXT NOT NULL,
          created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS shared_context_snapshots (
          scope TEXT NOT NULL,
          key TEXT NOT NULL,
          value TEXT NOT NULL,
          updated_at INTEGER NOT NULL,
          PRIMARY KEY(scope, key)
        );
        CREATE TABLE IF NOT EXISTS dynamic_catalog_entries (
          kind TEXT NOT NULL,
          name TEXT NOT NULL,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          PRIMARY KEY(kind, name)
        );
        CREATE TABLE IF NOT EXISTS app_sessions (
          id TEXT PRIMARY KEY,
          title TEXT NOT NULL,
          active_role TEXT NOT NULL DEFAULT 'Jockey',
          runtime_kind TEXT,
          cwd TEXT,
          created_at INTEGER NOT NULL,
          last_active_at INTEGER NOT NULL,
          closed_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS app_session_roles (
          app_session_id TEXT NOT NULL REFERENCES app_sessions(id) ON DELETE CASCADE,
          role_name TEXT NOT NULL,
          runtime_kind TEXT NOT NULL,
          acp_session_id TEXT,
          model_override TEXT,
          mode_override TEXT,
          mcp_servers_json TEXT,
          config_options_json TEXT,
          PRIMARY KEY(app_session_id, role_name)
        );
        CREATE TABLE IF NOT EXISTS app_skills (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL UNIQUE,
          description TEXT NOT NULL DEFAULT '',
          content TEXT NOT NULL DEFAULT '',
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS app_session_messages (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          session_id TEXT NOT NULL REFERENCES app_sessions(id) ON DELETE CASCADE,
          role_name TEXT NOT NULL,
          content TEXT NOT NULL,
          created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_roles_updated_at
          ON roles(updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_workflows_updated_at
          ON workflows(updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_sessions_created_at
          ON sessions(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_sessions_workflow_id
          ON sessions(workflow_id);
        CREATE INDEX IF NOT EXISTS idx_session_events_session_id_id
          ON session_events(session_id, id ASC);
        CREATE INDEX IF NOT EXISTS idx_shared_context_scope_updated_at
          ON shared_context_snapshots(scope, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_dynamic_catalog_kind_updated_at
          ON dynamic_catalog_entries(kind, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_app_sessions_last_active
          ON app_sessions(last_active_at DESC);
        CREATE INDEX IF NOT EXISTS idx_app_skills_updated_at
          ON app_skills(updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_app_session_messages_session_id
          ON app_session_messages(session_id, id ASC);

        CREATE TABLE IF NOT EXISTS global_mcp_servers (
          name TEXT PRIMARY KEY,
          config_json TEXT NOT NULL,
          is_builtin INTEGER NOT NULL DEFAULT 0,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );

        PRAGMA user_version = 4;
        ",
    )
    .map_err(|e| e.to_string())?;
    ensure_column(
        conn,
        "roles",
        "config_option_defs_json",
        "TEXT DEFAULT '[]'",
    )?;
    ensure_column(conn, "app_session_roles", "model_override", "TEXT")?;
    ensure_column(conn, "app_session_roles", "mode_override", "TEXT")?;
    ensure_column(conn, "app_session_roles", "mcp_servers_json", "TEXT")?;
    ensure_column(conn, "app_session_roles", "config_options_json", "TEXT")?;

    Ok(())
}

pub(crate) fn with_db<T>(
    state: &AppState,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let conn = state.db.get().map_err(|e| e.to_string())?;
    f(&conn)
}

pub(crate) fn parse_payload(payload: String) -> Value {
    serde_json::from_str::<Value>(&payload).unwrap_or(json!({ "text": payload }))
}

pub(crate) fn seed_default_dynamic_catalog(state: &AppState) -> Result<(), String> {
    for model in DEFAULT_MODELS {
        let _ = context::upsert_dynamic_catalog_item(state, "model", model)?;
    }
    for server in DEFAULT_MCP_SERVERS {
        let _ = context::upsert_dynamic_catalog_item(state, "mcp", server)?;
    }
    for skill in DEFAULT_SKILLS {
        let _ = context::upsert_dynamic_catalog_item(state, "skill", skill)?;
    }
    Ok(())
}
