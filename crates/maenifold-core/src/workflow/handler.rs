use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    append_h2_section, extract_wikilinks, file_exists, read_markdown, write_markdown,
};
use crate::sessions::{load_help_doc, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};

use super::{
    ResumeWorkflowResult, append_workflow_queue, build_workflow_response_content,
    format_workflow_step, frontmatter_usize, handle_cancelled_workflow_status,
    handle_completed_workflow_status, handle_queue_exhaustion, maybe_resume_workflow_submachine,
    parse_workflow_identifiers, read_workflow_definition, read_workflow_queue,
    register_workflow_submachine, set_current_step, start_workflow_session,
    transition_to_next_workflow, view_workflow_session, workflow_steps,
};

// T-12.0: RTM NFR-1 keeps Workflow request dispatch and step advancement in a focused handler.

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

    // T-15.14: RTM FR-8 — C# throws InvalidOperationException; Rust must return Err, not Ok.
    Err(MaenifoldError::Message(
        "Invalid parameter combination.\n\n\
         VALID OPERATIONS:\n  \
         Start new workflow: workflowId only\n  \
         Continue session: sessionId + response\n  \
         View queue: sessionId + view=true\n  \
         Append to queue: sessionId + append\n\n\
         TIP: view=true and append both require a sessionId to identify which session to operate on."
            .to_string(),
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

pub(crate) fn validate_workflow_response_concepts(
    response: &str,
    thoughts: Option<&str>,
) -> Option<String> {
    let total_concepts = extract_wikilinks(response).len()
        + thoughts.map(extract_wikilinks).unwrap_or_default().len();
    if total_concepts == 0 {
        return Some(
            "ERROR: Response or thoughts must contain at least one [[WikiLink]] in double brackets.\nExample: 'Implementing [[REST API]] using [[ASP.NET Core]]'\nThis ensures your workflow contributions build the knowledge graph.".to_string(),
        );
    }
    None
}
