use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use jockey::EventSender;
use serde_json::Value as JsonValue;

use crate::error::Result;
use crate::tools::ToolContext;

use super::super::runner::AgentSessionConfig;
use super::executor_nodes_cwd::resolve_node_cwd;
use super::executor_helpers::{
    DispatchedNode, assemble_prompt_from_parts, predecessor_context_content,
};
use super::mapping_apply::mapped_tool_args;
use super::mapping_resolve::{interpolate_string, pipeline_inputs_scope};
use super::types::AgentConfig;
use super::{NodeState, PipelineDefinition, PipelineNode};

#[cfg(test)]
pub(super) use super::executor_run::FailurePhase;
pub(super) use super::executor_run::{NodeOutcome, NodeTaskResult, classify_node_result};

#[derive(Debug, Clone)]
pub(super) enum NodeExecutionRequest {
    Workflow {
        workflow_id: String,
        existing_session_id: Option<String>,
        prompt: String,
        session_config: AgentSessionConfig,
    },
    Prompt {
        session_id: String,
        prompt: String,
        tool_mode: String,
        session_config: AgentSessionConfig,
    },
    SequentialThinking {
        node_session_id: String,
        topic: String,
        prompt_context: String,
        total_thoughts: u32,
        session_config: AgentSessionConfig,
    },
    Tool {
        tool_name: String,
        tool_args: Option<JsonValue>,
        agent: Option<String>,
        allowed_tools: Option<Vec<String>>,
        cwd: PathBuf,
        timeout_seconds: Option<u64>,
    },
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_node_execution_request(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node: &PipelineNode,
    dispatch: &DispatchedNode,
    ambient_prompt: &str,
    inputs: &serde_json::Map<String, JsonValue>,
) -> Result<NodeExecutionRequest> {
    let scope = pipeline_inputs_scope(inputs);
    Ok(match node {
        PipelineNode::Workflow {
            workflow_id,
            prompt,
            cwd,
            timeout,
            agent_config,
            ..
        } => {
            let session_id = node_session_id(node_states, node.id(), dispatch);
            NodeExecutionRequest::Workflow {
                workflow_id: workflow_id.clone(),
                existing_session_id: Some(session_id.clone()),
                prompt: resolve_execution_prompt(
                    context,
                    definition,
                    node_states,
                    node.id(),
                    ambient_prompt,
                    prompt.as_deref(),
                    true,
                    &scope,
                )?,
                session_config: merged_session_config(
                    context,
                    node.id(),
                    &session_id,
                    agent_config.as_ref(),
                    cwd.as_deref(),
                    &None,
                    &None,
                    &None,
                    None,
                    *timeout,
                )?,
            }
        }
        PipelineNode::Prompt {
            prompt,
            tool_mode,
            agent_config,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            cwd,
            timeout,
            ..
        } => {
            let session_id = node_session_id(node_states, node.id(), dispatch);
            NodeExecutionRequest::Prompt {
                session_id: session_id.clone(),
                prompt: resolve_execution_prompt(
                    context,
                    definition,
                    node_states,
                    node.id(),
                    ambient_prompt,
                    Some(prompt.as_str()),
                    false,
                    &scope,
                )?,
                tool_mode: tool_mode.clone(),
                session_config: merged_session_config(
                    context,
                    node.id(),
                    &session_id,
                    agent_config.as_ref(),
                    cwd.as_deref(),
                    model,
                    thought_level,
                    allowed_tools,
                    *max_turns,
                    *timeout,
                )?,
            }
        }
        PipelineNode::SequentialThinking {
            topic,
            total_thoughts,
            agent_config,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            cwd,
            timeout,
            ..
        } => {
            let session_id = node_session_id(node_states, node.id(), dispatch);
            NodeExecutionRequest::SequentialThinking {
                node_session_id: session_id.clone(),
                topic: interpolate_string(topic.trim(), &scope)?,
                prompt_context: resolve_execution_prompt(
                    context,
                    definition,
                    node_states,
                    node.id(),
                    ambient_prompt,
                    Some(topic.as_str()),
                    false,
                    &scope,
                )?,
                total_thoughts: *total_thoughts,
                session_config: merged_session_config(
                    context,
                    node.id(),
                    &session_id,
                    agent_config.as_ref(),
                    cwd.as_deref(),
                    model,
                    thought_level,
                    allowed_tools,
                    *max_turns,
                    *timeout,
                )?,
            }
        }
        PipelineNode::Tool {
            tool_name,
            tool_args,
            agent,
            allowed_tools,
            timeout,
            ..
        } => NodeExecutionRequest::Tool {
            tool_name: tool_name.clone(),
            tool_args: mapped_tool_args(
                definition,
                node_states,
                node.id(),
                tool_args.clone(),
                &scope,
            )?,
            agent: agent.clone(),
            allowed_tools: allowed_tools.clone(),
            cwd: resolve_node_cwd(context, node.cwd())?,
            timeout_seconds: *timeout,
        },
    })
}

fn resolve_execution_prompt(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
    ambient_prompt: &str,
    node_instruction: Option<&str>,
    workflow_fallback: bool,
    scope: &JsonValue,
) -> Result<String> {
    let predecessor_context =
        predecessor_context_content(context, definition, node_states, node_id)?;
    let ambient_prompt = interpolate_string(ambient_prompt.trim(), scope)?;
    let node_instruction = node_instruction
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| interpolate_string(value, scope))
        .transpose()?;
    Ok(assemble_prompt_from_parts(
        predecessor_context.as_deref(),
        &ambient_prompt,
        node_instruction.as_deref(),
        workflow_fallback,
    ))
}

fn node_session_id(
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
    dispatch: &DispatchedNode,
) -> String {
    dispatch.session_id.clone().unwrap_or_else(|| {
        node_states
            .get(node_id)
            .map(|state| state.session_id.clone())
            .unwrap_or_else(|| format!("pipeline-{node_id}"))
    })
}

#[allow(clippy::too_many_arguments)]
fn merged_session_config(
    context: &ToolContext,
    _node_id: &str,
    session_id: &str,
    agent_config: Option<&AgentConfig>,
    node_cwd: Option<&Path>,
    model: &Option<String>,
    thought_level: &Option<String>,
    allowed_tools: &Option<Vec<String>>,
    max_turns: Option<u32>,
    timeout_seconds: Option<u64>,
) -> Result<AgentSessionConfig> {
    Ok(AgentSessionConfig {
        cwd: resolve_node_cwd(context, node_cwd)?,
        model: model
            .clone()
            .or_else(|| agent_config.and_then(|config| config.model.clone())),
        mode: agent_config.and_then(|config| config.mode.clone()),
        thought_level: thought_level
            .clone()
            .or_else(|| agent_config.and_then(|config| config.thought_level.clone())),
        config_options: agent_config.and_then(|config| config.config_options.clone()),
        allowed_tools: allowed_tools.clone(),
        max_turns,
        timeout_seconds,
        session_id: Some(session_id.to_string()),
        auto_approve: true, // Pipeline nodes run headless; auto-grant permissions.
    })
}



pub(super) async fn execute_node_request(
    context: ToolContext,
    node_id: String,
    execution: NodeExecutionRequest,
    agent: Option<String>,
    event_tx: EventSender,
) -> (String, NodeTaskResult) {
    super::executor_nodes_dispatch::execute_node_request(
        context, node_id, execution, agent, event_tx,
    )
    .await
}

#[cfg(test)]
#[path = "executor_nodes_tests.rs"]
mod tests;
