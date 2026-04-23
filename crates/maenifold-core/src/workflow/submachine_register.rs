use serde_yaml::Value as YamlValue;

use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, append_h2_section, read_markdown, write_markdown};
use crate::sessions::display_utc_now;
use crate::tools::ToolContext;
use crate::workflow::{build_workflow_response_content, read_workflow_definition, workflow_steps};

use super::frontmatter_usize;
use super::read_workflow_queue;
use super::submachine::{HostSessionType, SubmachineType, clear_submachine_frontmatter};
use super::submachine_link::{
    link_child_to_parent, push_related_link, resolved_terminal_state,
    validate_submachine_registration,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn register_workflow_submachine(
    context: &ToolContext,
    workflow_session_id: &str,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    response: &str,
    thoughts: Option<&str>,
    submachine_session_id: &str,
    submachine_type: SubmachineType,
) -> Result<String> {
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

    register_host_submachine(
        context,
        HostSessionType::Workflow,
        workflow_session_id,
        workflow_uri,
        &mut frontmatter,
        submachine_session_id,
        submachine_type,
    )
}

pub(crate) fn register_host_submachine(
    context: &ToolContext,
    host_type: HostSessionType,
    host_session_id: &str,
    host_uri: &str,
    frontmatter: &mut Frontmatter,
    submachine_session_id: &str,
    submachine_type: SubmachineType,
) -> Result<String> {
    let Some(resolved_state) =
        resolved_terminal_state(context, submachine_type, submachine_session_id, None)?
    else {
        return Ok(format!(
            "ERROR: {} '{}' not found under type '{}'. Ensure the child exists before registering.",
            submachine_type.display_name(),
            submachine_session_id,
            submachine_type.as_str()
        ));
    };

    validate_submachine_registration(
        context,
        host_type,
        host_session_id,
        submachine_type,
        submachine_session_id,
    )?;

    if let Some(child_kind) = submachine_type.as_session_kind() {
        link_child_to_parent(
            context,
            child_kind,
            submachine_session_id,
            host_type,
            host_session_id,
        )?;
        push_related_link(frontmatter, &child_kind.link(submachine_session_id));
    }

    // T-HSM-001.1b: RTM FR-10.1 - register submachine lifecycle state in workflow frontmatter.
    // T-HSM-001.1c: RTM FR-10.4 - do not advance currentStep while waiting on the submachine.
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("waiting".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(submachine_type.as_str().to_string()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(submachine_session_id.to_string()),
    );
    frontmatter.insert(
        "activeSubmachineTarget".to_string(),
        YamlValue::String(resolved_state.target_reference.unwrap_or_default()),
    );
    let (_, content_before_register, _) = read_markdown(host_uri, &context.config)?;
    write_markdown(
        host_uri,
        Some(frontmatter),
        &content_before_register,
        &context.config,
    )?;
    append_h2_section(
        host_uri,
        "Submachine Registered",
        &format!(
            "Submachine registered: {} {submachine_session_id}\n\n*{}*",
            submachine_type.as_str(),
            display_utc_now(),
        ),
        &context.config,
    )?;

    Ok(format!(
        "Registered submachine {} {submachine_session_id}. {} is waiting. Complete the submachine, then call {} again to continue.",
        submachine_type.as_str(),
        host_type.display_name(),
        host_type.display_name(),
    ))
}

#[allow(clippy::too_many_lines)]
pub(crate) fn maybe_resume_submachine(
    context: &ToolContext,
    host_type: HostSessionType,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
) -> Result<super::submachine::ResumeSubmachineResult> {
    use super::submachine::{ResumeSubmachineResult, parse_submachine_type};
    use crate::sessions::frontmatter_string;

    // T-12.0: RTM FR-10 - resume only sessions that are marked waiting and still reference an active submachine.
    if frontmatter_string(&frontmatter, "phase") != "waiting" {
        return Ok(ResumeSubmachineResult::Ready(frontmatter));
    }

    let active_submachine_session_id =
        frontmatter_string(&frontmatter, "activeSubmachineSessionId");
    let active_submachine_type = frontmatter_string(&frontmatter, "activeSubmachineType");
    let active_submachine_target = frontmatter_string(&frontmatter, "activeSubmachineTarget");

    if active_submachine_session_id.is_empty() || active_submachine_type.is_empty() {
        return Ok(ResumeSubmachineResult::Ready(frontmatter));
    }

    let submachine_type = parse_submachine_type(Some(&active_submachine_type))?;
    let Some(resolved_state) = resolved_terminal_state(
        context,
        submachine_type,
        &active_submachine_session_id,
        (!active_submachine_target.is_empty()).then_some(active_submachine_target.as_str()),
    )?
    else {
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
                "Registered {} '{}' not found. Clearing waiting state and resuming {}.\n\n*{}*",
                submachine_type.display_name(),
                active_submachine_session_id,
                host_type.display_name(),
                display_utc_now(),
            ),
            &context.config,
        )?;
        let (updated_frontmatter, _, _) = read_markdown(workflow_uri, &context.config)?;
        return Ok(ResumeSubmachineResult::Ready(
            updated_frontmatter.unwrap_or(frontmatter),
        ));
    };

    if resolved_state.terminal {
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
                "{} '{}' reached terminal state '{}'. Resuming {}.\n\n*{}*",
                submachine_type.display_name(),
                active_submachine_session_id,
                resolved_state.target_description,
                host_type.display_name(),
                display_utc_now(),
            ),
            &context.config,
        )?;
        let (updated_frontmatter, _, _) = read_markdown(workflow_uri, &context.config)?;
        return Ok(ResumeSubmachineResult::Ready(
            updated_frontmatter.unwrap_or(frontmatter),
        ));
    }

    Ok(ResumeSubmachineResult::Waiting(format!(
        "{} is waiting on submachine {active_submachine_type} {active_submachine_session_id}. Complete it first, then call {} again.",
        host_type.display_name(),
        host_type.display_name(),
    )))
}
