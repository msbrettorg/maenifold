use crate::emitter::Emitter;
mod assistant_cmd;
mod catalog_cmd;
pub(crate) mod completion;
mod context_cmd;
mod fallback_cmd;
mod query_cmd;
mod role_templates;
pub(crate) mod runtime_cmd;
pub(crate) mod session_context_cmd;
mod shell_cmd;

use crate::types::*;
use serde_json::{json, Value};

pub(crate) fn split_tokens(input: &str) -> Vec<&str> {
    input.split_whitespace().collect()
}

pub(crate) fn enrich_command_message(base: &str, payload: &Value) -> String {
    let Value::Object(map) = payload else {
        return base.to_string();
    };
    if map.is_empty() {
        return base.to_string();
    }
    if let Some(Value::String(help)) = map.get("help") {
        return format!("{base}\n{help}");
    }
    let detail = serde_json::to_string_pretty(payload).unwrap_or_else(|_| payload.to_string());
    format!("{base}\n{detail}")
}

type ExternalCommandHandler =
    fn(&[&str], &AppState, Option<&str>, &mut ChatCommandResult) -> Result<bool, String>;

fn dispatch_external_handlers(
    event_tx: &crate::EventSender,
    tokens: &[&str],
    state: &AppState,
    app_session_id_ref: Option<&str>,
    result: &mut ChatCommandResult,
) -> Result<bool, String> {
    let handlers: [ExternalCommandHandler; 7] = [
        role_templates::handle_role_template_command,
        shell_cmd::handle_shell_command,
        assistant_cmd::handle_assistant_command,
        catalog_cmd::handle_catalog_command,
        context_cmd::handle_context_command,
        query_cmd::handle_query_command,
        fallback_cmd::handle_fallback_command,
    ];
    for handler in handlers {
        if handler(tokens, state, app_session_id_ref, result)? {
            result.message = enrich_command_message(&result.message, &result.payload);
            event_tx.emit("command/applied", result.clone())
                .map_err(|e| e.to_string())?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) async fn apply_chat_command(
    event_tx: crate::EventSender,
    state: &AppState,
    input: String,
    runtime_kind: Option<String>,
    app_session_id: Option<String>,
) -> Result<ChatCommandResult, String> {
    let trimmed = input.trim();
    if !trimmed.starts_with("/app_") {
        return Err("app commands must start with /app".to_string());
    }

    let tokens = split_tokens(trimmed);
    if tokens.is_empty() {
        return Err("empty command".to_string());
    }

    let mut result = ChatCommandResult {
        ok: true,
        message: "ok".to_string(),
        runtime_kind: runtime_kind.clone(),
        session_id: None,
        payload: json!({}),
    };

    let app_session_id_ref = app_session_id.as_deref().filter(|s| !s.trim().is_empty());

    if dispatch_external_handlers(
        &event_tx,
        tokens.as_slice(),
        state,
        app_session_id_ref,
        &mut result,
    )? {
        return Ok(result);
    }

    result.ok = false;
    result.message = "unsupported command".to_string();
    result.message = enrich_command_message(&result.message, &result.payload);

    event_tx.emit("command/applied", result.clone())
        .map_err(|e| e.to_string())?;
    Ok(result)
}
