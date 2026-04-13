use serde_json::{json, Value};

use crate::db::context::{clear_shared_context_internal, list_shared_context_internal, set_shared_context_internal};
use crate::types::AppState;

pub(crate) fn set_shared_context(state: &AppState, params: Value) -> Result<Value, String> {
    let scope = params.get("scope").and_then(|v| v.as_str()).unwrap_or("global");
    let key = params.get("key").and_then(|v| v.as_str()).ok_or("key is required")?;
    let value = params.get("value").and_then(|v| v.as_str()).ok_or("value is required")?;
    set_shared_context_internal(state, scope, key, value)?;
    Ok(json!(format!("Context set: [{scope}] {key}")))
}

pub(crate) fn get_shared_context(state: &AppState, params: Value) -> Result<Value, String> {
    let scope = params.get("scope").and_then(|v| v.as_str()).unwrap_or("global");
    let entries = list_shared_context_internal(state, scope)?;
    let out: Vec<Value> = entries.iter().map(|e| json!({ "key": e.key, "value": e.value })).collect();
    Ok(json!(out))
}

pub(crate) fn delete_shared_context(state: &AppState, params: Value) -> Result<Value, String> {
    let scope = params.get("scope").and_then(|v| v.as_str()).unwrap_or("global");
    let key = params.get("key").and_then(|v| v.as_str()).ok_or("key is required")?;
    clear_shared_context_internal(state, scope, key)?;
    Ok(json!(format!("Context deleted: [{scope}] {key}")))
}
