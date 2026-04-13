use crate::types::{AppState, ChatCommandResult};
use serde_json::json;

pub(crate) fn handle_fallback_command(
    tokens: &[&str],
    _state: &AppState,
    _app_session_id_ref: Option<&str>,
    result: &mut ChatCommandResult,
) -> Result<bool, String> {
    match tokens {
        ["/app_team", ..] => {
            result.ok = false;
            result.message = "workspace commands are managed automatically.".to_string();
            result.payload = json!({});
            Ok(true)
        }
        _ => Ok(false),
    }
}
