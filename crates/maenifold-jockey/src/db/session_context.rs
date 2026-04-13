use crate::db::with_db;
use crate::types::{AppState, ContextEntry};
use rusqlite::params;

pub(crate) fn app_session_scope(app_session_id: &str) -> String {
    format!("session:{app_session_id}")
}

pub(crate) fn app_session_role_scope(app_session_id: &str, role_name: &str) -> String {
    format!("session:{app_session_id}:role:{role_name}")
}

pub(crate) fn list_shared_context_prefix_internal(
    state: &AppState,
    prefix: &str,
) -> Result<Vec<ContextEntry>, String> {
    with_db(state, |conn| {
        let like = format!("{prefix}%");
        let mut stmt = conn
            .prepare(
                "SELECT scope, key, value, updated_at
                 FROM shared_context_snapshots
                 WHERE scope LIKE ?1
                 ORDER BY scope ASC, updated_at DESC, key ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![like], |row| {
                Ok(ContextEntry {
                    scope: row.get(0)?,
                    key: row.get(1)?,
                    value: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for item in rows {
            items.push(item.map_err(|e| e.to_string())?);
        }
        Ok(items)
    })
}
