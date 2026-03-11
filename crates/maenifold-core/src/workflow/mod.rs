use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use serde_json::Value;
use std::fs;

mod handler;
mod queue;
mod status;
mod submachine;

// T-12.0: RTM NFR-1 consolidates the workflow facade and helpers under a single directory module.
pub(crate) use handler::*;
pub(crate) use queue::{
    append_workflow_queue, frontmatter_usize, read_workflow_queue, start_workflow_session,
    view_workflow_session,
};
pub(crate) use status::{
    handle_cancelled_workflow_status, handle_completed_workflow_status, handle_queue_exhaustion,
    transition_to_next_workflow,
};
pub(crate) use submachine::{
    ResumeWorkflowResult, maybe_resume_workflow_submachine, register_workflow_submachine,
};

// PARITY-WORK001: RTM NFR-1 — pretty-printed JSON with 4-space extra indent matches
// C# GetRawText() output.  C# preserves the source file indentation where step
// properties sit at 6 spaces (2-space base indent + 4 for array nesting).  Rust
// re-serialises via serde, so we pretty-print (2-space) then pad every line after
// the opening brace with 4 extra spaces to recreate the same visual layout.
pub(crate) fn format_workflow_step(step: &Value) -> Result<String> {
    let pretty = serde_json::to_string_pretty(step)?;
    let mut result = String::with_capacity(pretty.len() + pretty.lines().count() * 4);
    for (i, line) in pretty.lines().enumerate() {
        if i > 0 {
            result.push('\n');
            if !line.is_empty() {
                result.push_str("    "); // 4-space extra indent
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

// PARITY-WORK001: RTM NFR-1 — unit tests for pretty-printed JSON workflow step content.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_workflow_step_produces_pretty_json_with_indent() {
        let step = json!({
            "id": "step-1",
            "name": "Step 1",
            "description": "A test step for [[workflow]] coverage"
        });
        let output = format_workflow_step(&step).expect("format step");

        // Pretty JSON must span multiple lines
        assert!(
            output.lines().count() > 1,
            "pretty JSON must be multi-line, got: {output:?}"
        );

        // Must contain 6-space indented properties (2 base + 4 extra)
        assert!(
            output.contains("\n      \"id\""),
            "properties must be indented 6 spaces: {output:?}"
        );

        // Must be valid JSON and roundtrip correctly
        let parsed: Value = serde_json::from_str(&output).expect("valid JSON roundtrip");
        assert_eq!(parsed["id"], "step-1");
        assert_eq!(parsed["name"], "Step 1");
    }

    #[test]
    fn format_workflow_step_empty_object_is_braces() {
        let step = json!({});
        let output = format_workflow_step(&step).expect("format empty step");
        assert_eq!(output, "{}", "empty object must be compact: {output:?}");
    }

    #[test]
    fn format_workflow_step_nested_object_has_deeper_indent() {
        let step = json!({
            "id": "step-2",
            "name": "Step 2",
            "metadata": {
                "sequentialThinking": true,
                "tags": ["research", "analysis"]
            }
        });
        let output = format_workflow_step(&step).expect("format nested step");

        // Multi-line pretty output
        assert!(
            output.lines().count() > 1,
            "nested pretty JSON must be multi-line, got: {output:?}"
        );

        // Nested properties should be at 8 spaces (4 base + 4 extra)
        assert!(
            output.contains("\n        \"sequentialThinking\""),
            "nested properties must be indented 8+ spaces: {output:?}"
        );

        // Roundtrip check
        let parsed: Value = serde_json::from_str(&output).expect("valid JSON roundtrip");
        assert_eq!(parsed["metadata"]["sequentialThinking"], true);
    }

    #[test]
    fn format_workflow_step_closing_brace_at_four_spaces() {
        let step = json!({
            "id": "step-1"
        });
        let output = format_workflow_step(&step).expect("format step");

        // Closing brace should be at 4 spaces indent
        let last_line = output.lines().last().unwrap();
        assert_eq!(last_line, "    }", "closing brace at 4-space indent: {output:?}");
    }

    #[test]
    fn format_workflow_step_with_multiline_description_roundtrips() {
        let step = json!({
            "id": "step-3",
            "name": "Step 3",
            "description": "Line 1\nLine 2\nLine 3"
        });
        let output = format_workflow_step(&step).expect("format multiline desc step");

        // The JSON string roundtrips correctly — newlines in values are escaped as \n
        let parsed: Value = serde_json::from_str(&output).expect("valid JSON roundtrip");
        assert_eq!(parsed["description"], "Line 1\nLine 2\nLine 3");
    }
}
