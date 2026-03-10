use crate::assets::AssetManager;
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, success};
use crate::markdown::{
    Frontmatter, append_h2_section, extract_wikilinks, file_exists, read_markdown, write_markdown,
};
use crate::sessions::{
    date_parts_from_session_id, display_utc_now, frontmatter_string, load_help_doc,
    now_iso8601_roundtrip, workflow_session_uri,
};
use crate::tools::ToolContext;
use chrono::Utc;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;

// T-11.9: RTM FR-10 - extracted Workflow tool handling into a dedicated module.

pub fn workflow(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "Workflow")
            .unwrap_or_else(|| "ERROR: Help file not found for Workflow".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let workflow_ids = parse_workflow_identifiers(payload.get("workflowId"))?;
    let session_id = payload.get("sessionId").and_then(Value::as_str);
    let response = payload.get("response").and_then(Value::as_str);
    let thoughts = payload.get("thoughts").and_then(Value::as_str);
    let status = payload.get("status").and_then(Value::as_str);
    let conclusion = payload.get("conclusion").and_then(Value::as_str);
    let view = payload
        .get("view")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let append = payload.get("append");
    let submachine_session_id = payload.get("submachineSessionId").and_then(Value::as_str);

    if workflow_ids.is_some() && session_id.is_some() {
        return Err(MaenifoldError::Message(format!(
            "Cannot provide both workflowId and sessionId together: workflowId={}, sessionId={}",
            payload
                .get("workflowId")
                .map(Value::to_string)
                .unwrap_or_default(),
            session_id.unwrap_or_default()
        )));
    }

    AssetManager::initialize_assets(&context.config)?;

    if let Some(workflow_ids) = workflow_ids {
        return start_workflow_session(context, &workflow_ids);
    }

    if let Some(session_id) = session_id {
        if view {
            return view_workflow_session(context, session_id);
        }
        if let Some(append) = append {
            return append_workflow_queue(context, session_id, append);
        }
        if let Some(response) = response {
            return continue_workflow_session(
                context,
                session_id,
                response,
                thoughts,
                status,
                conclusion,
                submachine_session_id,
            );
        }
    }

    Ok(
        "Workflow usage:\n\n  Start session: workflowId\n  Continue session: sessionId + response\n  View queue: sessionId + view=true\n  Append to queue: sessionId + append\n\nTIP: view=true and append both require a sessionId to identify which session to operate on.".to_string(),
    )
}

fn format_workflow_step(step: &Value) -> Result<String> {
    let pretty = serde_json::to_string_pretty(step)?;
    let mut lines = pretty.lines();
    let Some(first_line) = lines.next() else {
        return Ok(String::new());
    };
    let mut rendered = String::from(first_line);
    for line in lines {
        rendered.push('\n');
        rendered.push_str("    ");
        rendered.push_str(line);
    }
    Ok(rendered)
}

fn start_workflow_session(context: &ToolContext, workflow_ids: &[String]) -> Result<String> {
    let workflow_id = workflow_ids
        .first()
        .ok_or_else(|| MaenifoldError::MissingProperty("workflowId"))?;
    let workflow_value = read_workflow_definition(&context.config, workflow_id)?;
    let steps = workflow_steps(&workflow_value, workflow_id)?;
    if steps.is_empty() {
        return Ok(format!("ERROR: Workflow '{workflow_id}' has no steps"));
    }

    let session_id = generate_workflow_session_id();
    let first_step = steps.first().cloned().unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&first_step)?;
    let uri = workflow_session_uri(&session_id);

    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Workflow Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "queue".to_string(),
        yaml_sequence_from_strings(workflow_ids),
    );
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );

    let file_content = format!(
        "# Workflow Session\n\nStarted: {}\n\n## Step 1/{}\n\n{}\n",
        display_utc_now(),
        steps.len(),
        rendered_step,
    );
    write_markdown(&uri, Some(&frontmatter), &file_content, &context.config)?;

    Ok(format!(
        "Created session: {session_id}\n\nStep 1/{}\n\n{}",
        steps.len(),
        rendered_step
    ))
}

fn view_workflow_session(context: &ToolContext, session_id: &str) -> Result<String> {
    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Ok(format!("ERROR: Session {session_id} doesn't exist"));
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
        steps.len()
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

fn append_workflow_queue(
    context: &ToolContext,
    session_id: &str,
    append: &Value,
) -> Result<String> {
    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Ok(format!("ERROR: Session {session_id} doesn't exist"));
    }

    let (frontmatter, content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;
    let mut queue = read_workflow_queue(&frontmatter);
    let to_append = parse_workflow_identifiers(Some(append))?
        .ok_or(MaenifoldError::MissingProperty("append"))?;

    for workflow_id in &to_append {
        let _ = read_workflow_definition(&context.config, workflow_id)?;
    }

    // T-HSM-001.1d: RTM FR-10.5 - preserve serial workflow queue semantics.
    queue.extend(to_append.iter().cloned());
    frontmatter.insert("queue".to_string(), yaml_sequence_from_strings(&queue));
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)?;

    let append_content = format!(
        "Added workflows to queue: {}\n*{}*",
        to_append.join(", "),
        display_utc_now()
    );
    append_h2_section(&uri, "Queue Updated", &append_content, &context.config)?;

    Ok(format!(
        "Added {} workflow(s) to queue. New queue: [{}]",
        to_append.len(),
        queue.join(", ")
    ))
}

