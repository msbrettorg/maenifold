use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, append_h2_section, file_exists, read_markdown, write_markdown};
use crate::sessions::{display_utc_now, workflow_session_uri};
use crate::tools::ToolContext;
use crate::workflow::{
    emit_workflow_resource_updated, format_workflow_step, parse_workflow_identifiers,
    read_workflow_definition, workflow_steps,
};
use chrono::Utc;
use serde_json::{Value, json};
use serde_yml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

// T-12.0: RTM NFR-1 keeps workflow session creation, queue persistence, and frontmatter parsing in a focused submodule.

pub(crate) fn start_workflow_session(
    context: &ToolContext,
    workflow_ids: &[String],
    inputs: Option<HashMap<String, Value>>,
) -> Result<String> {
    let workflow_id = workflow_ids
        .first()
        .ok_or(MaenifoldError::MissingProperty("workflowId"))?;
    // T-16.3: RTM FR-25.2 — surface missing workflows as MCP errors before creating or mutating queue state.
    let workflow_value = match read_workflow_definition(&context.config, workflow_id) {
        Ok(value) => value,
        Err(MaenifoldError::Message(msg)) if msg.contains("not found") => {
            return Err(MaenifoldError::Message(format!(
                "Workflow '{workflow_id}' not found"
            )));
        }
        Err(other) => return Err(other),
    };
    let steps = workflow_steps(&workflow_value, workflow_id)?;
    if steps.is_empty() {
        return Err(MaenifoldError::Message(format!(
            "Workflow '{workflow_id}' has no steps"
        )));
    }

    let session_id = generate_workflow_session_id();
    let first_step = steps.first().cloned().unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&first_step)?;
    let uri = workflow_session_uri(&session_id);
    let frontmatter = initial_workflow_frontmatter(&session_id, workflow_ids, "active", inputs);

    let file_content = format!(
        "# Workflow Session\n\nStarted: {}\n\n## Step 1/{}\n\n{}\n",
        display_utc_now(),
        steps.len(),
        rendered_step,
    );
    write_markdown(&uri, Some(&frontmatter), &file_content, &context.config)?;
    emit_workflow_resource_updated(&session_id);

    Ok(json!({
        "sessionId": session_id,
        "message": format!("Created session: {session_id}"),
    })
    .to_string())
}

pub(crate) fn view_workflow_session(context: &ToolContext, session_id: &str) -> Result<String> {
    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} doesn't exist"
        )));
    }

    let (frontmatter, _, _) = read_markdown(&uri, &context.config)?;
    let frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let current_step = frontmatter_usize(&frontmatter, "currentStep");
    let current_workflow_id = queue
        .get(current_workflow_index)
        .ok_or_else(|| MaenifoldError::Message("Workflow queue is empty".to_string()))?;
    let workflow_value = read_workflow_definition(&context.config, current_workflow_id)?;
    let steps = workflow_steps(&workflow_value, current_workflow_id)?;

    let mut result = String::new();
    result.push_str(&format!("Queue: [{}]\n", queue.join(", ")));
    result.push_str(&format!(
        "Position: {} (workflow {}/{}, step {}/{})\n",
        current_workflow_id,
        current_workflow_index + 1,
        queue.len(),
        current_step + 1,
        steps.len(),
    ));

    if current_step + 1 < steps.len() {
        let next = steps[current_step + 1]
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Unnamed step");
        result.push_str(&format!("Next: {next}\n"));
    } else if current_workflow_index + 1 < queue.len() {
        result.push_str(&format!(
            "Next workflow: {}\n",
            queue[current_workflow_index + 1]
        ));
    } else {
        result.push_str("Next: Queue will complete\n");
    }

    Ok(result)
}

