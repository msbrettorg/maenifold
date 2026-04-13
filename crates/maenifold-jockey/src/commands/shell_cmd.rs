use crate::db::app_session::{get_app_session_cwd, set_app_session_cwd};
use crate::types::{AppState, ChatCommandResult};
use crate::{build_jockey_tool_prompt, resolve_chat_cwd};
use serde_json::json;

pub(crate) fn handle_shell_command(
    tokens: &[&str],
    state: &AppState,
    app_session_id_ref: Option<&str>,
    result: &mut ChatCommandResult,
) -> Result<bool, String> {
    match tokens {
        ["/app_help"] => {
            result.message = "command list".to_string();
            result.payload = json!({ "help": build_jockey_tool_prompt() });
            Ok(true)
        }
        ["/app_cd"] => {
            let cwd = if let Some(sid) = app_session_id_ref {
                get_app_session_cwd(state, sid).unwrap_or_else(resolve_chat_cwd)
            } else {
                resolve_chat_cwd()
            };
            result.message = format!("cwd: {}", cwd);
            result.payload = json!({ "cwd": cwd });
            Ok(true)
        }
        ["/app_cd", path] => {
            let resolved = crate::abs_cwd(path);
            if !std::path::Path::new(&resolved).is_dir() {
                return Err(format!("not a directory: {}", resolved));
            }
            if let Some(sid) = app_session_id_ref {
                set_app_session_cwd(state, sid, &resolved)?;
            }
            result.message = format!("cwd changed: {}", resolved);
            result.payload = json!({ "cwd": resolved });
            Ok(true)
        }
        _ => Ok(false),
    }
}
