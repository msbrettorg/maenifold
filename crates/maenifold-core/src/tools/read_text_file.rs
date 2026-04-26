use super::context::{ToolContext, render, required_string};
use crate::api::routes_data::resolve_requested_file_path;
use crate::error::{MaenifoldError, Result};
use serde_json::{Value, json};

pub fn read_text_file(context: &ToolContext, payload: &Value) -> Result<String> {
    let path = required_string(payload, "path")?;
    if path.trim().is_empty() {
        return Err(MaenifoldError::InvalidArgument("path must not be empty".into()));
    }
    let cwd = payload.get("cwd").and_then(Value::as_str);
    let resolved_path = resolve_requested_file_path(path, cwd)
        .map_err(|error| MaenifoldError::InvalidArgument(error.to_string()))?;

    let content = std::fs::read_to_string(&resolved_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            MaenifoldError::Message(format!("File not found: {}", resolved_path.to_string_lossy()))
        } else {
            MaenifoldError::Io(error)
        }
    })?;

    Ok(render(
        context.output_format,
        json!({
            "path": resolved_path.to_string_lossy().into_owned(),
            "content": content,
        }),
    ))
}