fn continue_workflow_session(
    context: &ToolContext,
    session_id: &str,
    response: &str,
    thoughts: Option<&str>,
    status: Option<&str>,
    conclusion: Option<&str>,
    submachine_session_id: Option<&str>,
) -> Result<String> {
    if let Some(message) = validate_workflow_response_concepts(response, thoughts) {
        return Ok(message);
    }

    let uri = workflow_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Ok(format!("ERROR: Session {session_id} doesn't exist"));
    }

    let (frontmatter, existing_content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;

    // T-HSM-001.1c: RTM FR-10.4 - block advancement while a submachine remains active.
    if frontmatter_string(&frontmatter, "phase") == "waiting" {
        let active_submachine_session_id =
            frontmatter_string(&frontmatter, "activeSubmachineSessionId");
        let active_submachine_type = frontmatter_string(&frontmatter, "activeSubmachineType");

        if !active_submachine_session_id.is_empty() && !active_submachine_type.is_empty() {
            let submachine_uri =
                session_uri_for_type(&active_submachine_type, &active_submachine_session_id);
            if !file_exists(&submachine_uri, &context.config)? {
                clear_submachine_frontmatter(&mut frontmatter);
                write_markdown(&uri, Some(&frontmatter), &existing_content, &context.config)?;
                append_h2_section(
                    &uri,
                    "Submachine Cleared",
                    &format!(
                        "Registered submachine '{}' session not found (deleted or expired). Clearing waiting state and resuming workflow.\n\n*{}*",
                        active_submachine_session_id,
                        display_utc_now()
                    ),
                    &context.config,
                )?;
                let (updated_frontmatter, _, _) = read_markdown(&uri, &context.config)?;
                frontmatter = updated_frontmatter.unwrap_or(frontmatter);
            } else {
                let (submachine_frontmatter, _, _) =
                    read_markdown(&submachine_uri, &context.config)?;
                let submachine_status = submachine_frontmatter
                    .as_ref()
                    .map(|values| frontmatter_string(values, "status"))
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "active".to_string());

                if matches!(
                    submachine_status.as_str(),
                    "completed" | "cancelled" | "abandoned"
                ) {
                    clear_submachine_frontmatter(&mut frontmatter);
                    let (_, content_after_clear, _) = read_markdown(&uri, &context.config)?;
                    write_markdown(
                        &uri,
                        Some(&frontmatter),
                        &content_after_clear,
                        &context.config,
                    )?;
                    append_h2_section(
                        &uri,
                        "Submachine Resumed",
                        &format!(
                            "Submachine {} {}. Resuming workflow.\n\n*{}*",
                            active_submachine_session_id,
                            submachine_status,
                            display_utc_now()
                        ),
                        &context.config,
                    )?;
                    let (updated_frontmatter, _, _) = read_markdown(&uri, &context.config)?;
                    frontmatter = updated_frontmatter.unwrap_or(frontmatter);
                } else {
                    return Ok(format!(
                        "Workflow is waiting on submachine {active_submachine_type} {active_submachine_session_id}. Complete it first, then call Workflow again."
                    ));
                }
            }
        }
    }

    if let Some(submachine_session_id) = submachine_session_id.filter(|value| !value.is_empty()) {
        return register_workflow_submachine(
            context,
            &uri,
            frontmatter,
            response,
            thoughts,
            submachine_session_id,
        );
    }

    let mut current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let mut current_step = frontmatter_usize(&frontmatter, "currentStep");
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_id = queue
        .get(current_workflow_index)
        .ok_or_else(|| MaenifoldError::Message("Workflow queue is empty".to_string()))?;
    let workflow_value = read_workflow_definition(&context.config, current_workflow_id)?;
    let steps = workflow_steps(&workflow_value, current_workflow_id)?;

    append_h2_section(
        &uri,
        &format!("Step {}/{} Response", current_step + 1, steps.len()),
        &build_workflow_response_content(response, thoughts),
        &context.config,
    )?;

    current_step += 1;
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );
    let (_, existing_content, _) = read_markdown(&uri, &context.config)?;

    if status == Some("completed") {
        return handle_completed_workflow_status(
            context,
            &uri,
            frontmatter,
            current_step,
            conclusion,
        );
    }

    if status == Some("cancelled") {
        return handle_cancelled_workflow_status(
            context,
            &uri,
            frontmatter,
            &existing_content,
            current_step,
        );
    }

    if current_step >= steps.len() {
        current_workflow_index += 1;
        if current_workflow_index >= queue.len() {
            return handle_queue_exhaustion(context, &uri, frontmatter, current_step, conclusion);
        }

        return transition_to_next_workflow(
            context,
            &uri,
            frontmatter,
            &existing_content,
            current_workflow_index,
            &queue,
        );
    }

    advance_to_next_workflow_step(
        context,
        &uri,
        frontmatter,
        &existing_content,
        current_step,
        steps,
    )
}

