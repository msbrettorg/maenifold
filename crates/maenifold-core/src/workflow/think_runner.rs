use std::sync::Arc;

use agent_client_protocol as acp;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::acp_native::{AcpxResponse, AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};
use crate::thinking::sequential_thinking;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::agent_session::{AgentSession, open_agent_session};
use super::runner::append_close_error_context;
use super::runner::sanitize_workflow_session_opts;
use super::runner_helpers::{Envelope, slug, tag_fallback};

fn trace_enabled() -> bool {
    std::env::var("MAENIFOLD_TRACE")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        })
        .unwrap_or(true)
}

/// Canonical default for sequential thinking depth.
/// Callers who don't have a strong preference reference this constant explicitly.
pub const DEFAULT_TOTAL_THOUGHTS: u32 = 5;

/// WikiLink tagging instructions injected as context for all ACP think/workflow sessions.
/// Sourced from the concept-as-protocol ontology. Agents see this before their task prompt.
pub(crate) const WIKILINK_CONTEXT: &str = r#"Tag important concepts as [[WikiLinks]] using these rules:
- Double brackets: [[WikiLink]] never [WikiLink]
- Normalized to lowercase-with-hyphens internally
- PRIMARY concept only: [[MCP]] not [[MCP-server]]
- GENERAL terms: [[authentication]] not [[auth-system]]
- NO file paths, code elements, or trivial words
- TAG substance: [[machine-learning]], [[GraphRAG]], [[vector-embeddings]]
- REUSE existing concepts before inventing near-duplicates
- HYPHENATE multiword: [[null-reference-exception]] not [[Null Reference Exception]]
Anti-patterns (avoid):
- Underscores: [[my_concept]] → use [[my-concept]]
- Slashes: [[foo/bar]] → use [[foo-bar]]
- Double hyphens: [[foo--bar]] → use [[foo-bar]]
WikiLinks become knowledge graph nodes. Bad tagging = graph corruption.
Only tag concepts worth a wiki page. Never tag meta-terms like [[code]], [[bug]], [[fix]]."#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkResult {
    pub session_id: String,
    pub acp_session_id: Option<String>,
    pub conclusion: Option<String>,
    pub thoughts_completed: u32,
}

const THINK_FAILURE_PREFIX: &str = "\u{0}pipeline_think_failure:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkOptions {
    pub total_thoughts: u32,
    pub parent_workflow_id: Option<String>,
    pub model: Option<String>,
}

fn think_failure_result(
    session_id: impl Into<String>,
    acp_session_id: impl Into<String>,
    error: &MaenifoldError,
) -> ThinkResult {
    ThinkResult {
        session_id: session_id.into(),
        acp_session_id: Some(acp_session_id.into()),
        conclusion: Some(encode_think_failure(error)),
        thoughts_completed: 0,
    }
}

