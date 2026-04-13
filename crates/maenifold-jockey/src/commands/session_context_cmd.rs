use crate::db::context::{clear_shared_context_internal, set_shared_context_internal};
use crate::db::session_context::{app_session_scope, list_shared_context_prefix_internal};
use crate::types::{AppState, ContextEntry};

fn ensure_scope_in_app_session(app_session_id: &str, scope: &str) -> Result<(), String> {
    let root = app_session_scope(app_session_id);
    let child_prefix = format!("{root}:");
    if scope == root || scope.starts_with(&child_prefix) {
        return Ok(());
    }
    Err(format!("scope must stay within {root}"))
}

pub(crate) fn list_session_context_entries_cmd(
    state: &AppState,
    app_session_id: String,
) -> Result<Vec<ContextEntry>, String> {
    let sid = app_session_id.trim();
    if sid.is_empty() {
        return Err("app session id required".to_string());
    }
    let root = app_session_scope(sid);
    let child_prefix = format!("{root}:");
    list_shared_context_prefix_internal(state, &root).map(|items| {
        items
            .into_iter()
            .filter(|entry| entry.scope == root || entry.scope.starts_with(&child_prefix))
            .collect()
    })
}

pub(crate) fn set_session_context_entry_cmd(
    state: &AppState,
    app_session_id: String,
    scope: String,
    key: String,
    value: String,
) -> Result<ContextEntry, String> {
    let sid = app_session_id.trim();
    if sid.is_empty() {
        return Err("app session id required".to_string());
    }
    let scope = scope.trim();
    let key = key.trim();
    if scope.is_empty() || key.is_empty() {
        return Err("scope/key required".to_string());
    }
    ensure_scope_in_app_session(sid, scope)?;
    set_shared_context_internal(state, scope, key, value.trim())
}

pub(crate) fn delete_session_context_entry_cmd(
    state: &AppState,
    app_session_id: String,
    scope: String,
    key: String,
) -> Result<(), String> {
    let sid = app_session_id.trim();
    if sid.is_empty() {
        return Err("app session id required".to_string());
    }
    let scope = scope.trim();
    let key = key.trim();
    if scope.is_empty() || key.is_empty() {
        return Err("scope/key required".to_string());
    }
    ensure_scope_in_app_session(sid, scope)?;
    clear_shared_context_internal(state, scope, key)
}
