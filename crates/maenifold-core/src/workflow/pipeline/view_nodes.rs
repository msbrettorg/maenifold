use crate::error::Result;
use crate::tools::ToolContext;
use serde_json::{Value, json};
use std::collections::HashMap;

use super::view::node_state_for;
use super::view_cognitive::{
    assembled_prompt_for_view, parse_thinking_trace, parse_workflow_steps,
};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

pub(super) fn pipeline_nodes_json(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    ambient_prompt: &str,
) -> Result<Vec<Value>> {
    definition
        .nodes
        .iter()
        .map(|node| {
            let state = node_state_for(node_states, node.id())?;
            let mut node_json = json!({
                "nodeId": node.id(),
                "sessionId": state.session_id,
                "status": state.status,
                "attempts": state.attempts,
                "output": state.output,
                "type": node.node_type_name(),
            });
            add_pipeline_node_view_fields(&mut node_json, node);
            let assembled_prompt = assembled_prompt_for_view(
                context,
                definition,
                node_states,
                node,
                state,
                ambient_prompt,
            )?;
            if let Some(node_object) = node_json.as_object_mut() {
                if let Some(display_label) = node.display_label() {
                    node_object.insert("displayLabel".to_string(), json!(display_label));
                }
                if let Some(retry_after) = &state.retry_after {
                    node_object.insert("retryAfter".to_string(), json!(retry_after));
                }
                if let Some(started_at) = &state.started_at {
                    node_object.insert("startedAt".to_string(), json!(started_at));
                }
                if let Some(completed_at) = &state.completed_at {
                    node_object.insert("completedAt".to_string(), json!(completed_at));
                }
                if let Some(retry_count) = state.retry_count {
                    node_object.insert("retryCount".to_string(), json!(retry_count));
                }
                if let Some(input_tokens) = state.input_tokens {
                    node_object.insert("inputTokens".to_string(), json!(input_tokens));
                }
                if let Some(output_tokens) = state.output_tokens {
                    node_object.insert("outputTokens".to_string(), json!(output_tokens));
                }
                if let Some(model) = &state.model {
                    node_object.insert("model".to_string(), json!(model));
                }
                if let Some(failure_reason) = &state.failure_reason {
                    node_object.insert("failureReason".to_string(), json!(failure_reason));
                }
                if let Some(acp_session_id) = &state.acp_session_id {
                    node_object.insert("acpSessionId".to_string(), json!(acp_session_id));
                }
                if let Some(thinking_session_id) = &state.thinking_session_id {
                    node_object.insert("thinkingSessionId".to_string(), json!(thinking_session_id));
                }
                if let Some(tool_provenance) = &state.tool_provenance {
                    node_object.insert("toolProvenance".to_string(), json!(tool_provenance));
                }
                node_object.insert(
                    "provenance".to_string(),
                    pipeline_node_provenance_json(node, state),
                );
                node_object.insert(
                    "assembled_prompt".to_string(),
                    assembled_prompt.map(Value::String).unwrap_or(Value::Null),
                );
                let workflow_steps = if matches!(node, PipelineNode::Workflow { .. })
                    && matches!(state.status, NodeStatus::Active | NodeStatus::Completed)
                {
                    parse_workflow_steps(context, &state.session_id)
                } else {
                    None
                };
                node_object.insert(
                    "workflow_steps".to_string(),
                    workflow_steps.map(Value::Array).unwrap_or(Value::Null),
                );
                let thinking_trace = if matches!(node, PipelineNode::SequentialThinking { .. })
                    && matches!(state.status, NodeStatus::Active | NodeStatus::Completed)
                {
                    state
                        .thinking_session_id
                        .as_deref()
                        .and_then(|thinking_session_id| {
                            parse_thinking_trace(context, thinking_session_id)
                        })
                        .map(Value::Array)
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                node_object.insert("thinking_trace".to_string(), thinking_trace);
            }
            Ok(node_json)
        })
        .collect()
}

fn pipeline_node_provenance_json(node: &PipelineNode, state: &NodeState) -> Value {
    let can_reconnect = node_can_reconnect(node, state);
    let inspectable =
        can_reconnect || state.thinking_session_id.is_some() || state.tool_provenance.is_some();

    json!({
        "agent": node_agent_name(node),
        "inspectable": inspectable,
        "detail": {
            "sessionId": state.acp_session_id.as_deref(),
            "thinkingSessionId": state.thinking_session_id.as_deref(),
            "canReconnect": can_reconnect,
            "adoption": node_adoption_state_json(node),
        }
    })
}

fn node_adoption_state_json(node: &PipelineNode) -> Value {
    let adopt = node.adopt();
    json!({
        "role": normalize_adoption_value(adopt.and_then(|config| config.role.as_deref())),
        "color": normalize_adoption_value(adopt.and_then(|config| config.color.as_deref())),
        "perspective": normalize_adoption_value(adopt.and_then(|config| config.perspective.as_deref())),
    })
}

fn normalize_adoption_value(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "none".to_string())
}