fn register_workflow_submachine(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    response: &str,
    thoughts: Option<&str>,
    submachine_session_id: &str,
) -> Result<String> {
    let submachine_type = "sequential";
    let submachine_uri = session_uri_for_type(submachine_type, submachine_session_id);
    if !file_exists(&submachine_uri, &context.config)? {
        return Ok(format!(
            "ERROR: Submachine session '{submachine_session_id}' not found under type '{submachine_type}'. Ensure the SequentialThinking session exists before registering."
        ));
    }

    let current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let current_step = frontmatter_usize(&frontmatter, "currentStep");
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_id = queue
        .get(current_workflow_index)
        .ok_or_else(|| MaenifoldError::Message("Workflow queue is empty".to_string()))?;
    let workflow_value = read_workflow_definition(&context.config, current_workflow_id)?;
    let steps = workflow_steps(&workflow_value, current_workflow_id)?;

    append_h2_section(
        workflow_uri,
        &format!("Step {}/{} Response", current_step + 1, steps.len()),
        &build_workflow_response_content(response, thoughts),
        &context.config,
    )?;

    // T-HSM-001.1b: RTM FR-10.1 - register submachine lifecycle state in workflow frontmatter.
    // T-HSM-001.1c: RTM FR-10.4 - do not advance currentStep while waiting on the submachine.
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("waiting".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(submachine_type.to_string()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(submachine_session_id.to_string()),
    );
    let (_, content_before_register, _) = read_markdown(workflow_uri, &context.config)?;
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        &content_before_register,
        &context.config,
    )?;
    append_h2_section(
        workflow_uri,
        "Submachine Registered",
        &format!(
            "Submachine registered: {submachine_type} {submachine_session_id}\n\n*{}*",
            display_utc_now()
        ),
        &context.config,
    )?;

    Ok(format!(
        "Registered submachine {submachine_session_id}. Workflow is waiting. Complete the submachine, then call Workflow with sessionId and response to continue."
    ))
}

fn handle_completed_workflow_status(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    current_step: usize,
    conclusion: Option<&str>,
) -> Result<String> {
    let conclusion = match conclusion {
        Some(conclusion) if !conclusion.is_empty() => conclusion,
        _ => {
            return Ok(
                "ERROR: Conclusion required when completing session. Must synthesize findings with [[WikiLinks]]."
                    .to_string(),
            );
        }
    };

    if extract_wikilinks(conclusion).is_empty() {
        return Ok(
            "ERROR: Conclusion must include [[WikiLinks]] for knowledge graph integration."
                .to_string(),
        );
    }

    append_h2_section(workflow_uri, "Conclusion", conclusion, &context.config)?;
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    frontmatter.insert(
        "completed".to_string(),
        YamlValue::String(now_iso8601_roundtrip()),
    );
    frontmatter.insert(
        "stepCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );

    let (_, content_after_conclusion, _) = read_markdown(workflow_uri, &context.config)?;
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        &content_after_conclusion,
        &context.config,
    )?;
    Ok("✅ Workflow session completed".to_string())
}

fn handle_cancelled_workflow_status(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
    current_step: usize,
) -> Result<String> {
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("cancelled".to_string()),
    );
    frontmatter.insert(
        "cancelled".to_string(),
        YamlValue::String(now_iso8601_roundtrip()),
    );
    frontmatter.insert(
        "stepCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;
    Ok("❌ Workflow session cancelled".to_string())
}

