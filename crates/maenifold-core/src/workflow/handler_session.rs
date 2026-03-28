use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    append_h2_section, extract_wikilinks, file_exists, read_markdown, write_markdown,
};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;

use super::{
    ResumeWorkflowResult, build_workflow_response_content, format_workflow_step, frontmatter_usize,
    handle_cancelled_workflow_status, handle_completed_workflow_status, handle_queue_exhaustion,
    maybe_resume_workflow_submachine, read_workflow_definition, read_workflow_queue,
    register_workflow_submachine, set_current_step, transition_to_next_workflow, workflow_steps,
};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn continue_workflow_session(
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
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} doesn't exist"
        )));
    }

    let (frontmatter, existing_content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;
    frontmatter =
        gate_workflow_session_status(context, &uri, session_id, frontmatter, &existing_content)?;

    match maybe_resume_workflow_submachine(context, &uri, frontmatter, &existing_content)? {
        ResumeWorkflowResult::Ready(updated_frontmatter) => frontmatter = updated_frontmatter,
        ResumeWorkflowResult::Waiting(message) => return Ok(message),
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
    set_current_step(&mut frontmatter, current_step);
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

fn gate_workflow_session_status(
    context: &ToolContext,
    uri: &str,
    session_id: &str,
    mut frontmatter: crate::markdown::Frontmatter,
    existing_content: &str,
) -> Result<crate::markdown::Frontmatter> {
    let status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .unwrap_or("active");

    match status {
        "pending" => Err(MaenifoldError::Message(format!(
            "Session {session_id} is pending — cannot continue until activated"
        ))),
        "completed" => Err(MaenifoldError::Message(format!(
            "Session {session_id} is already completed"
        ))),
        "abandoned" => Err(MaenifoldError::Message(format!(
            "Session {session_id} is abandoned — use restart to create a new attempt"
        ))),
        "ready" => {
            frontmatter.insert(
                "status".to_string(),
                YamlValue::String("active".to_string()),
            );
            write_markdown(uri, Some(&frontmatter), existing_content, &context.config)?;
            Ok(frontmatter)
        }
        _ => Ok(frontmatter),
    }
}

#[allow(clippy::too_many_arguments)]
fn advance_to_next_workflow_step(
    context: &ToolContext,
    workflow_uri: &str,
    frontmatter: crate::markdown::Frontmatter,
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
        rendered_step,
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
