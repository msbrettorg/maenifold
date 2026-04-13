use crate::assistant::{cached_assistant_catalog, normalize_runtime_key};
use crate::types::{AppState, ChatCommandResult};
use serde_json::json;

pub(crate) fn handle_assistant_command(
    tokens: &[&str],
    _state: &AppState,
    _app_session_id_ref: Option<&str>,
    result: &mut ChatCommandResult,
) -> Result<bool, String> {
    match tokens {
        ["/app_assistant", "list"] => {
            let assistants = cached_assistant_catalog();
            result.message = "assistant list".to_string();
            result.payload = json!({ "assistants": assistants });
            Ok(true)
        }
        ["/app_assistant", "select", runtime] => {
            match normalize_runtime_key(runtime) {
                Some(normalized) => {
                    result.runtime_kind = Some(normalized.to_string());
                    result.message = format!("assistant selected: {}", normalized);
                    result.payload = json!({ "assistant": normalized });
                }
                None => {
                    result.ok = false;
                    result.message = format!("unsupported assistant: {}", runtime);
                    result.payload = json!({ "assistant": runtime });
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}