impl Default for ThinkOptions {
    fn default() -> Self {
        Self {
            total_thoughts: DEFAULT_TOTAL_THOUGHTS,
            parent_workflow_id: None,
            model: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThinkSessionData {
    session_id: String,
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

pub(crate) async fn seed_think_session(
    context: &ToolContext,
    topic: &str,
    opts: &ThinkOptions,
) -> Result<String> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let ThinkSessionData { session_id } =
        think_call(&json_context, &think_seed_payload(topic, opts))?;
    if trace_enabled() {
        eprintln!("[think] seed session");
    }
    Ok(session_id)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn drive_think_session(
    context: &ToolContext,
    agent: &str,
    session_id: &str,
    topic: &str,
    prompt_context: &str,
    opts: &ThinkOptions,
    session_opts: SessionOpts,
    pool: &AgentPool,
    content_blocks: Option<Vec<acp::ContentBlock>>,
    keep_session_alive: bool,
) -> Result<ThinkResult> {
    let driver = open_agent_session(
        pool,
        agent,
        &format!("think-{session_id}"),
        sanitize_workflow_session_opts(session_opts),
    )
    .await?;
    let result = drive_think_session_with_driver(
        context,
        &driver,
        session_id,
        topic,
        prompt_context,
        opts,
        content_blocks,
    )
    .await;
    let acp_session_id = driver.remote_id().to_string();
    let failure_result =
        |error: &MaenifoldError| think_failure_result(session_id, acp_session_id.clone(), error);
    if keep_session_alive {
        driver.keep_alive();
        match result {
            Ok(value) => Ok(value),
            Err(error) => Ok(failure_result(&error)),
        }
    } else {
        match (result, driver.close().await) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(close_error)) => {
                let error = append_close_error_context(error, close_error);
                Ok(failure_result(&error))
            }
            (Err(error), Ok(())) => Ok(failure_result(&error)),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn drive_think_session_with_driver(
    context: &ToolContext,
    driver: &AgentSession,
    session_id: &str,
    topic: &str,
    prompt_context: &str,
    opts: &ThinkOptions,
    content_blocks: Option<Vec<acp::ContentBlock>>,
) -> Result<ThinkResult> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let mut thought_number = 1u32;
    let mut first_context = content_blocks;
    loop {
        if trace_enabled() {
            eprintln!("[think] thought {}/{}", thought_number, opts.total_thoughts);
        }
        let prompt = think_prompt(thought_number, opts.total_thoughts, topic, prompt_context);
        let response = if let Some(blocks) = first_context.take() {
            driver.prompt_with_context(&prompt, blocks).await
        } else {
            driver.prompt(&prompt).await
        };
        let response_text = normalize_think_prompt_response(response)?;
        let done = is_completion_response(&response_text)
            || thought_number >= opts.total_thoughts.saturating_add(3);
        let _: Value = think_call(
            &json_context,
            &think_progress_payload(
                session_id,
                &response_text,
                thought_number,
                opts.total_thoughts,
                done,
            ),
        )?;
        if done {
            if trace_enabled() {
                eprintln!("[think] complete");
            }
            return Ok(ThinkResult {
                session_id: session_id.to_string(),
                acp_session_id: Some(driver.remote_id().to_string()),
                conclusion: Some(response_text),
                thoughts_completed: thought_number,
            });
        }
        thought_number += 1;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_think_with_pool(
    context: &ToolContext,
    agent: &str,
    topic: &str,
    prompt_context: &str,
    opts: ThinkOptions,
    session_opts: SessionOpts,
    pool: Arc<AgentPool>,
    content_blocks: Option<Vec<acp::ContentBlock>>,
    keep_session_alive: bool,
) -> Result<ThinkResult> {
    let session_id = seed_think_session(context, topic, &opts).await?;
    drive_think_session(
        context,
        agent,
        &session_id,
        topic,
        prompt_context,
        &opts,
        session_opts,
        pool.as_ref(),
        content_blocks,
        keep_session_alive,
    )
    .await
}

fn think_seed_payload(topic: &str, opts: &ThinkOptions) -> Value {
    serialize_payload(ThinkSeedPayload {
        thought_number: 0,
        total_thoughts: opts.total_thoughts,
        next_thought_needed: true,
        response: format!("[[{}]]", slug(&topic.chars().take(80).collect::<String>())),
        parent_workflow_id: opts.parent_workflow_id.as_deref(),
    })
}

fn think_call<T: DeserializeOwned>(context: &ToolContext, payload: &Value) -> Result<T> {
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

fn think_progress_payload(
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

fn think_response_text(response: AcpxResponse) -> Result<String> {
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

fn encode_think_failure(error: &MaenifoldError) -> String {
    format!("{THINK_FAILURE_PREFIX}{error}")
}

fn normalize_think_prompt_response(response: Result<AcpxResponse>) -> Result<String> {
    response
        .and_then(think_response_text)
        .map(|text| tag_fallback(&text))
}

fn is_completion_response(response: &str) -> bool {
    response.lines().any(|line| {
        line.as_bytes()
            .get(..4)
            .map(|prefix| prefix.eq_ignore_ascii_case(b"DONE"))
            .unwrap_or(false)
    })
}

fn think_prompt(
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
    use super::{
        decode_think_failure, is_completion_response, normalize_think_prompt_response,
        think_failure_result, think_prompt,
    };
    use crate::acp_native::AcpxResponse;
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
        let response = normalize_think_prompt_response(Ok(AcpxResponse {
            text: "DONE final synthesis [[step]]".to_string(),
            ..AcpxResponse::default()
        }))
        .expect("success response should remain ok");

        assert_eq!(response, "DONE final synthesis [[step]]");
    }

    #[test]
    fn normalize_think_prompt_response_propagates_prompt_failure() {
        let response = normalize_think_prompt_response(Ok(AcpxResponse {
            error: Some("max_turn_requests".to_string()),
            ..AcpxResponse::default()
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
