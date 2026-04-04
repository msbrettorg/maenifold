use crate::error::{MaenifoldError, Result};

use super::agent_session::AgentSession;
use super::output_parser::{ParseError, parse_output};
use super::output_types::ThinkingOutput;

pub(super) async fn parse_with_retry(
    driver: &AgentSession,
    initial_text: &str,
    max_retries: u32,
    thought_number: u32,
    total_thoughts: u32,
) -> Result<ThinkingOutput> {
    let mut text = initial_text.to_string();

    for retries_remaining in (0..=max_retries).rev() {
        match parse_output(&text)
            .and_then(|parsed| ThinkingOutput::from_parsed(&parsed))
            .and_then(validate_completed_thinking_output)
        {
            Ok(output) => return Ok(output),
            Err(parse_error) if retries_remaining > 0 => {
                let retry_prompt = format!(
                    "Your response could not be parsed. Error: {}\n\n{}\n\nPlease rewrite your response in the correct format.",
                    parse_error.message,
                    ThinkingOutput::expected_shape()
                );
                text = driver.prompt(&retry_prompt).await?.text;
            }
            Err(parse_error) => {
                return Err(MaenifoldError::Message(format!(
                    "thinking output parse failed for thought {thought_number}/{total_thoughts} after {} attempt(s): {}. Last response preview: {}",
                    max_retries + 1,
                    parse_error.message,
                    preview_response(&text),
                )));
            }
        }
    }

    Err(MaenifoldError::Message(
        "thinking parsing retry loop exhausted unexpectedly".to_string(),
    ))
}

pub(super) fn force_completion_output(output: &ThinkingOutput) -> ThinkingOutput {
    ThinkingOutput {
        response: output.response.clone(),
        done: true,
        needs_more: false,
        revision_of: output.revision_of,
        conclusion: output.conclusion.clone(),
    }
}

pub(super) fn ensure_conclusion_for_completed_output(
    output: ThinkingOutput,
) -> Result<ThinkingOutput> {
    validate_completed_thinking_output(output)
        .map_err(|error| MaenifoldError::Message(error.message))
}

fn validate_completed_thinking_output(
    output: ThinkingOutput,
) -> std::result::Result<ThinkingOutput, ParseError> {
    let conclusion = output
        .conclusion
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if output.done && conclusion.is_none() {
        return Err(ParseError {
            message: "missing required <conclusion> when <done>true</done>".to_string(),
        });
    }
    Ok(ThinkingOutput {
        conclusion,
        ..output
    })
}

fn preview_response(text: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 500;

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "<empty response>".to_string();
    }

    let preview: String = trimmed.chars().take(MAX_PREVIEW_CHARS).collect();
    if trimmed.chars().count() > MAX_PREVIEW_CHARS {
        format!("{preview}…")
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_conclusion_for_completed_output, force_completion_output,
        validate_completed_thinking_output,
    };
    use crate::workflow::output_types::ThinkingOutput;

    fn thinking_output(done: bool, conclusion: Option<&str>) -> ThinkingOutput {
        ThinkingOutput {
            response: "[[concept]]".to_string(),
            done,
            needs_more: false,
            revision_of: None,
            conclusion: conclusion.map(str::to_string),
        }
    }

    #[test]
    fn validate_completed_thinking_output_rejects_missing_conclusion() {
        let error = validate_completed_thinking_output(thinking_output(true, None))
            .expect_err("completed thinking output without conclusion should fail");

        assert_eq!(
            error.message,
            "missing required <conclusion> when <done>true</done>"
        );
    }

    #[test]
    fn validate_completed_thinking_output_trims_completed_conclusion() {
        let output =
            validate_completed_thinking_output(thinking_output(true, Some("  final summary  ")))
                .expect("completed thinking output with conclusion should succeed");

        assert_eq!(output.conclusion.as_deref(), Some("final summary"));
    }

    #[test]
    fn ensure_conclusion_for_completed_output_rejects_forced_completion_without_conclusion() {
        let forced = force_completion_output(&thinking_output(false, None));
        let error = ensure_conclusion_for_completed_output(forced)
            .expect_err("forced completion without conclusion should fail");

        assert_eq!(
            error.to_string(),
            "missing required <conclusion> when <done>true</done>"
        );
    }
}
