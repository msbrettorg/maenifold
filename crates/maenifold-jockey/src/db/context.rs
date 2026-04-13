use crate::db::with_db;
use crate::types::*;
use crate::{acp, now_ms};
use rusqlite::{params, OptionalExtension};

pub(crate) fn shared_key(scope: &str, key: &str) -> String {
    format!("{scope}:{key}")
}

pub(crate) fn set_shared_context_internal(
    state: &AppState,
    scope: &str,
    key: &str,
    value: &str,
) -> Result<ContextEntry, String> {
    let now = now_ms();
    state
        .shared_context
        .insert(shared_key(scope, key), value.to_string());
    with_db(state, |conn| {
        conn.execute(
            "INSERT INTO shared_context_snapshots (scope, key, value, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(scope, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![scope, key, value, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(ContextEntry {
        scope: scope.to_string(),
        key: key.to_string(),
        value: value.to_string(),
        updated_at: now,
    })
}

pub(crate) fn clear_shared_context_internal(
    state: &AppState,
    scope: &str,
    key: &str,
) -> Result<(), String> {
    with_db(state, |conn| {
        conn.execute(
            "DELETE FROM shared_context_snapshots WHERE scope = ?1 AND key = ?2",
            params![scope, key],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;
    state.shared_context.remove(&shared_key(scope, key));
    Ok(())
}

pub(crate) fn list_shared_context_internal(
    state: &AppState,
    scope: &str,
) -> Result<Vec<ContextEntry>, String> {
    with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT scope, key, value, updated_at
                 FROM shared_context_snapshots
                 WHERE scope = ?1
                 ORDER BY updated_at DESC, key ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![scope], |row| {
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

pub(crate) fn sanitize_dynamic_item_name(raw: &str) -> Option<String> {
    let clean = raw.trim().to_ascii_lowercase();
    if clean.is_empty() {
        return None;
    }
    if clean
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
    {
        return Some(clean);
    }
    None
}

pub(crate) fn upsert_dynamic_catalog_item(
    state: &AppState,
    kind: &str,
    name: &str,
) -> Result<String, String> {
    let normalized = sanitize_dynamic_item_name(name)
        .ok_or_else(|| format!("invalid {} name: {}", kind, name))?;
    let now = now_ms();
    with_db(state, |conn| {
        conn.execute(
            "INSERT INTO dynamic_catalog_entries (kind, name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(kind, name) DO UPDATE SET updated_at = excluded.updated_at",
            params![kind, &normalized, now, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(normalized)
}

pub(crate) fn remove_dynamic_catalog_item(
    state: &AppState,
    kind: &str,
    name: &str,
) -> Result<bool, String> {
    let normalized = sanitize_dynamic_item_name(name)
        .ok_or_else(|| format!("invalid {} name: {}", kind, name))?;
    with_db(state, |conn| {
        let affected = conn
            .execute(
                "DELETE FROM dynamic_catalog_entries WHERE kind = ?1 AND name = ?2",
                params![kind, &normalized],
            )
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    })
}

pub(crate) fn list_dynamic_catalog(state: &AppState, kind: &str) -> Result<Vec<String>, String> {
    with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT name
                 FROM dynamic_catalog_entries
                 WHERE kind = ?1
                 ORDER BY updated_at DESC, name ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![kind], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    })
}

pub(crate) fn dynamic_catalog_contains(
    state: &AppState,
    kind: &str,
    name: &str,
) -> Result<bool, String> {
    let normalized = sanitize_dynamic_item_name(name)
        .ok_or_else(|| format!("invalid {} name: {}", kind, name))?;
    with_db(state, |conn| {
        conn.query_row(
            "SELECT 1 FROM dynamic_catalog_entries WHERE kind = ?1 AND name = ?2 LIMIT 1",
            params![kind, &normalized],
            |_row| Ok(()),
        )
        .optional()
        .map_err(|e| e.to_string())
        .map(|v| v.is_some())
    })
}

pub(crate) fn list_enabled_feature_flags(
    state: &AppState,
    scope: &str,
    prefix: &str,
) -> Vec<String> {
    list_shared_context_internal(state, scope)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            if !entry.key.starts_with(prefix) || !entry.value.eq_ignore_ascii_case("enabled") {
                return None;
            }
            Some(entry.key.trim_start_matches(prefix).to_string())
        })
        .collect()
}

pub(crate) fn resolve_model_runtime(runtime_kind: Option<&str>) -> String {
    runtime_kind
        .and_then(crate::assistant::normalize_runtime_key)
        .unwrap_or("mock")
        .to_string()
}

pub(crate) fn merge_model_lists(
    mut discovered: Vec<String>,
    configured: Vec<String>,
) -> Vec<String> {
    discovered.extend(configured);
    discovered.sort_unstable();
    discovered.dedup();
    discovered
}

pub(crate) fn list_models_for_runtime(
    state: &AppState,
    runtime: &str,
) -> Result<Vec<String>, String> {
    let configured = list_dynamic_catalog(state, "model")?;
    let discovered = acp::list_discovered_models(runtime);
    Ok(merge_model_lists(discovered, configured))
}

pub(crate) fn list_all_known_models(state: &AppState) -> Vec<String> {
    let configured = list_dynamic_catalog(state, "model").unwrap_or_default();
    let mut discovered = Vec::new();
    for runtime in KNOWN_RUNTIME_KEYS {
        discovered.extend(acp::list_discovered_models(runtime));
    }
    merge_model_lists(discovered, configured)
}
