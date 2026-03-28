use crate::acpx_driver::AcpxDriver;
use crate::error::{MaenifoldError, Result};
use crate::thinking::sequential_thinking;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

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
}

impl Default for ThinkOptions {
    fn default() -> Self {
        Self {
            total_thoughts: 5,
            parent_workflow_id: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThinkSessionData {
    session_id: String,
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
    opts: &ThinkOptions,
) -> Result<ThinkResult> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let acpx_session = format!("think-{session_id}");
    let driver = AcpxDriver::new(agent.to_string(), 0, context.config.maenifold_root.clone())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    driver
        .create_session(&acpx_session)
        .await
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;

    let mut thought_number = 1u32;
    loop {
        let prompt = think_prompt(thought_number, opts.total_thoughts, topic);
        let response = driver
            .prompt(&acpx_session, &prompt)
            .await
            .map_err(|error| MaenifoldError::Message(error.to_string()))?;
        let thinking_output = parse_with_retry(&driver, &acpx_session, &response.text, 3).await?;
        let capped_output = if !thinking_output.done && thought_number >= opts.total_thoughts + 3 { force_completion_output(&thinking_output) } else { thinking_output };
        let payload = think_progress_payload_from_output(session_id, &capped_output, thought_number, opts);
        let _: Value = think_call(&json_context, &payload)?;
        if capped_output.done {
            return Ok(ThinkResult {
                session_id: session_id.to_string(),
                conclusion: capped_output.conclusion,
                thoughts_completed: thought_number,
            });
        }
        thought_number += 1;
    }
}

pub async fn run_think(
    context: &ToolContext,
    agent: &str,
    topic: &str,
    opts: ThinkOptions,
) -> Result<ThinkResult> {
    let session_id = seed_think_session(context, topic, &opts).await?;
    drive_think_session(context, agent, &session_id, topic, &opts).await
}

fn think_seed_payload(topic: &str, opts: &ThinkOptions) -> Value {
    let mut seed = json!({
        "thoughtNumber": 0,
        "totalThoughts": opts.total_thoughts,
        "nextThoughtNeeded": true,
        "response": format!("[[{}]]", slug(topic))
    });
    if let Some(parent) = &opts.parent_workflow_id {
        seed["parentWorkflowId"] = json!(parent);
    }
    seed
}

pub(crate) fn think_progress_payload_from_output(
    session_id: &str,
    output: &ThinkingOutput,
    thought_number: u32,
    opts: &ThinkOptions,
) -> Value {
    let mut payload = json!({
        "sessionId": session_id,
        "response": output.response,
        "thoughtNumber": thought_number,
        "totalThoughts": opts.total_thoughts,
        "nextThoughtNeeded": !output.done,
    });
    if output.done && let Some(conclusion) = &output.conclusion {
        payload["conclusion"] = json!(conclusion);
    }
    if output.needs_more {
        payload["needsMoreThoughts"] = json!(true);
    }
    if let Some(revision_of) = output.revision_of {
        payload["isRevision"] = json!(true);
        payload["revisesThought"] = json!(revision_of);
    }
    payload
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

fn think_prompt(thought_number: u32, total_thoughts: u32, topic: &str) -> String {
    format!(
        "Thought {thought_number}/~{total_thoughts} on: {topic}. Include [[WikiLinks]].\n\nWrap your response in <output> tags:\n{}\n\nYour thinking can appear outside the tags. Only the <output> block is parsed.",
        ThinkingOutput::expected_shape()
    )
}

async fn parse_with_retry(
    driver: &AcpxDriver,
    session: &str,
    initial_text: &str,
    max_retries: u32,
) -> Result<ThinkingOutput> {
    let mut text = initial_text.to_string();
    for attempt in 0..=max_retries {
        match parse_output(&text).and_then(|parsed| ThinkingOutput::from_parsed(&parsed)) {
            Ok(output) => return Ok(output),
            Err(parse_error) if attempt < max_retries => {
                let retry_prompt = format!(
                    "Your response could not be parsed. Error: {}\n\nExpected format:\n{}\n\nPlease rewrite your response with the correct <output> tags.",
                    parse_error.message,
                    ThinkingOutput::expected_shape()
                );
                let retry_response = driver
                    .prompt(session, &retry_prompt)
                    .await
                    .map_err(|error| MaenifoldError::Message(error.to_string()))?;
                text = retry_response.text;
            }
            Err(_) => {
                let tagged = tag_fallback(&text);
                return Ok(ThinkingOutput {
                    response: tagged.clone(),
                    done: true,
                    needs_more: false,
                    revision_of: None,
                    conclusion: Some(tagged),
                });
            }
        }
    }
    unreachable!()
}

#[rustfmt::skip]
fn force_completion_output(output: &ThinkingOutput) -> ThinkingOutput {
    let response = tag_fallback(&output.response);
    ThinkingOutput { response: response.clone(), done: true, needs_more: false, revision_of: output.revision_of, conclusion: Some(output.conclusion.as_deref().map(tag_fallback).unwrap_or(response)) }
}

fn tag_fallback(response: &str) -> String {
    if response.contains("[[") && response.contains("]]") {
        response.to_string()
    } else {
        format!("{response} [[step]]")
    }
}