fn handle_queue_exhaustion(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    current_step: usize,
    conclusion: Option<&str>,
) -> Result<String> {
    let conclusion = match conclusion {
        Some(conclusion) if !conclusion.is_empty() => conclusion,
        _ => {
            return Ok(
                "ERROR: All workflows complete. Call workflow tool again with conclusion parameter to finalize."
                    .to_string(),
            );
        }
    };

    if extract_wikilinks(conclusion).is_empty() {
        return Ok(
            "ERROR: Conclusion must include [[WikiLinks]] for knowledge graph integration."
                .to_string(),
        );
    }

    append_h2_section(workflow_uri, "Conclusion", conclusion, &context.config)?;
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    frontmatter.insert(
        "completed".to_string(),
        YamlValue::String(now_iso8601_roundtrip()),
    );
    frontmatter.insert(
        "stepCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );
    let (_, content_after_conclusion, _) = read_markdown(workflow_uri, &context.config)?;
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        &content_after_conclusion,
        &context.config,
    )?;
    Ok("✅ All workflows in queue completed".to_string())
}

fn transition_to_next_workflow(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
    current_workflow_index: usize,
    queue: &[String],
) -> Result<String> {
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_workflow_index as i64)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;

    let next_workflow_id = queue.get(current_workflow_index).ok_or_else(|| {
        MaenifoldError::Message("Workflow queue transition index out of range".to_string())
    })?;
    let workflow_value = read_workflow_definition(&context.config, next_workflow_id)?;
    let next_steps = workflow_steps(&workflow_value, next_workflow_id)?;
    let next_step = next_steps.first().cloned().unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&next_step)?;
    append_h2_section(
        workflow_uri,
        &format!("Step 1/{}", next_steps.len()),
        &rendered_step,
        &context.config,
    )?;

    Ok(format!("Step 1/{}\n\n{}", next_steps.len(), rendered_step))
}

fn advance_to_next_workflow_step(
    context: &ToolContext,
    workflow_uri: &str,
    frontmatter: Frontmatter,
    existing_content: &str,
    current_step: usize,
    steps: &[Value],
) -> Result<String> {
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;
    let next_step = steps
        .get(current_step)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&next_step)?;
    append_h2_section(
        workflow_uri,
        &format!("Step {}/{}", current_step + 1, steps.len()),
        &rendered_step,
        &context.config,
    )?;
    Ok(format!(
        "Step {}/{}\n\n{}",
        current_step + 1,
        steps.len(),
        rendered_step
    ))
}

fn validate_workflow_response_concepts(response: &str, thoughts: Option<&str>) -> Option<String> {
    let total_concepts = extract_wikilinks(response).len()
        + thoughts.map(extract_wikilinks).unwrap_or_default().len();
    if total_concepts == 0 {
        return Some(
            "ERROR: Response or thoughts must contain at least one [[WikiLink]] in double brackets.\nExample: 'Implementing [[REST API]] using [[ASP.NET Core]]'\nThis ensures your workflow contributions build the knowledge graph.".to_string(),
        );
    }
    None
}

fn build_workflow_response_content(response: &str, thoughts: Option<&str>) -> String {
    let mut content = String::new();
    content.push_str(response);
    if let Some(thoughts) = thoughts.filter(|value| !value.is_empty()) {
        content.push_str("\n\n");
        content.push_str(&format!("*Thoughts: {thoughts}*"));
    }
    content.push_str("\n\n");
    content.push_str(&format!("*{}*", display_utc_now()));
    content
}

fn parse_workflow_identifiers(value: Option<&Value>) -> Result<Option<Vec<String>>> {
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

fn read_workflow_definition(config: &Config, workflow_id: &str) -> Result<Value> {
    let workflow_path = config
        .assets_path
        .join("workflows")
        .join(format!("{workflow_id}.json"));
    let workflow_text = fs::read_to_string(&workflow_path)
        .map_err(|_| MaenifoldError::Message(format!("Workflow '{workflow_id}' not found")))?;
    Ok(serde_json::from_str(&workflow_text)?)
}

fn workflow_steps<'a>(workflow_value: &'a Value, workflow_id: &str) -> Result<&'a Vec<Value>> {
    workflow_value
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "Workflow '{workflow_id}' is missing 'steps' property"
            ))
        })
}

fn read_workflow_queue(frontmatter: &Frontmatter) -> Vec<String> {
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

fn yaml_sequence_from_strings(values: &[String]) -> YamlValue {
    YamlValue::Sequence(
        values
            .iter()
            .cloned()
            .map(YamlValue::String)
            .collect::<Vec<_>>(),
    )
}

fn frontmatter_usize(frontmatter: &Frontmatter, key: &str) -> usize {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            YamlValue::Number(number) => number.as_u64().map(|value| value as usize),
            YamlValue::String(text) => text.parse::<usize>().ok(),
            _ => None,
        })
        .unwrap_or(0)
}

fn clear_submachine_frontmatter(frontmatter: &mut Frontmatter) {
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );
}

fn session_uri_for_type(session_type: &str, session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/{session_type}/{year:04}/{month:02}/{day:02}/{session_id}")
}

fn generate_workflow_session_id() -> String {
    format!("workflow-{}", Utc::now().timestamp_millis())
}
