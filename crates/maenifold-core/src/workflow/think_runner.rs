use agent_client_protocol as acp;
use jockey::EventSender;
use serde_json::Value;

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::adoption::require_adopted_role;
use super::agent_session::{AgentSession, open_agent_session};
use super::runner::append_close_error_context;
use super::runner::{AgentSessionConfig, sanitize_workflow_session_config};
use super::runner_helpers::prompt_with_embedded_context;
use super::think_runner_payloads::{
    ThinkSessionData, is_completion_response, normalize_think_prompt_response, think_call,
    think_progress_payload, think_prompt, think_seed_payload,
};

pub(crate) use super::think_runner_payloads::decode_think_failure;
use super::think_runner_payloads::encode_think_failure;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkResult {
    pub session_id: String,
    pub acp_session_id: Option<String>,
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
            total_thoughts: DEFAULT_TOTAL_THOUGHTS,
            parent_workflow_id: None,
            model: None,
        }
    }
}

pub(super) fn think_failure_result(
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
    adopt_instructions: &[String],
    adopt_role: Option<&str>,
    requires_role: Option<&str>,
    session_config: AgentSessionConfig,
    event_tx: &EventSender,
    content_blocks: Option<Vec<acp::ContentBlock>>,
    keep_session_alive: bool,
) -> Result<ThinkResult> {
    require_adopted_role("sequential thinking invocation", requires_role, adopt_role)?;
    let session_config = sanitize_workflow_session_config(session_config);
    let driver = open_agent_session(agent, &session_config, event_tx).await?;
    for instruction in adopt_instructions {
        let response = driver.prompt(instruction).await?;
        if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
            return Err(MaenifoldError::Message(error));
        }
    }
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
    let acp_session_id = driver.remote_id();
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
        let prompt = first_context
            .take()
            .map(|blocks| prompt_with_embedded_context(&prompt, blocks))
            .unwrap_or(prompt);
        let response = driver.prompt(&prompt).await;
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
                acp_session_id: Some(driver.remote_id()),
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
    adopt_instructions: &[String],
    adopt_role: Option<&str>,
    requires_role: Option<&str>,
    session_config: AgentSessionConfig,
    event_tx: EventSender,
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
        adopt_instructions,
        adopt_role,
        requires_role,
        session_config,
        &event_tx,
        content_blocks,
        keep_session_alive,
    )
    .await
}
