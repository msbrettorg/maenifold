use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{MaenifoldError, Result};
use crate::thinking::sequential_thinking;
use crate::tools::ToolContext;

use super::agent_session::AgentResponse;
use super::runner_helpers::{Envelope, slug, tag_fallback};
use super::think_runner::ThinkOptions;

pub(super) const THINK_FAILURE_PREFIX: &str = "\u{0}pipeline_think_failure:";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThinkSessionData {
    pub(super) session_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ThinkSeedPayload<'a> {
    thought_number: u32,
    total_thoughts: u32,
    next_thought_needed: bool,
    response: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_workflow_id: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ThinkProgressPayload<'a> {
    session_id: &'a str,
    response: &'a str,
    thought_number: u32,
    total_thoughts: u32,
    next_thought_needed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    conclusion: Option<&'a str>,
}

pub(super) fn think_seed_payload(topic: &str, opts: &ThinkOptions) -> Value {
    serialize_payload(ThinkSeedPayload {
        thought_number: 0,
        total_thoughts: opts.total_thoughts,
        next_thought_needed: true,
        response: format!("[[{}]]", slug(&topic.chars().take(80).collect::<String>())),
        parent_workflow_id: opts.parent_workflow_id.as_deref(),
    })
}

pub(super) fn think_call<T: DeserializeOwned>(context: &ToolContext, payload: &Value) -> Result<T> {
    let envelope: Envelope<T> = serde_json::from_str(&sequential_thinking(context, payload)?)?;
    if envelope.success {
        Ok(envelope.data)
    } else {
        Err(MaenifoldError::Message(
            envelope
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| "SequentialThinking failed".to_string()),
        ))
    }
}

pub(super) fn think_progress_payload(
    session_id: &str,
    response: &str,
    thought_number: u32,
    total_thoughts: u32,
    done: bool,
) -> Value {
    serialize_payload(ThinkProgressPayload {
        session_id,
        response,
        thought_number,
        total_thoughts,
        next_thought_needed: !done,
        conclusion: done.then_some(response),
    })
}

fn think_response_text(response: AgentResponse) -> Result<String> {
    if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
        return Err(MaenifoldError::Message(error));
    }
    Ok(response.text)
}

pub(crate) fn decode_think_failure(response: &str) -> Option<String> {
    response
        .strip_prefix(THINK_FAILURE_PREFIX)
        .map(str::to_string)
}

pub(super) fn encode_think_failure(error: &MaenifoldError) -> String {
    format!("{THINK_FAILURE_PREFIX}{error}")
}

pub(super) fn normalize_think_prompt_response(response: Result<AgentResponse>) -> Result<String> {
    response
        .and_then(think_response_text)
        .map(|text| tag_fallback(&text))
}

pub(super) fn is_completion_response(response: &str) -> bool {
    response.lines().any(|line| {
        line.as_bytes()
            .get(..4)
            .map(|prefix| prefix.eq_ignore_ascii_case(b"DONE"))
            .unwrap_or(false)
    })
}

pub(super) fn think_prompt(
    thought_number: u32,
    total_thoughts: u32,
    topic: &str,
    prompt_context: &str,
) -> String {
    let prompt = format!(
        "Thought {thought_number}/~{total_thoughts} on: {topic}. Include [[WikiLinks]]. Say DONE when complete."
    );
    let context = prompt_context.trim();
    if context.is_empty() {
        prompt
    } else {
        format!("{context}\n\n{prompt}")
    }
}

fn serialize_payload<T: Serialize>(payload: T) -> Value {
    serde_json::to_value(payload).expect("serializing think payload should be infallible")
}

#[cfg(test)]
mod tests {
    use super::super::agent_session::AgentResponse;
    use super::super::think_runner::think_failure_result;
    use super::{
        decode_think_failure, is_completion_response, normalize_think_prompt_response, think_prompt,
    };
    use crate::error::MaenifoldError;

    #[test]
    fn completion_response_matches_done_at_start_of_any_line() {
        assert!(is_completion_response(
            "preamble line\nDONE final synthesis [[step]]"
        ));
    }

    #[test]
    fn completion_response_does_not_match_indented_done() {
        assert!(!is_completion_response("preamble\n DONE later"));
    }

    #[test]
    fn think_prompt_preserves_context_as_plain_text_prefix() {
        assert_eq!(
            think_prompt(2, 5, "topic", "upstream context"),
            "upstream context\n\nThought 2/~5 on: topic. Include [[WikiLinks]]. Say DONE when complete."
        );
    }

    #[test]
    fn normalize_think_prompt_response_keeps_success_text() {
        let response = normalize_think_prompt_response(Ok(AgentResponse {
            text: "DONE final synthesis [[step]]".to_string(),
            error: None,
        }))
        .expect("success response should remain ok");

        assert_eq!(response, "DONE final synthesis [[step]]");
    }

    #[test]
    fn normalize_think_prompt_response_propagates_prompt_failure() {
        let response = normalize_think_prompt_response(Ok(AgentResponse {
            text: String::new(),
            error: Some("max_turn_requests".to_string()),
        }));

        assert!(
            response.is_err(),
            "think prompt failures should propagate instead of falling back to agent error text"
        );
    }

    #[test]
    fn normalize_think_prompt_response_propagates_transport_failure() {
        let response = normalize_think_prompt_response(Err(MaenifoldError::Message(
            "transport failed".to_string(),
        )));

        assert!(
            response.is_err(),
            "think transport failures should propagate instead of falling back to agent error text"
        );
    }

    #[test]
    fn think_failure_result_preserves_session_provenance() {
        let result = think_failure_result(
            "thinking-123",
            "acp-456",
            &MaenifoldError::Message("transport failed".to_string()),
        );

        assert_eq!(result.session_id, "thinking-123");
        assert_eq!(result.acp_session_id.as_deref(), Some("acp-456"));
        assert_eq!(
            result.conclusion.as_deref().and_then(decode_think_failure),
            Some("transport failed".to_string())
        );
        assert_eq!(result.thoughts_completed, 0);
    }
}
