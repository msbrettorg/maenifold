use std::error::Error as StdError;

use serde_json::Value as JsonValue;

use crate::acp_native::{AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::super::runner::{self, WorkflowRunStatus};
use super::executor_helpers::{MAX_CONCLUSION_CHARS, truncate_chars};

pub(super) enum NodeTaskResult {
    Workflow(Result<runner::WorkflowResult>),
    Prompt(Result<String>),
    SequentialThinking(Result<runner::ThinkResult>),
    Tool(Result<String>),
}

pub(super) enum NodeOutcome {
    Completed {
        conclusion: String,
        thinking_session_id: Option<String>,
    },
    Abandoned {
        reason: String,
    },
}

pub(super) fn classify_node_result(node_id: &str, result: NodeTaskResult) -> NodeOutcome {
    match result {
        NodeTaskResult::Workflow(result) => classify_workflow_result(node_id, result),
        NodeTaskResult::Prompt(result) => classify_prompt_result(node_id, result),
        NodeTaskResult::SequentialThinking(result) => classify_think_result(node_id, result),
        NodeTaskResult::Tool(result) => classify_prompt_result(node_id, result),
    }
}

fn classify_workflow_result(node_id: &str, result: Result<runner::WorkflowResult>) -> NodeOutcome {
    match result {
        Ok(workflow) => match workflow.status {
            WorkflowRunStatus::Completed => NodeOutcome::Completed {
                conclusion: truncate_chars(
                    &workflow.conclusion.unwrap_or_default(),
                    MAX_CONCLUSION_CHARS,
                ),
                thinking_session_id: None,
            },
            WorkflowRunStatus::Failed(ref msg) => {
                eprintln!("[pipeline] node {node_id} workflow failed: {msg}");
                NodeOutcome::Abandoned {
                    reason: msg.clone(),
                }
            }
            WorkflowRunStatus::Cancelled => {
                eprintln!("[pipeline] node {node_id} workflow was cancelled, marking abandoned");
                NodeOutcome::Abandoned {
                    reason: "workflow cancelled".to_string(),
                }
            }
        },
        Err(error) => NodeOutcome::Abandoned {
            reason: log_node_execution_error(node_id, &error),
        },
    }
}

fn classify_prompt_result(node_id: &str, result: Result<String>) -> NodeOutcome {
    match result {
        Ok(response) if !response.trim().is_empty() => NodeOutcome::Completed {
            conclusion: response,
            thinking_session_id: None,
        },
        Ok(_) => NodeOutcome::Abandoned {
            reason: "empty response".to_string(),
        },
        Err(error) => NodeOutcome::Abandoned {
            reason: log_node_execution_error(node_id, &error),
        },
    }
}

fn classify_think_result(node_id: &str, result: Result<runner::ThinkResult>) -> NodeOutcome {
    match result {
        Ok(think) => think
            .conclusion
            .map(|conclusion| conclusion.trim().to_string())
            .filter(|conclusion| !conclusion.is_empty())
            .map(|conclusion| NodeOutcome::Completed {
                conclusion,
                thinking_session_id: Some(think.session_id),
            })
            .unwrap_or(NodeOutcome::Abandoned {
                reason: "missing conclusion".to_string(),
            }),
        Err(error) => NodeOutcome::Abandoned {
            reason: log_node_execution_error(node_id, &error),
        },
    }
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

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_prompt_node(
    _context: &ToolContext,
    pool: &AgentPool,
    agent: &str,
    _session_id: &str,
    prompt: &str,
    _tool_mode: &str,
    session_opts: SessionOpts,
) -> Result<String> {
    super::super::runner::run_prompt_with_pool(pool, agent, prompt, session_opts).await
}

pub(super) async fn run_tool_node(
    _context: &ToolContext,
    tool_name: &str,
    tool_args: Option<&JsonValue>,
) -> Result<String> {
    let arguments = tool_args
        .cloned()
        .unwrap_or_else(|| JsonValue::Object(Default::default()));
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
        let outcome = classify_prompt_result("prompt-node", Ok("  \n\t  ".to_string()));

        match outcome {
            NodeOutcome::Abandoned { reason } => assert_eq!(reason, "empty response"),
            NodeOutcome::Completed { .. } => {
                panic!("whitespace-only prompt output should be abandoned")
            }
        }
    }

    #[test]
    fn classify_prompt_result_completes_non_empty_output() {
        let outcome = classify_prompt_result("prompt-node", Ok("  kept  ".to_string()));

        match outcome {
            NodeOutcome::Completed { conclusion, .. } => assert_eq!(conclusion, "  kept  "),
            NodeOutcome::Abandoned { reason } => {
                panic!("non-empty prompt output should complete, got: {reason}")
            }
        }
    }
}