fn node_agent_name(node: &PipelineNode) -> Option<&str> {
    match node {
        PipelineNode::Workflow { acp_client, .. } => acp_client.as_deref(),
        PipelineNode::Prompt { agent, .. } => Some(agent.as_str()),
        PipelineNode::SequentialThinking { agent, .. } => Some(agent.as_str()),
        PipelineNode::Tool { agent, .. } => agent.as_deref(),
    }
}

fn node_can_reconnect(node: &PipelineNode, state: &NodeState) -> bool {
    !matches!(node, PipelineNode::Tool { .. }) && state.acp_session_id.is_some()
}

fn add_pipeline_node_view_fields(node_json: &mut Value, node: &PipelineNode) {
    let additional_fields = match node {
        PipelineNode::Workflow {
            workflow_id,
            acp_client,
            prompt,
            timeout,
            agent_config,
            ..
        } => json!({
            "workflowId": workflow_id,
            "acpClient": acp_client,
            "prompt": prompt,
            "timeout": timeout,
            "agentConfig": agent_config,
        }),
        PipelineNode::Prompt {
            prompt,
            agent,
            tool_mode,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            cwd,
            timeout,
            agent_config,
            ..
        } => json!({
            "prompt": prompt,
            "agent": agent,
            "toolMode": tool_mode,
            "model": model,
            "thoughtLevel": thought_level,
            "allowedTools": allowed_tools,
            "maxTurns": max_turns,
            "cwd": cwd,
            "timeout": timeout,
            "agentConfig": agent_config,
        }),
        PipelineNode::SequentialThinking {
            topic,
            total_thoughts,
            agent,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            cwd,
            timeout,
            agent_config,
            ..
        } => json!({
            "topic": topic,
            "agent": agent,
            "totalThoughts": total_thoughts,
            "model": model,
            "thoughtLevel": thought_level,
            "allowedTools": allowed_tools,
            "maxTurns": max_turns,
            "cwd": cwd,
            "timeout": timeout,
            "agentConfig": agent_config,
        }),
        PipelineNode::Tool {
            tool_name,
            tool_args,
            agent,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            cwd,
            timeout,
            agent_config,
            ..
        } => json!({
            "agent": agent,
            "toolName": tool_name,
            "toolArgs": tool_args,
            "model": model,
            "thoughtLevel": thought_level,
            "allowedTools": allowed_tools,
            "maxTurns": max_turns,
            "cwd": cwd,
            "timeout": timeout,
            "agentConfig": agent_config,
        }),
    };

    if let (Some(node_object), Some(additional_object)) =
        (node_json.as_object_mut(), additional_fields.as_object())
    {
        node_object.extend(additional_object.clone());
    }
}

#[cfg(test)]
#[path = "view_nodes_tests.rs"]
mod tests;
