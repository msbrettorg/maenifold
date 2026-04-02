use std::sync::Arc;

use crate::acp_native::{AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};
use crate::thinking::sequential_thinking;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use agent_client_protocol as acp;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

use super::agent_session::{AgentSession, open_agent_session};
use super::output_parser::parse_output;
use super::output_types::ThinkingOutput;
use super::runner_helpers::{Envelope, slug};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkResult {
    pub session_id: String,
    pub conclusion: Option<String>,
    pub thoughts_completed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkOptions {
    pub total_thoughts: u32,
    pub parent_workflow_id: Option<String>,
    pub model: Option<String>,
}

impl Default for ThinkOptions {
    fn default() -> Self {
        Self {
            total_thoughts: 5,
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
    #[serde(skip_serializing_if = "is_false")]
    needs_more_thoughts: bool,
    #[serde(skip_serializing_if = "is_false")]
    is_revision: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    revises_thought: Option<u64>,
}

pub(crate) async fn seed_think_session(
    context: &ToolContext,
    topic: &str,
    opts: &ThinkOptions,
) -> Result<String> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let ThinkSessionData { session_id } =
        think_call(&json_context, &think_seed_payload(topic, opts))?;
    Ok(session_id)
}

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
) -> Result<ThinkResult> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let acpx_session = format!("think-{session_id}");
    let driver = open_agent_session(pool, agent, &acpx_session, session_opts).await?;

    let mut thought_number = 1u32;
    let mut first_context = content_blocks;
    let result: Result<ThinkResult> = async {
        loop {
            let prompt = think_prompt(thought_number, opts.total_thoughts, topic, prompt_context);
            let response = if let Some(blocks) = first_context.take() {
                driver.prompt_with_context(&prompt, blocks).await?
            } else {
                driver.prompt(&prompt).await?
            };
            let thinking_output = parse_with_retry(
                &driver,
                &response.text,
                0,
                thought_number,
                opts.total_thoughts,
            )
            .await?;
            let capped_output =
                if !thinking_output.done && thought_number >= opts.total_thoughts + 3 {
                    force_completion_output(&thinking_output)
                } else {
                    thinking_output
                };
            let capped_output = ensure_conclusion_for_completed_output(capped_output);
            let payload = think_progress_payload_from_output(
                session_id,
                &capped_output,
                thought_number,
                opts,
            );
            let _: Value = think_call(&json_context, &payload)?;
            if capped_output.done {
                break Ok(ThinkResult {
                    session_id: session_id.to_string(),
                    conclusion: capped_output.conclusion,
                    thoughts_completed: thought_number,
                });
            }
            thought_number += 1;
        }
    }
    .await;
    match (result, driver.close().await) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) | (Err(_), Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
    }
}

pub async fn run_think(
    context: &ToolContext,
    agent: &str,
    topic: &str,
    opts: ThinkOptions,
) -> Result<ThinkResult> {
    let model = opts.model.clone();
    run_think_with_pool(
        context,
        agent,
        topic,
        WIKILINK_CONTEXT,
        opts,
        SessionOpts {
            cwd: context.config.maenifold_root.clone(),
            model,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds: 14400,
            mcp_servers: vec![],
        },
        Arc::clone(&crate::acp_native::GLOBAL_AGENT_POOL),
        None,
    )
    .await
}

pub(crate) async fn run_think_with_pool(
    context: &ToolContext,
    agent: &str,
    topic: &str,
    prompt_context: &str,
    opts: ThinkOptions,
    session_opts: SessionOpts,
    pool: Arc<AgentPool>,
    content_blocks: Option<Vec<acp::ContentBlock>>,
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

pub(crate) fn think_progress_payload_from_output(
    session_id: &str,
    output: &ThinkingOutput,
    thought_number: u32,
    opts: &ThinkOptions,
) -> Value {
    serialize_payload(ThinkProgressPayload {
        session_id,
        response: &output.response,
        thought_number,
        total_thoughts: opts.total_thoughts,
        next_thought_needed: !output.done,
        conclusion: output.conclusion.as_deref().filter(|_| output.done),
        needs_more_thoughts: output.needs_more,
        is_revision: output.revision_of.is_some(),
        revises_thought: output.revision_of,
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

fn think_prompt(
    thought_number: u32,
    total_thoughts: u32,
    topic: &str,
    prompt_context: &str,
) -> String {
    format!(
        "Context:\n{prompt_context}\n\nTask: {topic}\n\nThought {thought_number}/~{total_thoughts}. Include [[WikiLinks]] for key concepts.\n\n{shape}\n\nSet done=true with a conclusion on your final thought.",
        shape = ThinkingOutput::expected_shape()
    )
}

async fn parse_with_retry(
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

fn force_completion_output(output: &ThinkingOutput) -> ThinkingOutput {
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

fn ensure_conclusion_for_completed_output(mut output: ThinkingOutput) -> ThinkingOutput {
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
    let truncated = if prefix.len() == trimmed.len() {
        prefix.to_string()
    } else {
        let boundary = prefix.rfind(char::is_whitespace).unwrap_or(prefix.len());
        format!("{}…", &prefix[..boundary])
    };
    truncated
}

fn serialize_payload<T: Serialize>(payload: T) -> Value {
    serde_json::to_value(payload).expect("serializing think payload should be infallible")
}

fn is_false(value: &bool) -> bool {
    !value
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
