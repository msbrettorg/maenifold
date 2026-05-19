use std::error::Error as StdError;

use jockey::EventSender;
use serde_json::Value as JsonValue;
use std::path::Path;
use tracing::{error, info};

use crate::acp_config::AgentsConfig;
use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::super::adoption::require_adopted_role;
use super::super::agent_session::{AgentSession, open_agent_session};
use super::super::runner::{self, AgentSessionConfig, WorkflowRunStatus};
use super::super::think_runner::decode_think_failure;
use super::executor_helpers::{MAX_CONCLUSION_CHARS, truncate_chars};
use super::external_mcp::{invoke_external_tool, split_namespaced_tool_name};
use super::types::ToolProvenance;

#[path = "executor_run_classify.rs"]
mod executor_run_classify;
#[path = "executor_run_exec.rs"]
mod executor_run_exec;
#[path = "executor_run_persist.rs"]
mod executor_run_persist;
#[path = "executor_run_structured.rs"]
mod executor_run_structured;

use executor_run_classify::{
    classify_prompt_result, classify_think_result, classify_tool_result, classify_workflow_result,
};
pub(super) use executor_run_exec::{
    THINKING_PER_THOUGHT_BUDGET_SECS, WORKFLOW_BUDGET_SECS, await_with_budget, run_prompt_node,
    run_tool_node,
};
pub(crate) use executor_run_persist::persist_active_node_session;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PromptExecutionResult {
    pub response: String,
    pub acp_session_id: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToolExecutionResult {
    pub response: String,
    pub provenance: ToolProvenance,
}

pub(super) enum NodeTaskResult {
    Workflow(Result<runner::WorkflowResult>),
    Prompt(Result<PromptExecutionResult>),
    SequentialThinking(Result<runner::ThinkResult>),
    Tool(Result<ToolExecutionResult>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FailurePhase {
    PreExecution,
    PostPrompt,
}

impl FailurePhase {
    pub(super) fn is_retryable(&self) -> bool {
        matches!(self, FailurePhase::PreExecution)
    }
}

pub(super) enum NodeOutcome {
    Completed {
        conclusion: String,
        acp_session_id: Option<String>,
        thinking_session_id: Option<String>,
        tool_provenance: Option<ToolProvenance>,
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        model: Option<String>,
        retry_attempts: u32,
    },
    Abandoned {
        reason: String,
        phase: FailurePhase,
        transient_error_kind: Option<&'static str>,
        acp_session_id: Option<String>,
        thinking_session_id: Option<String>,
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        model: Option<String>,
        retry_attempts: u32,
        is_outer_timeout: bool,
    },
}

const PROMPT_FAILURE_PREFIX: &str = "\u{0}pipeline_prompt_failure:";
const PROMPT_HARD_FAILURE_PREFIX: &str = "\u{0}pipeline_prompt_hard_failure:";

pub(super) fn classify_node_result(node_id: &str, result: NodeTaskResult) -> NodeOutcome {
    match result {
        NodeTaskResult::Workflow(result) => {
            let acp_session_id = result
                .as_ref()
                .ok()
                .and_then(|workflow| workflow.acp_session_id.clone());
            classify_workflow_result(node_id, result, acp_session_id)
        }
        NodeTaskResult::Prompt(result) => {
            let acp_session_id = result
                .as_ref()
                .ok()
                .and_then(|prompt| prompt.acp_session_id.clone());
            classify_prompt_result(node_id, result, acp_session_id)
        }
        NodeTaskResult::SequentialThinking(result) => {
            let acp_session_id = result
                .as_ref()
                .ok()
                .and_then(|think| think.acp_session_id.clone());
            let thinking_session_id = result.as_ref().ok().map(|think| think.session_id.clone());
            classify_think_result(node_id, result, acp_session_id, thinking_session_id)
        }
        NodeTaskResult::Tool(result) => classify_tool_result(node_id, result),
    }
}

fn classify_retryable_error(
    error: &MaenifoldError,
    rendered: &str,
) -> (FailurePhase, Option<&'static str>) {
    use std::io::ErrorKind;

    let rendered_lower = rendered.to_ascii_lowercase();
    let message = match error {
        MaenifoldError::Message(message) => message.as_str(),
        MaenifoldError::AgentSetup(message) => message.as_str(),
        _ => rendered,
    };
    let message_lower = message.to_ascii_lowercase();

    if message_lower.contains("timed out")
        || rendered_lower.contains("deadline has elapsed")
        || rendered_lower.contains("deadline elapsed")
    {
        return (FailurePhase::PreExecution, Some("timeout"));
    }

    if message_lower.contains("cancelled") || message_lower.contains("canceled") {
        return (FailurePhase::PostPrompt, None);
    }

    if let MaenifoldError::Io(io_error) = error {
        let kind = match io_error.kind() {
            ErrorKind::Interrupted => Some("io-interrupted"),
            ErrorKind::TimedOut => Some("io-timeout"),
            ErrorKind::ConnectionReset => Some("io-connection-reset"),
            ErrorKind::BrokenPipe => Some("io-broken-pipe"),
            _ => None,
        };
        if let Some(kind) = kind {
            return (FailurePhase::PreExecution, Some(kind));
        }
    }

    if is_retryable_mcp_error_code(message) {
        return (FailurePhase::PreExecution, Some("mcp-retryable-code"));
    }

    if message.contains("external MCP")
        || message.contains("Failed to spawn")
        || message.contains("Failed to connect")
        || message.contains("Failed to invoke external MCP tool")
        || message.contains("Failed to build HTTP client for MCP server")
    {
        return (FailurePhase::PreExecution, Some("external-mcp"));
    }

    (FailurePhase::PostPrompt, None)
}

fn is_retryable_mcp_error_code(message: &str) -> bool {
    const RETRYABLE_CODES: [&str; 6] = [
        "code=-32001",
        "code = -32001",
        "code=-32002",
        "code = -32002",
        "code=-32004",
        "code = -32004",
    ];

    RETRYABLE_CODES.iter().any(|code| message.contains(code)) || message.contains("code=429")
}

fn log_node_execution_error(node_id: &str, error: &dyn StdError) -> String {
    let mut rendered = error.to_string();
    let mut source = error.source();
    while let Some(next) = source {
        let next_rendered = next.to_string();
        if !rendered.contains(&next_rendered) {
            rendered.push_str(": ");
            rendered.push_str(&next_rendered);
        }
        source = next.source();
    }
    eprintln!("[pipeline] node {node_id} failed: {rendered}");
    rendered
}

fn log_node_execution_error_str(node_id: &str, message: &str) -> String {
    eprintln!("[pipeline] node {node_id} failed: {message}");
    message.to_string()
}

fn encode_prompt_failure(message: &str) -> String {
    format!("{PROMPT_FAILURE_PREFIX}{message}")
}

fn decode_prompt_failure(response: &str) -> Option<String> {
    response
        .strip_prefix(PROMPT_FAILURE_PREFIX)
        .map(str::to_string)
}

fn encode_prompt_hard_failure(message: &str) -> String {
    format!("{PROMPT_HARD_FAILURE_PREFIX}{message}")
}

fn decode_prompt_hard_failure(response: &str) -> Option<String> {
    response
        .strip_prefix(PROMPT_HARD_FAILURE_PREFIX)
        .map(str::to_string)
}

#[cfg(test)]
#[path = "failure_classification_tests.rs"]
mod failure_classification_tests;
#[cfg(test)]
#[path = "executor_run_tests.rs"]
mod tests;
