use super::context::{ToolContext, load_help_doc};
use crate::error::{MaenifoldError, Result};
use crate::util::json::{OutputFormat, success};
use serde_json::Value;

pub fn list_sessions(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "ListSessions")
            .unwrap_or_else(|| "ERROR: Help file not found for ListSessions".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    Err(MaenifoldError::Message(
        "ACP session listing not yet wired to Jockey DB".to_string(),
    ))
}
