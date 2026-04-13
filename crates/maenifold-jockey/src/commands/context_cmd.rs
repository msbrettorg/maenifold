use crate::db::context::list_shared_context_internal;
use crate::db::session_context::{app_session_scope, list_shared_context_prefix_internal};
use crate::types::{AppState, ChatCommandResult};
use serde_json::json;

fn required_app_session_id(app_session_id: Option<&str>) -> Result<&str, String> {
    app_session_id
        .filter(|sid| !sid.trim().is_empty())
        .ok_or_else(|| "app session id required".to_string())
}

pub(crate) fn handle_context_command(
    tokens: &[&str],
    state: &AppState,
    app_session_id_ref: Option<&str>,
    result: &mut ChatCommandResult,
) -> Result<bool, String> {
    match tokens {
        ["/app_context", "list"] | ["/app_context", "list", ..] => {
            let sid = required_app_session_id(app_session_id_ref)?;
            let prefix = app_session_scope(sid);
            let scoped_prefix = format!("{prefix}:");
            let scope = tokens.get(2).copied().unwrap_or("");
            let entries = if scope.is_empty() {
                list_shared_context_prefix_internal(state, &prefix)?
                    .into_iter()
                    .filter(|entry| {
                        entry.scope == prefix || entry.scope.starts_with(&scoped_prefix)
                    })
                    .collect()
            } else {
                if !(scope == prefix || scope.starts_with(&scoped_prefix)) {
                    return Err(format!("scope must stay within {}", prefix));
                }
                list_shared_context_internal(state, scope)?
            };
            result.message = format!(
                "{} context entries{}",
                entries.len(),
                if scope.is_empty() {
                    String::new()
                } else {
                    format!(" (scope: {})", scope)
                }
            );
            result.payload = json!({ "entries": entries });
            Ok(true)
        }
        _ => Ok(false),
    }
}
