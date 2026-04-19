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

use super::super::agent_session::{AgentSession, open_agent_session};
use super::super::adoption::require_adopted_role;
use super::super::runner::{self, AgentSessionConfig, WorkflowRunStatus};
use super::super::think_runner::decode_think_failure;
use super::executor_helpers::{MAX_CONCLUSION_CHARS, truncate_chars};
use super::external_mcp::{invoke_external_tool, split_namespaced_tool_name};
use super::types::ToolProvenance;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PromptExecutionResult {
    pub response: String,
    pub acp_session_id: Option<String>,
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
        retry_attempts: u32,
    },
    Abandoned {
        reason: String,
        phase: FailurePhase,
        transient_error_kind: Option<&'static str>,
        acp_session_id: Option<String>,
        thinking_session_id: Option<String>,
        retry_attempts: u32,
        is_outer_timeout: bool,
    },
}

const PROMPT_FAILURE_PREFIX: &str = "\u{0}pipeline_prompt_failure:";

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

fn classify_workflow_result(
    node_id: &str,
    result: Result<runner::WorkflowResult>,
    acp_session_id: Option<String>,
) -> NodeOutcome {
    match result {
        Ok(workflow) => match workflow.status {
            WorkflowRunStatus::Completed => NodeOutcome::Completed {
                conclusion: truncate_chars(
                    &workflow.conclusion.unwrap_or_default(),
                    MAX_CONCLUSION_CHARS,
                ),
                acp_session_id: workflow.acp_session_id,
                thinking_session_id: None,
                tool_provenance: None,
                retry_attempts: 0,
            },
            WorkflowRunStatus::Failed(ref msg) => {
                eprintln!("[pipeline] node {node_id} workflow failed: {msg}");
                NodeOutcome::Abandoned {
                    reason: msg.clone(),
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: workflow.acp_session_id.clone(),
                    thinking_session_id: None,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                }
            }
            WorkflowRunStatus::Cancelled => {
                eprintln!("[pipeline] node {node_id} workflow was cancelled, marking abandoned");
                NodeOutcome::Abandoned {
                    reason: "workflow cancelled".to_string(),
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: workflow.acp_session_id,
                    thinking_session_id: None,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                }
            }
        },
        Err(MaenifoldError::AgentSetup(message)) => NodeOutcome::Abandoned {
            reason: log_node_execution_error_str(node_id, &message),
            phase: FailurePhase::PreExecution,
            transient_error_kind: Some("agent-setup"),
            acp_session_id: None,
            thinking_session_id: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);
            NodeOutcome::Abandoned {
                reason,
                phase,
                transient_error_kind,
                acp_session_id,
                thinking_session_id: None,
                retry_attempts: 0,
                is_outer_timeout: false,
            }
        }
    }
}

fn classify_prompt_result(
    node_id: &str,
    result: Result<PromptExecutionResult>,
    acp_session_id: Option<String>,
) -> NodeOutcome {
    match result {
        Ok(result) => {
            if let Some(reason) = decode_prompt_failure(&result.response) {
                eprintln!("[pipeline] node {node_id} failed: {reason}");
                NodeOutcome::Abandoned {
                    reason,
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: result.acp_session_id,
                    thinking_session_id: None,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                }
            } else if !result.response.trim().is_empty() {
                NodeOutcome::Completed {
                    conclusion: result.response,
                    acp_session_id: result.acp_session_id,
                    thinking_session_id: None,
                    tool_provenance: None,
                    retry_attempts: 0,
                }
            } else {
                NodeOutcome::Abandoned {
                    reason: "empty response".to_string(),
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: result.acp_session_id,
                    thinking_session_id: None,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                }
            }
        }
        Err(MaenifoldError::AgentSetup(message)) => NodeOutcome::Abandoned {
            reason: log_node_execution_error_str(node_id, &message),
            phase: FailurePhase::PreExecution,
            transient_error_kind: Some("agent-setup"),
            acp_session_id: None,
            thinking_session_id: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);
            NodeOutcome::Abandoned {
                reason,
                phase,
                transient_error_kind,
                acp_session_id,
                thinking_session_id: None,
                retry_attempts: 0,
                is_outer_timeout: false,
            }
        }
    }
}

