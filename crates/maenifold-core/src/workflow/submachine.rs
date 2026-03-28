use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, append_h2_section, file_exists, read_markdown, write_markdown};
use crate::sessions::{date_parts_from_session_id, display_utc_now};
use crate::tools::ToolContext;
use crate::workflow::{build_workflow_response_content, read_workflow_definition, workflow_steps};
use serde_yaml::Value as YamlValue;

use super::{frontmatter_usize, read_workflow_queue};

// T-12.0: RTM NFR-1 keeps submachine registration and waiting/resume transitions isolated from queue helpers.

pub(crate) enum ResumeWorkflowResult {
    Ready(Frontmatter),
    Waiting(String),
}

#[allow(clippy::too_many_lines)]
pub(crate) fn maybe_resume_workflow_submachine(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
) -> Result<ResumeWorkflowResult> {
    // T-12.0: RTM FR-10 - resume only sessions that are marked waiting and still reference an active submachine.
    if crate::sessions::frontmatter_string(&frontmatter, "phase") != "waiting" {
        return Ok(ResumeWorkflowResult::Ready(frontmatter));
    }

    let active_submachine_session_id =
        crate::sessions::frontmatter_string(&frontmatter, "activeSubmachineSessionId");
    let active_submachine_type =
        crate::sessions::frontmatter_string(&frontmatter, "activeSubmachineType");

    if active_submachine_session_id.is_empty() || active_submachine_type.is_empty() {
        return Ok(ResumeWorkflowResult::Ready(frontmatter));
    }

    let submachine_uri =
        session_uri_for_type(&active_submachine_type, &active_submachine_session_id);
    if !file_exists(&submachine_uri, &context.config)? {
        clear_submachine_frontmatter(&mut frontmatter);
        write_markdown(
            workflow_uri,
            Some(&frontmatter),
            existing_content,
            &context.config,
        )?;
        append_h2_section(
            workflow_uri,
            "Submachine Cleared",
            &format!(
                "Registered submachine '{}' session not found (deleted or expired). Clearing waiting state and resuming workflow.\n\n*{}*",
                active_submachine_session_id,
                display_utc_now(),
            ),
            &context.config,
        )?;
        let (updated_frontmatter, _, _) = read_markdown(workflow_uri, &context.config)?;
        return Ok(ResumeWorkflowResult::Ready(
            updated_frontmatter.unwrap_or(frontmatter),
        ));
    }

    let (submachine_frontmatter, _, _) = read_markdown(&submachine_uri, &context.config)?;
    let submachine_status = submachine_frontmatter
        .as_ref()
        .map(|values| crate::sessions::frontmatter_string(values, "status"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "active".to_string());

    if matches!(
        submachine_status.as_str(),
        "completed" | "cancelled" | "abandoned"
    ) {
        clear_submachine_frontmatter(&mut frontmatter);
        let (_, content_after_clear, _) = read_markdown(workflow_uri, &context.config)?;
        write_markdown(
            workflow_uri,
            Some(&frontmatter),
            &content_after_clear,
            &context.config,
        )?;
        append_h2_section(
            workflow_uri,
            "Submachine Resumed",
            &format!(
                "Submachine {} {}. Resuming workflow.\n\n*{}*",
                active_submachine_session_id,
                submachine_status,
                display_utc_now(),
            ),
            &context.config,
        )?;
        let (updated_frontmatter, _, _) = read_markdown(workflow_uri, &context.config)?;
        return Ok(ResumeWorkflowResult::Ready(
            updated_frontmatter.unwrap_or(frontmatter),
        ));
    }

    Ok(ResumeWorkflowResult::Waiting(format!(
        "Workflow is waiting on submachine {active_submachine_type} {active_submachine_session_id}. Complete it first, then call Workflow again."
    )))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn register_workflow_submachine(
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
            display_utc_now(),
        ),
        &context.config,
    )?;

    Ok(format!(
        "Registered submachine {submachine_session_id}. Workflow is waiting. Complete the submachine, then call Workflow with sessionId and response to continue."
    ))
}

pub(crate) fn clear_submachine_frontmatter(frontmatter: &mut Frontmatter) {
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

pub(crate) fn session_uri_for_type(session_type: &str, session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/{session_type}/{year:04}/{month:02}/{day:02}/{session_id}")
}
