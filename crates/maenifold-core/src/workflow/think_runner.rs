use crate::acpx_driver::AcpxDriver;
use crate::error::{MaenifoldError, Result};
use crate::thinking::sequential_thinking;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::runner_helpers::{Envelope, slug, tag_response};

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
        let prompt = format!(
            "Thought {thought_number}/~{} on: {topic}. Include [[WikiLinks]]. Say DONE when complete.",
            opts.total_thoughts
        );
        let response = driver
            .prompt(&acpx_session, &prompt)
            .await
            .map_err(|error| MaenifoldError::Message(error.to_string()))?;
        let tagged_response = tag_response(&response.text);
        let done = tagged_response
            .trim()
            .to_ascii_lowercase()
            .starts_with("done")
            || thought_number >= opts.total_thoughts + 3;
        let payload =
            think_progress_payload(session_id, &tagged_response, thought_number, opts, done);
        let _: Value = think_call(&json_context, &payload)?;
        if done {
            return Ok(ThinkResult {
                session_id: session_id.to_string(),
                conclusion: Some(tagged_response),
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

fn think_progress_payload(
    session_id: &str,
    response: &str,
    thought_number: u32,
    opts: &ThinkOptions,
    done: bool,
) -> Value {
    if done {
        json!({
            "sessionId": session_id,
            "response": response,
            "thoughtNumber": thought_number,
            "totalThoughts": opts.total_thoughts,
            "nextThoughtNeeded": false,
            "conclusion": response
        })
    } else {
        json!({
            "sessionId": session_id,
            "response": response,
            "thoughtNumber": thought_number,
            "totalThoughts": opts.total_thoughts,
            "nextThoughtNeeded": true
        })
    }
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