fn classify_think_result(
    node_id: &str,
    result: Result<runner::ThinkResult>,
    acp_session_id: Option<String>,
    thinking_session_id: Option<String>,
) -> NodeOutcome {
    match result {
        Ok(think) => {
            let runner::ThinkResult {
                session_id,
                acp_session_id,
                conclusion,
                ..
            } = think;
            conclusion
                .map(|conclusion| conclusion.trim().to_string())
                .filter(|conclusion| !conclusion.is_empty())
                .map(|conclusion| {
                    if let Some(reason) = decode_think_failure(&conclusion) {
                        NodeOutcome::Abandoned {
                            reason,
                            phase: FailurePhase::PostPrompt,
                            transient_error_kind: None,
                            acp_session_id: acp_session_id.clone(),
                            thinking_session_id: Some(session_id.clone()),
                            retry_attempts: 0,
                            is_outer_timeout: false,
                        }
                    } else {
                        NodeOutcome::Completed {
                            conclusion,
                            acp_session_id: acp_session_id.clone(),
                            thinking_session_id: Some(session_id.clone()),
                            tool_provenance: None,
                            retry_attempts: 0,
                        }
                    }
                })
                .unwrap_or(NodeOutcome::Abandoned {
                    reason: "missing conclusion".to_string(),
                    phase: FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id,
                    thinking_session_id: Some(session_id),
                    retry_attempts: 0,
                    is_outer_timeout: false,
                })
        }
        Err(MaenifoldError::AgentSetup(message)) => NodeOutcome::Abandoned {
            reason: log_node_execution_error_str(node_id, &message),
            phase: FailurePhase::PreExecution,
            transient_error_kind: Some("agent-setup"),
            acp_session_id: None,
            thinking_session_id: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);
            NodeOutcome::Abandoned {
                reason,
                phase,
                transient_error_kind,
                acp_session_id,
                thinking_session_id,
                retry_attempts: 0,
                is_outer_timeout: false,
            }
        }
    }
}

fn classify_tool_result(node_id: &str, result: Result<ToolExecutionResult>) -> NodeOutcome {
    match result {
        Ok(result) if !result.response.trim().is_empty() => NodeOutcome::Completed {
            conclusion: result.response,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: Some(result.provenance),
            retry_attempts: 0,
        },
        Ok(_) => NodeOutcome::Abandoned {
            reason: "empty response".to_string(),
            phase: FailurePhase::PostPrompt,
            transient_error_kind: None,
            acp_session_id: None,
            thinking_session_id: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(MaenifoldError::AgentSetup(message)) => NodeOutcome::Abandoned {
            reason: log_node_execution_error_str(node_id, &message),
            phase: FailurePhase::PreExecution,
            transient_error_kind: Some("agent-setup"),
            acp_session_id: None,
            thinking_session_id: None,
            retry_attempts: 0,
            is_outer_timeout: false,
        },
        Err(error) => {
            let reason = log_node_execution_error(node_id, &error);
            let (phase, transient_error_kind) = classify_retryable_error(&error, &reason);

            NodeOutcome::Abandoned {
                reason,
                phase,
                transient_error_kind,
                acp_session_id: None,
                thinking_session_id: None,
                retry_attempts: 0,
                is_outer_timeout: false,
            }
        }
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

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_prompt_node(
    _context: &ToolContext,
    event_tx: &EventSender,
    agent: &str,
    _session_id: &str,
    prompt: &str,
    _tool_mode: &str,
    adopt_instructions: &[String],
    adopt_role: Option<&str>,
    requires_role: Option<&str>,
    session_config: AgentSessionConfig,
    keep_session_alive: bool,
) -> Result<PromptExecutionResult> {
    require_adopted_role("prompt invocation", requires_role, adopt_role)?;
    let session = open_agent_session(agent, &session_config, event_tx).await?;
    apply_adopt_instructions(&session, adopt_instructions).await?;
    let response_result = session.prompt(prompt).await;
    let acp_session_id = session.remote_id();
    let prompt_result = match response_result {
        Ok(response) => {
            if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
                Ok(PromptExecutionResult {
                    response: encode_prompt_failure(&error),
                    acp_session_id: Some(acp_session_id),
                })
            } else {
                Ok(PromptExecutionResult {
                    response: response.text.trim().to_string(),
                    acp_session_id: Some(acp_session_id),
                })
            }
        }
        Err(error) => Ok(PromptExecutionResult {
            response: encode_prompt_failure(&error.to_string()),
            acp_session_id: Some(acp_session_id),
        }),
    };
    if keep_session_alive {
        session.keep_alive();
        prompt_result
    } else {
        let close_result = session.close().await;
        match (prompt_result, close_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(value), Err(error)) => {
                let response = match decode_prompt_failure(&value.response) {
                    Some(reason) => encode_prompt_failure(
                        &runner::append_close_error_context(MaenifoldError::Message(reason), error)
                            .to_string(),
                    ),
                    None => return Err(error),
                };
                Ok(PromptExecutionResult {
                    response,
                    acp_session_id: value.acp_session_id,
                })
            }
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(close_error)) => {
                Err(runner::append_close_error_context(error, close_error))
            }
        }
    }
}

pub(super) async fn apply_adopt_instructions(
    session: &AgentSession,
    adopt_instructions: &[String],
) -> Result<()> {
    for instruction in adopt_instructions {
        let response = session.prompt(instruction).await?;
        if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
            return Err(MaenifoldError::Message(error));
        }
    }
    Ok(())
}

