use super::agent_session::AgentSession;
use super::output_parser::parse_output;
use super::output_types::WorkflowOutput;
use super::runner::WorkflowRunStatus;
use crate::acpx_driver::AcpxResponse;
use crate::error::{MaenifoldError, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
static WORKFLOW_SESSION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"workflow-\d+").unwrap());
static STEP_PROGRESS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"Step (\d+)/(\d+)").unwrap());
#[derive(Debug, Deserialize)]
pub(crate) struct Envelope<T> {
    pub(crate) success: bool,
    pub(crate) data: T,
    pub(crate) error: Option<EnvelopeError>,
}
#[derive(Debug, Deserialize)]
pub(crate) struct EnvelopeError {
    pub(crate) message: String,
}
pub(crate) fn workflow_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Ok(envelope) = serde_json::from_str::<Envelope<String>>(trimmed) {
        return if envelope.success {
            envelope.data.trim().to_string()
        } else {
            envelope
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| trimmed.to_string())
        };
    }
    if trimmed.starts_with('{')
        && let Some(end) = json_prefix_end(trimmed)
    {
        let rest = trimmed[end..].trim();
        if !rest.is_empty() {
            return rest.to_string();
        }
    }
    trimmed.to_string()
}
pub(crate) fn extract_workflow_session_id(text: &str) -> Option<String> {
    WORKFLOW_SESSION_RE
        .find(text)
        .map(|capture| capture.as_str().to_string())
}
pub(crate) fn parse_step_progress(text: &str) -> Option<(u32, u32)> {
    let captures = STEP_PROGRESS_RE.captures(text)?;
    let current = captures.get(1)?.as_str().parse().ok()?;
    let total = captures.get(2)?.as_str().parse().ok()?;
    Some((current, total))
}
pub(crate) fn step_think_config(step: &str) -> Option<(String, u32)> {
    if !step.contains(r#""requiresSequentialThinking":true"#)
        && !step.contains(r#""requiresSequentialThinking": true"#)
    {
        return None;
    }

    let parsed = step
        .find('{')
        .and_then(|index| serde_json::from_str::<Value>(&step[index..]).ok());
    let total_thoughts = parsed
        .as_ref()
        .and_then(|value| {
            value
                .pointer("/metadata/toolHints/SequentialThinking/estimated_thoughts")
                .or_else(|| value.pointer("/metadata/estimated_thoughts"))
                .and_then(Value::as_u64)
        })
        .unwrap_or(5) as u32;
    let topic = parsed
        .as_ref()
        .and_then(|value| value.get("description").and_then(Value::as_str))
        .unwrap_or(step)
        .to_string();
    Some((topic, total_thoughts))
}
pub(crate) fn terminal_workflow_status(step: &str) -> Option<WorkflowRunStatus> {
    if step.starts_with('✅') {
        Some(WorkflowRunStatus::Completed)
    } else if step.starts_with('❌') {
        Some(if step.to_ascii_lowercase().contains("cancelled") {
            WorkflowRunStatus::Cancelled
        } else {
            WorkflowRunStatus::Failed(step.to_string())
        })
    } else {
        None
    }
}
pub(crate) fn workflow_prompt(prompt: &str, step: &str) -> String {
    if prompt.trim().is_empty() {
        step.to_string()
    } else {
        format!("{prompt}\n{step}")
    }
}
pub(crate) async fn prompt_workflow_agent(
    driver: &AgentSession,
    prompt: &str,
    step: &str,
    final_step: bool,
) -> Result<WorkflowAgentResponse> {
    let actual_prompt = workflow_prompt(prompt, step);
    let prompt_with_shape = format!(
        "{actual_prompt}\n\nWrap your response in <output> tags:\n{}\n\nYour thinking can appear outside the tags. Only the <output> block is parsed.",
        WorkflowOutput::expected_shape()
    );
    let response = driver.prompt(&prompt_with_shape).await?;
    let text = workflow_response_text(response)?;
    parse_workflow_with_retry(driver, &text, 3, final_step).await
}
pub(crate) fn workflow_step_payload(
    session_id: &str,
    response: &WorkflowAgentResponse,
    final_step: bool,
) -> serde_json::Value {
    let response_text = workflow_response_text_for_step(response, final_step);
    if response.output.done || final_step {
        json!({
            "sessionId": session_id,
            "response": response_text,
            "status": "completed",
            "conclusion": workflow_conclusion_text(response, final_step)
        })
    } else {
        json!({ "sessionId": session_id, "response": response.output.response })
    }
}
pub(crate) fn workflow_completion_text(
    response: &WorkflowAgentResponse,
    final_step: bool,
) -> Option<String> {
    if response.output.done || final_step {
        Some(workflow_conclusion_text(response, final_step))
    } else {
        None
    }
}
pub(crate) struct WorkflowAgentResponse {
    output: WorkflowOutput,
    used_fallback: bool,
}

async fn parse_workflow_with_retry(
    driver: &AgentSession,
    initial_text: &str,
    max_retries: usize,
    final_step: bool,
) -> Result<WorkflowAgentResponse> {
    let mut text = initial_text.trim().to_string();
    for attempt in 0..=max_retries {
        match parse_workflow_output(&text) {
            Ok(output) => {
                return Ok(WorkflowAgentResponse {
                    output,
                    used_fallback: false,
                });
            }
            Err(error) if attempt < max_retries && !final_step => {
                let retry_prompt = workflow_retry_prompt(&error);
                let response = driver.prompt(&retry_prompt).await?;
                text = workflow_response_text(response)?.trim().to_string();
            }
            Err(error) => {
                eprintln!("workflow output parse failed after retries: {error}");
                return Ok(WorkflowAgentResponse {
                    output: WorkflowOutput {
                        response: text,
                        done: false,
                        conclusion: None,
                    },
                    used_fallback: true,
                });
            }
        }
    }
    Err(MaenifoldError::Message(
        "workflow parsing retry loop exhausted unexpectedly".to_string(),
    ))
}
fn parse_workflow_output(text: &str) -> std::result::Result<WorkflowOutput, String> {
    let parsed = parse_output(text).map_err(|error| error.to_string())?;
    let output = WorkflowOutput::from_parsed(&parsed).map_err(|error| error.to_string())?;
    let conclusion = output
        .conclusion
        .clone()
        .filter(|value| !value.trim().is_empty());
    if output.done && conclusion.is_none() {
        return Err("missing required <conclusion> when <done>true</done>".to_string());
    }
    Ok(WorkflowOutput {
        response: output.response,
        done: output.done,
        conclusion,
    })
}
fn workflow_response_text(response: AcpxResponse) -> Result<String> {
    if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
        return Err(MaenifoldError::Message(error));
    }
    Ok(response.text)
}
fn workflow_conclusion_text(response: &WorkflowAgentResponse, final_step: bool) -> String {
    response
        .output
        .conclusion
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| workflow_response_text_for_step(response, final_step))
}
fn workflow_response_text_for_step(response: &WorkflowAgentResponse, final_step: bool) -> String {
    if response.used_fallback && final_step {
        fallback_step_tag(&response.output.response)
    } else {
        response.output.response.clone()
    }
}
fn workflow_retry_prompt(error: &str) -> String {
    format!(
        "Your response could not be parsed. Error: {error}\nExpected format:\n{}\n\nPlease rewrite your response with the correct <output> tags.",
        WorkflowOutput::expected_shape()
    )
}
fn fallback_step_tag(response: &str) -> String {
    if response.contains("[[") && response.contains("]]") {
        response.to_string()
    } else {
        format!("{response} [[step]]")
    }
}
pub(crate) fn slug(topic: &str) -> String {
    let mut slug = topic
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    if slug.is_empty() {
        "topic".to_string()
    } else {
        slug.trim_matches('-').to_string()
    }
}
fn json_prefix_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index + ch.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}
