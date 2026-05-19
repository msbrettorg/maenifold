use std::{collections::HashMap, path::PathBuf};

use serde_json::Value as JsonValue;

use crate::error::Result;
use crate::tools::ToolContext;

use super::super::runner::AgentSessionConfig;
use super::executor_helpers::DispatchedNode;
use super::executor_nodes_build_helpers::{
    build_adopt_instructions, merged_session_config, node_session_id, resolve_execution_prompt,
    with_output_schema_contract,
};
use super::executor_nodes_cwd::resolve_node_cwd;
use super::mapping_apply::mapped_tool_args;
use super::mapping_resolve::pipeline_inputs_scope;
use super::{NodeState, PipelineDefinition, PipelineNode, validate_pipeline_inputs};

#[cfg(test)]
pub(super) use super::executor_run::FailurePhase;
pub(super) use super::executor_run::{NodeOutcome, NodeTaskResult, classify_node_result};

#[derive(Debug, Clone)]
pub(super) enum NodeExecutionRequest {
    Workflow {
        workflow_id: String,
        existing_session_id: Option<String>,
        prompt: String,
        adopt_instructions: Vec<String>,
        adopt_role: Option<String>,
        requires_role: Option<String>,
        session_config: AgentSessionConfig,
    },
    Prompt {
        session_id: String,
        prompt: String,
        tool_mode: String,
        adopt_instructions: Vec<String>,
        adopt_role: Option<String>,
        requires_role: Option<String>,
        session_config: AgentSessionConfig,
    },
    SequentialThinking {
        node_session_id: String,
        topic: String,
        prompt_context: String,
        total_thoughts: u32,
        adopt_instructions: Vec<String>,
        adopt_role: Option<String>,
        requires_role: Option<String>,
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
    if let Some(schema) = &definition.input_schema {
        validate_pipeline_inputs(schema, inputs)?;
    }
    Ok(match node {
        PipelineNode::Workflow {
            workflow_id,
            prompt,
            cwd,
            timeout,
            agent_config,
            adopt,
            requires_role,
            output_schema,
            ..
        } => {
            let session_id = node_session_id(node_states, node.id(), dispatch);
            let adopt_role = adopt
                .as_ref()
                .and_then(|config| config.role.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
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
                )?,
                adopt_instructions: build_adopt_instructions(
                    context,
                    node.id(),
                    adopt,
                    requires_role,
                )?,
                adopt_role,
                requires_role: requires_role.clone(),
                session_config: with_output_schema_contract(
                    merged_session_config(
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
                        None,
                    )?,
                    output_schema.as_ref(),
                ),
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
            adopt,
            requires_role,
            session_ref,
            output_schema,
            ..
        } => {
            let session_id = node_session_id(node_states, node.id(), dispatch);
            let adopt_role = adopt
                .as_ref()
                .and_then(|config| config.role.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
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
                )?,
                tool_mode: tool_mode.clone(),
                adopt_instructions: build_adopt_instructions(
                    context,
                    node.id(),
                    adopt,
                    requires_role,
                )?,
                adopt_role,
                requires_role: requires_role.clone(),
                session_config: with_output_schema_contract(
                    merged_session_config(
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
                        session_ref.as_ref().map(|sr| sr.session_id.clone()),
                    )?,
                    output_schema.as_ref(),
                ),
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
            adopt,
            requires_role,
            output_schema,
            ..
        } => {
            let session_id = node_session_id(node_states, node.id(), dispatch);
            let adopt_role = adopt
                .as_ref()
                .and_then(|config| config.role.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            NodeExecutionRequest::SequentialThinking {
                node_session_id: session_id.clone(),
                topic: topic.trim().to_string(),
                prompt_context: resolve_execution_prompt(
                    context,
                    definition,
                    node_states,
                    node.id(),
                    ambient_prompt,
                    Some(topic.as_str()),
                    false,
                )?,
                total_thoughts: *total_thoughts,
                adopt_instructions: build_adopt_instructions(
                    context,
                    node.id(),
                    adopt,
                    requires_role,
                )?,
                adopt_role,
                requires_role: requires_role.clone(),
                session_config: with_output_schema_contract(
                    merged_session_config(
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
                        None,
                    )?,
                    output_schema.as_ref(),
                ),
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

#[cfg(test)]
#[path = "executor_nodes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "executor_nodes_tests_extended.rs"]
mod tests_extended;

#[cfg(test)]
#[path = "executor_nodes_tests_interp.rs"]
mod tests_interp;

#[cfg(test)]
#[path = "executor_nodes_tests_runtime.rs"]
mod tests_runtime;
