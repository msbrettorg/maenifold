use crate::assistant::normalize_runtime_key;
use crate::db::role::{list_all_roles, resolve_role_runtime, upsert_role};
use crate::types::{AppState, ChatCommandResult};
use serde_json::json;

pub(crate) fn handle_role_template_command(
    tokens: &[&str],
    state: &AppState,
    _app_session_id: Option<&str>,
    result: &mut ChatCommandResult,
) -> Result<bool, String> {
    if !matches!(tokens.first(), Some(&"/app_role")) {
        return Ok(false);
    }

    match tokens {
        ["/app_role", "list"] => {
            let roles = list_all_roles(state)?;
            result.message = format!("{} roles", roles.len());
            result.payload = json!({ "roles": roles });
        }
        ["/app_role", "bind", role_name, runtime_kind, prompt @ ..] => {
            let normalized_runtime = normalize_runtime_key(runtime_kind)
                .map(|v| v.to_string())
                .unwrap_or_else(|| (*runtime_kind).to_string());
            let role = upsert_role(
                state,
                (*role_name).to_string(),
                normalized_runtime.clone(),
                if prompt.is_empty() {
                    "default-system-prompt".to_string()
                } else {
                    prompt.join(" ")
                },
                None,
                None,
                None,
                None,
                None,
                None,
            )?;
            result.message = format!("role bound: {}", role.role_name);
            result.payload = json!({ "role": role });
        }
        ["/app_role", "prompt", role_name, prompt @ ..] => {
            let runtime = resolve_role_runtime(state, role_name)?;
            let role = upsert_role(
                state,
                (*role_name).to_string(),
                runtime,
                prompt.join(" "),
                None,
                None,
                None,
                None,
                None,
                None,
            )?;
            result.message = format!("role prompt updated: {}", role.role_name);
            result.payload = json!({ "role": role });
        }
        _ => {
            result.ok = false;
            result.message = "unsupported command".to_string();
        }
    }

    Ok(true)
}
