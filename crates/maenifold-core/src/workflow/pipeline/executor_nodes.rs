use std::collections::HashMap;
use std::error::Error as StdError;
use std::sync::Arc;

use serde_json::Value as JsonValue;

use crate::acp_native::AgentPool;
use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::super::agent_session::open_agent_session;
use super::super::runner::{self, WorkflowRunStatus};
use super::executor_helpers::{
    DispatchedNode, MAX_CONCLUSION_CHARS, assemble_node_prompt, truncate_chars,
};
use super::executor_mapping::mapped_tool_args;
use super::{NodeState, PipelineDefinition, PipelineNode};

#[derive(Debug)]
pub(super) enum NodeExecutionRequest {
    Workflow {
        workflow_id: String,
        prompt: String,
    },
    Prompt {
        session_id: String,
        prompt: String,
        tool_mode: String,
    },
    SequentialThinking {
        node_session_id: String,
        topic: String,
        total_thoughts: u32,
    },
    Tool {
        tool_name: String,
        tool_args: Option<JsonValue>,
    },
}

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
    Abandoned,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_node_execution_request(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node: &PipelineNode,
    dispatch: &DispatchedNode,
    ambient_prompt: &str,
) -> Result<NodeExecutionRequest> {
    Ok(match node {
        PipelineNode::Workflow { workflow_id, .. } => NodeExecutionRequest::Workflow {
            workflow_id: workflow_id.clone(),
            prompt: assemble_node_prompt(context, definition, node_states, node, ambient_prompt)?,
        },
        PipelineNode::Prompt { tool_mode, .. } => NodeExecutionRequest::Prompt {
            session_id: dispatch.session_id.clone().unwrap_or_else(|| {
                node_states
                    .get(node.id())
                    .map(|state| state.session_id.clone())
                    .unwrap_or_else(|| format!("pipeline-{}", node.id()))
            }),
            prompt: assemble_node_prompt(context, definition, node_states, node, ambient_prompt)?,
            tool_mode: tool_mode.clone(),
        },
        PipelineNode::SequentialThinking { total_thoughts, .. } => {
            NodeExecutionRequest::SequentialThinking {
                node_session_id: dispatch.session_id.clone().unwrap_or_else(|| {
                    node_states
                        .get(node.id())
                        .map(|state| state.session_id.clone())
                        .unwrap_or_else(|| format!("pipeline-{}", node.id()))
                }),
                topic: assemble_node_prompt(
                    context,
                    definition,
                    node_states,
                    node,
                    ambient_prompt,
                )?,
                total_thoughts: *total_thoughts,
            }
        }
        PipelineNode::Tool {
            tool_name,
            tool_args,
            ..
        } => NodeExecutionRequest::Tool {
            tool_name: tool_name.clone(),
            tool_args: mapped_tool_args(definition, node_states, node.id(), tool_args.clone()),
        },
    })
}

pub(super) async fn execute_node_request(
    context: ToolContext,
    node_id: String,
    execution: NodeExecutionRequest,
    agent: Option<String>,
    pool: Arc<AgentPool>,
) -> (String, NodeTaskResult) {
    let result = match execution {
        NodeExecutionRequest::Workflow {
            workflow_id,
            prompt,
        } => {
            let agent = agent.as_deref().expect("agent required for workflow nodes");
            NodeTaskResult::Workflow(
                runner::run_workflow_with_pool(
                    &context,
                    &workflow_id,
                    agent,
                    &prompt,
                    pool.clone(),
                )
                .await,
            )
        }
        NodeExecutionRequest::Prompt {
            session_id,
            prompt,
            tool_mode,
        } => {
            let agent = agent.as_deref().expect("agent required for prompt nodes");
            NodeTaskResult::Prompt(
                run_prompt_node(
                    &context,
                    pool.as_ref(),
                    agent,
                    &session_id,
                    &prompt,
                    &tool_mode,
                )
                .await,
            )
        }
        NodeExecutionRequest::SequentialThinking {
            node_session_id,
            topic,
            total_thoughts,
        } => {
            let agent = agent.as_deref().expect("agent required for thinking nodes");
            NodeTaskResult::SequentialThinking(
                runner::run_think_with_pool(
                    &context,
                    agent,
                    &topic,
                    runner::ThinkOptions {
                        total_thoughts,
                        parent_workflow_id: Some(node_session_id),
                    },
                    pool,
                )
                .await,
            )
        }
        NodeExecutionRequest::Tool {
            tool_name,
            tool_args,
        } => NodeTaskResult::Tool(run_tool_node(&context, &tool_name, tool_args.as_ref()).await),
    };

    (node_id, result)
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
                NodeOutcome::Abandoned
            }
            WorkflowRunStatus::Cancelled => {
                eprintln!("[pipeline] node {node_id} workflow was cancelled, marking abandoned");
                NodeOutcome::Abandoned
            }
        },
        Err(error) => {
            log_node_execution_error(node_id, &error);
            NodeOutcome::Abandoned
        }
    }
}

fn classify_prompt_result(node_id: &str, result: Result<String>) -> NodeOutcome {
    match result {
        Ok(response) if !response.trim().is_empty() => NodeOutcome::Completed {
            conclusion: response,
            thinking_session_id: None,
        },
        Ok(_) => NodeOutcome::Abandoned,
        Err(error) => {
            log_node_execution_error(node_id, &error);
            NodeOutcome::Abandoned
        }
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
            .unwrap_or(NodeOutcome::Abandoned),
        Err(error) => {
            log_node_execution_error(node_id, &error);
            NodeOutcome::Abandoned
        }
    }
}

fn log_node_execution_error(node_id: &str, error: &dyn StdError) {
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
}

async fn run_prompt_node(
    context: &ToolContext,
    pool: &AgentPool,
    agent: &str,
    session_id: &str,
    prompt: &str,
    _tool_mode: &str,
) -> Result<String> {
    let session =
        open_agent_session(pool, &context.config.maenifold_root, agent, session_id).await?;
    let response = session.prompt(prompt).await;
    let _ = session.close().await;
    let response = response?;
    if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
        return Err(MaenifoldError::Message(error));
    }
    Ok(response.text.trim().to_string())
}

async fn run_tool_node(
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
