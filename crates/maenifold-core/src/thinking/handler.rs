use crate::error::Result;
use crate::markdown::append_h2_section;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};

use super::formatting::{build_completion_message, build_thought_section};
use super::render_text_success;
use super::session::{
    finalize_session, initialize_session, load_help_doc, sequential_session_uri,
    validate_session_state,
};
use super::validation::{SequentialThinkingInput, validate_completion, validate_payload};

// T-12.0: RTM NFR-1 - preserve SequentialThinking behavior while delegating cohesive helper groups.
#[allow(clippy::too_many_lines)]
pub fn sequential_thinking(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "SequentialThinking")
            .unwrap_or_else(|| "ERROR: Help file not found for SequentialThinking".to_string());
        return Ok(render_text_success(context, help));
    }

    let input = SequentialThinkingInput::from_payload(payload);
    if let Some(output) = validate_payload(context, &input)? {
        return Ok(output);
    }

    let uri = sequential_session_uri(&input.session_id);
    let (session_exists, early_output) = validate_session_state(context, &input, &uri)?;
    if let Some(output) = early_output {
        return Ok(output);
    }

    if !session_exists {
        initialize_session(context, &input, &uri)?;
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
