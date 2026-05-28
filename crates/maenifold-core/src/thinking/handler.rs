use crate::error::Result;
use crate::markdown::{append_h2_section, read_markdown};
use crate::memory::learn_requested;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use crate::workflow::{
    HostSessionType, ResumeSubmachineResult, maybe_resume_submachine, parse_submachine_type,
    register_host_submachine,
};
use serde_json::{Value, json};

use super::formatting::{build_completion_message, build_thought_section};
use super::render_text_success;
use super::session::{
    finalize_session, load_help_doc, sequential_session_uri, validate_session_state,
};
use super::session_init::initialize_session;
use super::validation::{SequentialThinkingInput, validate_completion, validate_payload};

// T-12.0: RTM NFR-1 - preserve SequentialThinking behavior while delegating cohesive helper groups.
#[allow(clippy::too_many_lines)]
pub fn sequential_thinking(context: &ToolContext, payload: &Value) -> Result<String> {
    let input = SequentialThinkingInput::from_payload(payload);
    if let Some(output) = validate_payload(context, payload, &input)? {
        return Ok(output);
    }
    if learn_requested(payload) {
        let help = load_help_doc(&context.config.assets_path, "SequentialThinking")
            .unwrap_or_else(|| "ERROR: Help file not found for SequentialThinking".to_string());
        return Ok(render_text_success(context, help));
    }

    let uri = sequential_session_uri(&input.session_id);
    let (session_exists, early_output) = validate_session_state(context, &input, &uri)?;
    if let Some(output) = early_output {
        return Ok(output);
    }

    if !session_exists {
        initialize_session(context, &input, &uri)?;
    }

    let (frontmatter, existing_content, _) = read_markdown(&uri, &context.config)?;
    let mut frontmatter = frontmatter.unwrap_or_default();
    match maybe_resume_submachine(
        context,
        HostSessionType::Sequential,
        &uri,
        frontmatter,
        &existing_content,
    )? {
        ResumeSubmachineResult::Ready(updated_frontmatter) => frontmatter = updated_frontmatter,
        ResumeSubmachineResult::Waiting(message) => return Ok(message),
    }

    if let Some(submachine_session_id) = input
        .submachine_session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        let response = input.response.as_deref().unwrap_or("");
        let (heading, section) = build_thought_section(
            input.thought_number,
            input.total_thoughts,
            input.needs_more_thoughts,
            input.branch_id.as_deref(),
            input.is_revision,
            input.revises_thought,
            response,
            input.thoughts.as_deref(),
        );
        append_h2_section(&uri, &heading, &section, &context.config)?;
        let submachine_type = parse_submachine_type(input.submachine_type.as_deref())?;
        return register_host_submachine(
            context,
            HostSessionType::Sequential,
            &input.session_id,
            &uri,
            &mut frontmatter,
            submachine_session_id,
            submachine_type,
        );
    }

    if !input.cancel {
        // PARITY-T07: RTM NFR-1 — response may be None when thoughts carried the WikiLinks.
        let response = input.response.as_deref().unwrap_or("");
        let (heading, section) = build_thought_section(
            input.thought_number,
            input.total_thoughts,
            input.needs_more_thoughts,
            input.branch_id.as_deref(),
            input.is_revision,
            input.revises_thought,
            response,
            input.thoughts.as_deref(),
        );
        append_h2_section(&uri, &heading, &section, &context.config)?;
    }

    let status = if input.cancel {
        "cancelled"
    } else if input.next_thought_needed {
        "in_progress"
    } else {
        if let Some(output) = validate_completion(context, &input)? {
            return Ok(output);
        }
        append_h2_section(
            &uri,
            "Conclusion",
            input.conclusion.as_deref().expect("validated conclusion"),
            &context.config,
        )?;
        "completed"
    };

    if status != "in_progress" {
        finalize_session(context, &uri, status, input.thought_number)?;
    }

    let message = build_completion_message(
        input.thought_number,
        &input.session_id,
        input.cancel,
        input.next_thought_needed,
        input.needs_more_thoughts,
        input.total_thoughts,
    );

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({
            "sessionId": input.session_id,
            "thoughtNumber": input.thought_number,
            "totalThoughts": input.total_thoughts,
            "status": status,
            "nextThoughtNeeded": input.next_thought_needed,
            "message": message,
        })),
        OutputFormat::Markdown => message,
    })
}

#[cfg(test)]
#[path = "handler_tests.rs"]
mod tests;
