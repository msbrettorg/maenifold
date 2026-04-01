use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value as JsonValue;

use crate::acp_native::{AgentPool, SessionOpts};
use crate::error::Result;
use crate::tools::ToolContext;

use super::super::runner;
use super::executor_helpers::{DispatchedNode, assemble_node_prompt};
use super::executor_mapping::mapped_tool_args;
use super::executor_run::{run_prompt_node, run_tool_node};
use super::types::AgentConfig;
use super::{NodeState, PipelineDefinition, PipelineNode};

pub(super) use super::executor_run::{NodeOutcome, NodeTaskResult, classify_node_result};

#[derive(Debug)]
pub(super) enum NodeExecutionRequest {
    Workflow {
        workflow_id: String,
        prompt: String,
        timeout_seconds: Option<u64>,
    },
    Prompt {
        session_id: String,
        prompt: String,
        tool_mode: String,
        session_opts: SessionOpts,
    },
    SequentialThinking {
        node_session_id: String,
        topic: String,
        total_thoughts: u32,
        session_opts: SessionOpts,
    },
    Tool {
        tool_name: String,
        tool_args: Option<JsonValue>,
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
) -> Result<NodeExecutionRequest> {
    Ok(match node {
        PipelineNode::Workflow {
            workflow_id,
            timeout,
            ..
        } => NodeExecutionRequest::Workflow {
            workflow_id: workflow_id.clone(),
            prompt: assemble_node_prompt(context, definition, node_states, node, ambient_prompt)?,
            timeout_seconds: *timeout,
        },
        PipelineNode::Prompt {
            tool_mode,
            agent_config,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            timeout,
            ..
        } => NodeExecutionRequest::Prompt {
            session_id: node_session_id(node_states, node.id(), dispatch),
            prompt: assemble_node_prompt(context, definition, node_states, node, ambient_prompt)?,
            tool_mode: tool_mode.clone(),
            session_opts: merged_session_opts(
                context,
                node.id(),
                agent_config.as_ref(),
                model,
                thought_level,
                allowed_tools,
                *max_turns,
                *timeout,
            ),
        },
        PipelineNode::SequentialThinking {
            total_thoughts,
            agent_config,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            timeout,
            ..
        } => NodeExecutionRequest::SequentialThinking {
            node_session_id: node_session_id(node_states, node.id(), dispatch),
            topic: assemble_node_prompt(context, definition, node_states, node, ambient_prompt)?,
            total_thoughts: *total_thoughts,
            session_opts: merged_session_opts(
                context,
                node.id(),
                agent_config.as_ref(),
                model,
                thought_level,
                allowed_tools,
                *max_turns,
                *timeout,
            ),
        },
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

fn merged_session_opts(
    context: &ToolContext,
    _node_id: &str,
    agent_config: Option<&AgentConfig>,
    model: &Option<String>,
    thought_level: &Option<String>,
    allowed_tools: &Option<Vec<String>>,
    max_turns: Option<u32>,
    timeout_seconds: Option<u64>,
) -> SessionOpts {
    SessionOpts {
        cwd: context.config.maenifold_root.clone(),
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
        mcp_servers: vec![],
    }
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
            timeout_seconds,
        } => match agent.as_deref() {
            Some(agent) => NodeTaskResult::Workflow(
                runner::run_workflow_with_pool(
                    &context,
                    &workflow_id,
                    agent,
                    &prompt,
                    timeout_seconds,
                    pool.clone(),
                )
                .await,
            ),
            None => {
                NodeTaskResult::Workflow(Err(missing_agent_configuration(&node_id, "workflow")))
            }
        },
        NodeExecutionRequest::Prompt {
            session_id,
            prompt,
            tool_mode,
            session_opts,
        } => match agent.as_deref() {
            Some(agent) => NodeTaskResult::Prompt(
                run_prompt_node(
                    &context,
                    pool.as_ref(),
                    agent,
                    &session_id,
                    &prompt,
                    &tool_mode,
                    session_opts,
                )
                .await,
            ),
            None => NodeTaskResult::Prompt(Err(missing_agent_configuration(&node_id, "prompt"))),
        },
        NodeExecutionRequest::SequentialThinking {
            node_session_id,
            topic,
            total_thoughts,
            session_opts,
        } => match agent.as_deref() {
            Some(agent) => NodeTaskResult::SequentialThinking(
                runner::run_think_with_pool(
                    &context,
                    agent,
                    &topic,
                    runner::ThinkOptions {
                        total_thoughts,
                        parent_workflow_id: Some(node_session_id),
                    },
                    session_opts,
                    pool,
                )
                .await,
            ),
            None => NodeTaskResult::SequentialThinking(Err(missing_agent_configuration(
                &node_id,
                "sequentialThinking",
            ))),
        },
        NodeExecutionRequest::Tool {
            tool_name,
            tool_args,
        } => NodeTaskResult::Tool(run_tool_node(&context, &tool_name, tool_args.as_ref()).await),
    };

    (node_id, result)
}

fn missing_agent_configuration(node_id: &str, node_type: &str) -> crate::error::MaenifoldError {
    crate::error::MaenifoldError::Message(format!(
        "ERROR: Pipeline node '{node_id}' of type '{node_type}' is missing required agent configuration"
    ))
}
