use super::runner::WorkflowRunStatus;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

static WIKILINKS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\[.*?\]\]").unwrap());
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

pub(crate) fn tag_response(response: &str) -> String {
    if WIKILINKS_RE.is_match(response) {
        response.to_string()
    } else {
        format!("{response} [[step]]")
    }
}

pub(crate) fn workflow_prompt(prompt: &str, step: &str) -> String {
    if prompt.trim().is_empty() {
        step.to_string()
    } else {
        format!("{prompt}\n{step}")
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