pub(crate) fn append_workflow_queue(
    context: &ToolContext,
    session_id: &str,
    append: &Value,
) -> Result<String> {
    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} doesn't exist"
        )));
    }

    let (frontmatter, content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;
    let mut queue = read_workflow_queue(&frontmatter);
    let to_append = parse_workflow_identifiers(Some(append))?
        .ok_or(MaenifoldError::MissingProperty("append"))?;

    // T-16.3: RTM FR-25.2 — validate all appended workflow IDs exist before mutating queue state.
    for workflow_id in &to_append {
        match read_workflow_definition(&context.config, workflow_id) {
            Ok(_) => {}
            Err(MaenifoldError::Message(msg)) if msg.contains("not found") => {
                return Err(MaenifoldError::Message(format!(
                    "Workflow '{workflow_id}' not found"
                )));
            }
            Err(other) => return Err(other),
        }
    }

    // T-HSM-001.1d: RTM FR-10.5 - preserve serial workflow queue semantics.
    queue.extend(to_append.iter().cloned());
    frontmatter.insert("queue".to_string(), yaml_sequence_from_strings(&queue));
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)?;

    let append_content = format!(
        "Added workflows to queue: {}\n*{}*",
        to_append.join(", "),
        display_utc_now(),
    );
    append_h2_section(&uri, "Queue Updated", &append_content, &context.config)?;
    emit_workflow_resource_updated(session_id);

    Ok(format!(
        "Added {} workflow(s) to queue. New queue: [{}]",
        to_append.len(),
        queue.join(", "),
    ))
}

pub(crate) fn read_workflow_queue(frontmatter: &Frontmatter) -> Vec<String> {
    match frontmatter.get("queue") {
        Some(YamlValue::Sequence(values)) => values
            .iter()
            .filter_map(YamlValue::as_str)
            .map(str::to_string)
            .collect(),
        Some(YamlValue::String(value)) => {
            let trimmed = value.trim();
            if trimmed.starts_with('[') {
                serde_json::from_str::<Vec<String>>(trimmed).unwrap_or_default()
            } else if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
        _ => Vec::new(),
    }
}

pub(crate) fn yaml_sequence_from_strings(values: &[String]) -> YamlValue {
    YamlValue::Sequence(
        values
            .iter()
            .cloned()
            .map(YamlValue::String)
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn frontmatter_usize(frontmatter: &Frontmatter, key: &str) -> usize {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            YamlValue::Number(number) => number.as_u64().map(|value| value as usize),
            YamlValue::String(text) => text.parse::<usize>().ok(),
            _ => None,
        })
        .unwrap_or(0)
}

pub(crate) fn initial_workflow_frontmatter(
    session_id: &str,
    workflow_ids: &[String],
    status: &str,
    inputs: Option<HashMap<String, Value>>,
) -> Frontmatter {
    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Workflow Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    frontmatter.insert(
        "queue".to_string(),
        yaml_sequence_from_strings(workflow_ids),
    );
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(serde_yml::Number::from(0)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yml::Number::from(0)),
    );
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    if let Some(inputs) = inputs {
        if !inputs.is_empty() {
            if let Ok(value) = serde_yml::to_value(inputs) {
                frontmatter.insert("inputs".to_string(), value);
            }
        }
    }
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineTarget".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter
}

fn generate_workflow_session_id() -> String {
    format!("workflow-{}", Utc::now().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::start_workflow_session;
    use crate::{config::Config, test_util::ConfigResetGuard, tools::ToolContext, util::json::OutputFormat};
    use serde_json::Value;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn workflow_start_returns_structured_session_id_json() {
        let temp = TempDir::new().expect("temp dir");
        Config::override_root(temp.path());
        Config::set_database_path(temp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        std::fs::create_dir_all(config.assets_path.join("workflows")).expect("create workflows dir");
        std::fs::write(config.assets_path.join("workflows/test.json"), r#"{"steps":[{"name":"Step 1"}]}"#)
            .expect("write workflow");
        let _guard = ConfigResetGuard;
        let response = start_workflow_session(&ToolContext::new(config, OutputFormat::Markdown), &["test".to_string()], None)
            .expect("start workflow");
        let value: Value = serde_json::from_str(&response).expect("workflow start should return JSON");
        assert!(value.get("sessionId").and_then(Value::as_str).is_some());
    }
}