pub(super) async fn run_tool_node(
    context: &ToolContext,
    tool_name: &str,
    tool_args: Option<&JsonValue>,
    agent: Option<&str>,
    allowed_tools: Option<&[String]>,
    cwd: &Path,
) -> Result<ToolExecutionResult> {
    info!(tool_name, "executing pipeline tool node");
    let arguments = tool_args
        .cloned()
        .unwrap_or_else(|| JsonValue::Object(Default::default()));

    let response = async {
        if let Some((server_name, remote_tool_name)) = split_namespaced_tool_name(tool_name) {
            if server_name == "maenifold" {
                Ok(invoke_local_tool(remote_tool_name, &arguments)?)
            } else {
                let agents_config = AgentsConfig::load(cwd);
                invoke_external_tool(
                    context,
                    &agents_config,
                    server_name,
                    remote_tool_name,
                    tool_args,
                    agent,
                    allowed_tools,
                    cwd,
                )
                .await
            }
        } else {
            Ok(invoke_local_tool(tool_name, &arguments)?)
        }
    }
    .await;

    match response {
        Ok(response) => {
            let response_summary = truncate_chars(&response, MAX_CONCLUSION_CHARS);
            Ok(ToolExecutionResult {
                response,
                provenance: ToolProvenance {
                    tool_name: tool_name.to_string(),
                    resolved_args: Some(arguments),
                    response_summary,
                },
            })
        }
        Err(e) => {
            error!(tool_name, error = %e, "pipeline tool node failed");
            Err(e)
        }
    }
}

fn invoke_local_tool(tool_name: &str, arguments: &JsonValue) -> Result<String> {
    ToolRegistry::global()
        .try_invoke(tool_name, &arguments, OutputFormat::Json)
        .map_err(|error| {
            if matches!(
                error,
                MaenifoldError::UnknownTool(_)
                    | MaenifoldError::InvalidPayload(_)
                    | MaenifoldError::InvalidArgument(_)
                    | MaenifoldError::InvalidResourceUri(_)
                    | MaenifoldError::AssetNotFound(_, _)
                    | MaenifoldError::PathTraversal(_)
                    | MaenifoldError::MissingProperty(_)
            ) {
                error
            } else {
                MaenifoldError::Message(format!("Error invoking {tool_name}: {error}"))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_prompt_result_abandons_whitespace_only_output() {
        let outcome = classify_prompt_result(
            "prompt-node",
            Ok(PromptExecutionResult {
                response: "  \n\t  ".to_string(),
                acp_session_id: Some("acp-prompt-1".to_string()),
            }),
            None,
        );

        match outcome {
            NodeOutcome::Abandoned { reason, .. } => assert_eq!(reason, "empty response"),
            NodeOutcome::Completed { .. } => {
                panic!("whitespace-only prompt output should be abandoned")
            }
        }
    }

    #[test]
    fn classify_prompt_result_completes_non_empty_output() {
        let outcome = classify_prompt_result(
            "prompt-node",
            Ok(PromptExecutionResult {
                response: "  kept  ".to_string(),
                acp_session_id: Some("acp-prompt-2".to_string()),
            }),
            None,
        );

        match outcome {
            NodeOutcome::Completed {
                conclusion,
                acp_session_id,
                ..
            } => {
                assert_eq!(conclusion, "  kept  ");
                assert_eq!(acp_session_id.as_deref(), Some("acp-prompt-2"));
            }
            NodeOutcome::Abandoned { reason, .. } => {
                panic!("non-empty prompt output should complete, got: {reason}")
            }
        }
    }

    #[test]
    fn classify_tool_result_records_structured_provenance() {
        let outcome = classify_tool_result(
            "tool-node",
            Ok(ToolExecutionResult {
                response: "{\"success\":true}".to_string(),
                provenance: ToolProvenance {
                    tool_name: "example.echo".to_string(),
                    resolved_args: Some(serde_json::json!({ "value": 1 })),
                    response_summary: "{\"success\":true}".to_string(),
                },
            }),
        );

        match outcome {
            NodeOutcome::Completed {
                conclusion,
                tool_provenance,
                ..
            } => {
                assert_eq!(conclusion, "{\"success\":true}");
                let provenance = tool_provenance.expect("tool provenance should be captured");
                assert_eq!(provenance.tool_name, "example.echo");
                assert_eq!(
                    provenance.resolved_args,
                    Some(serde_json::json!({ "value": 1 }))
                );
                assert_eq!(provenance.response_summary, "{\"success\":true}");
            }
            NodeOutcome::Abandoned { reason, .. } => {
                panic!("successful tool output should complete, got: {reason}")
            }
        }
    }
}

#[cfg(test)]
#[path = "failure_classification_tests.rs"]
mod failure_classification_tests;
