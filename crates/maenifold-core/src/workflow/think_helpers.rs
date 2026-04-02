use crate::error::Result;

use super::agent_session::AgentSession;
use super::output_parser::parse_output;
use super::output_types::ThinkingOutput;

pub(super) async fn parse_with_retry(
    driver: &AgentSession,
    initial_text: &str,
    max_retries: u32,
    thought_number: u32,
    total_thoughts: u32,
) -> Result<ThinkingOutput> {
    let original_text = initial_text.to_string();
    let mut text = original_text.clone();

    for retries_remaining in (0..=max_retries).rev() {
        match parse_output(&text).and_then(|parsed| ThinkingOutput::from_parsed(&parsed)) {
            Ok(output) => return Ok(output),
            Err(parse_error) if retries_remaining > 0 => {
                let retry_prompt = format!(
                    "Your response could not be parsed. Error: {}\n\n{}\n\nPlease rewrite your response in the correct format.",
                    parse_error.message,
                    ThinkingOutput::expected_shape()
                );
                text = driver.prompt(&retry_prompt).await?.text;
            }
            Err(_) => break,
        }
    }

    let done = thought_number >= total_thoughts;
    Ok(ThinkingOutput {
        response: original_text.clone(),
        done,
        needs_more: false,
        revision_of: None,
        conclusion: done.then(|| original_text.clone()),
    })
}

pub(super) fn force_completion_output(output: &ThinkingOutput) -> ThinkingOutput {
    let response = output.response.clone();
    let conclusion = output
        .conclusion
        .clone()
        .unwrap_or_else(|| response.clone());
    ThinkingOutput {
        response,
        done: true,
        needs_more: false,
        revision_of: output.revision_of,
        conclusion: Some(conclusion),
    }
}

pub(super) fn ensure_conclusion_for_completed_output(mut output: ThinkingOutput) -> ThinkingOutput {
    if output.done && output.conclusion.is_none() {
        output.conclusion = Some(synthesize_conclusion(&output.response));
    }
    output
}

fn synthesize_conclusion(response: &str) -> String {
    const MAX_LEN: usize = 500;
    let trimmed = response.trim();
    if trimmed.is_empty() {
        return "Session completed.".to_string();
    }
    let prefix = truncate_to_char_boundary(trimmed, MAX_LEN);
    if prefix.len() == trimmed.len() {
        prefix.to_string()
    } else {
        let boundary = prefix.rfind(char::is_whitespace).unwrap_or(prefix.len());
        format!("{}…", &prefix[..boundary])
    }
}

fn truncate_to_char_boundary(text: &str, max_len: usize) -> &str {
    if text.len() <= max_len {
        return text;
    }
    let mut end = max_len;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
