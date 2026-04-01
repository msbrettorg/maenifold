use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use serde_json::Value;
use std::fs;

// T-12.0: RTM NFR-1 consolidates shared workflow helpers used by queue, status, and submachine.

// PARITY-WORK001: RTM NFR-1 — pretty-printed JSON with 4-space extra indent matches
// C# GetRawText() output. C# preserves the source file indentation where step
// properties sit at 6 spaces (2-space base indent + 4 for array nesting). Rust
// re-serialises via serde, so we pretty-print (2-space) then pad every line after
// the opening brace with 4 extra spaces to recreate the same visual layout.
pub(crate) fn format_workflow_step(step: &Value) -> Result<String> {
    let pretty = serde_json::to_string_pretty(step)?;
    let mut result = String::with_capacity(pretty.len() + pretty.lines().count() * 4);
    for (i, line) in pretty.lines().enumerate() {
        if i > 0 {
            result.push('\n');
            if !line.is_empty() {
                result.push_str("    ");
            }
        }
        result.push_str(line);
    }
    Ok(result)
}

pub(crate) fn build_workflow_response_content(response: &str, thoughts: Option<&str>) -> String {
    let mut content = String::new();
    content.push_str(response);
    if let Some(thoughts) = thoughts.filter(|value| !value.is_empty()) {
        content.push_str("\n\n");
        content.push_str(&format!("*Thoughts: {thoughts}*"));
    }
    content.push_str("\n\n");
    content.push_str(&format!("*{}*", crate::sessions::display_utc_now()));
    content
}

pub(crate) fn parse_workflow_identifiers(value: Option<&Value>) -> Result<Option<Vec<String>>> {
    let Some(value) = value else {
        return Ok(None);
    };

    let ids = match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else if trimmed.starts_with('[') {
                serde_json::from_str::<Vec<String>>(trimmed)?
            } else {
                vec![trimmed.to_string()]
            }
        }
        Value::Array(values) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                MaenifoldError::InvalidArgument("Workflow identifiers must be strings".to_string())
            })?,
        _ => {
            return Err(MaenifoldError::InvalidArgument(
                "Workflow identifiers must be a string or string array".to_string(),
            ));
        }
    };

    if ids.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "At least one workflow identifier is required".to_string(),
        ));
    }

    Ok(Some(ids))
}

pub(crate) fn read_workflow_definition(config: &Config, workflow_id: &str) -> Result<Value> {
    let workflow_path = config
        .assets_path
        .join("workflows")
        .join(format!("{workflow_id}.json"));
    let workflow_text = fs::read_to_string(&workflow_path)
        .map_err(|_| MaenifoldError::Message(format!("Workflow '{workflow_id}' not found")))?;
    Ok(serde_json::from_str(&workflow_text)?)
}

pub(crate) fn workflow_steps<'a>(
    workflow_value: &'a Value,
    workflow_id: &str,
) -> Result<&'a Vec<Value>> {
    workflow_value
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "Workflow '{workflow_id}' is missing 'steps' property"
            ))
        })
}

pub(crate) fn set_current_step(
    frontmatter: &mut crate::markdown::Frontmatter,
    current_step: usize,
) {
    frontmatter.insert(
        "currentStep".to_string(),
        serde_yaml::Value::Number(serde_yaml::Number::from(current_step as i64)),
    );
}
