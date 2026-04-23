use super::agent_session::{AgentResponse, AgentSession};
use super::runner::WorkflowRunStatus;
use crate::error::{MaenifoldError, Result};
use agent_client_protocol as acp;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::LazyLock;
static STEP_PROGRESS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Step (\d+)/(\d+)").expect("static step-progress regex is valid"));

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
pub(crate) fn parse_step_progress(text: &str) -> Option<(u32, u32)> {
    let captures = STEP_PROGRESS_RE.captures(text)?;
    Some((
        captures.get(1)?.as_str().parse().ok()?,
        captures.get(2)?.as_str().parse().ok()?,
    ))
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
        .and_then(|v| {
            v.get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty() && s.chars().count() <= 300)
                .or_else(|| {
                    v.get("name")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                })
        })
        .unwrap_or("step analysis")
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
    content_blocks: Option<Vec<acp::ContentBlock>>,
) -> Result<String> {
    let full_prompt = workflow_prompt(prompt, step);
    let full_prompt = content_blocks
        .map(|blocks| prompt_with_embedded_context(&full_prompt, blocks))
        .unwrap_or(full_prompt);
    let response = driver.prompt(&full_prompt).await;
    normalize_workflow_prompt_response(response)
}

pub(crate) fn prompt_with_embedded_context(
    prompt: &str,
    content_blocks: Vec<acp::ContentBlock>,
) -> String {
    let serialized = serde_json::to_string_pretty(&content_blocks)
        .unwrap_or_else(|_| "[context serialization failed]".to_string());
    format!("{prompt}\n\n[context]\n{serialized}")
}
pub(crate) fn workflow_step_payload(
    session_id: &str,
    response: &str,
    final_step: bool,
) -> serde_json::Value {
    if final_step {
        json!({
            "sessionId": session_id,
            "response": response,
            "status": "completed",
            "conclusion": response
        })
    } else {
        json!({ "sessionId": session_id, "response": response })
    }
}
pub(crate) fn workflow_completion_text(response: &str, final_step: bool) -> Option<String> {
    final_step.then(|| response.to_string())
}
fn workflow_response_text(response: AgentResponse) -> Result<String> {
    if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
        return Err(MaenifoldError::Message(error));
    }
    Ok(response.text)
}

fn normalize_workflow_prompt_response(response: Result<AgentResponse>) -> Result<String> {
    response
        .and_then(workflow_response_text)
        .map(|text| tag_fallback(&text))
}

pub(crate) fn tag_fallback(response: &str) -> String {
    if has_well_formed_wikilink(response) {
        response.to_string()
    } else {
        format!("{response} [[step]]")
    }
}
pub(crate) fn has_well_formed_wikilink(text: &str) -> bool {
    let bytes = text.as_bytes();
    let (mut i, mut open, mut saw) = (0usize, false, false);
    while i + 1 < bytes.len() {
        match (bytes[i], bytes[i + 1]) {
            (b'[', b'[') if !open => {
                open = true;
                i += 2;
            }
            (b'[', b'[') => return false,
            (b']', b']') if open => {
                open = false;
                saw = true;
                i += 2;
            }
            (b']', b']') => return false,
            _ => i += 1,
        }
    }
    saw && !open
}

pub(crate) fn slug(topic: &str) -> String {
    let result = ::slug::slugify(topic);
    if result.is_empty() {
        "topic".to_string()
    } else {
        result
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

#[cfg(test)]
mod tests {
    use super::super::agent_session::AgentResponse;
    use super::normalize_workflow_prompt_response;
    use crate::error::MaenifoldError;

    #[test]
    fn normalize_workflow_prompt_response_keeps_success_text() {
        let response = normalize_workflow_prompt_response(Ok(AgentResponse {
            text: "Completed [[step]]".to_string(),
            error: None,
        }))
        .expect("success response should remain ok");

        assert_eq!(response, "Completed [[step]]");
    }

    #[test]
    fn normalize_workflow_prompt_response_propagates_prompt_failure() {
        let response = normalize_workflow_prompt_response(Ok(AgentResponse {
            text: String::new(),
            error: Some("refusal".to_string()),
        }));

        assert!(
            response.is_err(),
            "workflow prompt failures should propagate instead of falling back to agent error text"
        );
    }

    #[test]
    fn normalize_workflow_prompt_response_propagates_transport_failure() {
        let response = normalize_workflow_prompt_response(Err(MaenifoldError::Message(
            "transport failed".to_string(),
        )));

        assert!(
            response.is_err(),
            "workflow transport failures should propagate instead of falling back to agent error text"
        );
    }
}
