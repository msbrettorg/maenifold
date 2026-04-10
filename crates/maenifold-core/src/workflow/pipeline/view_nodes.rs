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
            let display_session_id = if matches!(node, PipelineNode::SequentialThinking { .. }) {
                state
                    .thinking_session_id
                    .as_deref()
                    .unwrap_or(&state.session_id)
            } else {
                &state.session_id
            };
            let mut node_json = json!({
                "nodeId": node.id(),
                "sessionId": display_session_id,
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
                if let Some(retry_after) = &state.retry_after {
                    node_object.insert("retryAfter".to_string(), json!(retry_after));
                }
                if let Some(retry_count) = state.retry_count {
                    node_object.insert("retryCount".to_string(), json!(retry_count));
                }
                if let Some(failure_reason) = &state.failure_reason {
                    node_object.insert("failureReason".to_string(), json!(failure_reason));
                }
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
                    parse_thinking_trace(context, display_session_id)
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

fn add_pipeline_node_view_fields(node_json: &mut Value, node: &PipelineNode) {
    let additional_fields = match node {
        PipelineNode::Workflow {
            workflow_id,
            acp_client,
            prompt,
            timeout,
            ..
        } => json!({
            "workflowId": workflow_id,
            "agent": acp_client,
            "prompt": prompt,
            "timeout": timeout,
        }),
        PipelineNode::Prompt {
            prompt,
            agent,
            tool_mode,
            model,
            thought_level,
            allowed_tools,
            max_turns,
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
            "timeout": timeout,
            "agentConfig": agent_config,
        }),
        PipelineNode::SequentialThinking {
            topic,
            total_thoughts,
            analysis_type,
            agent,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            timeout,
            agent_config,
            ..
        } => json!({
            "topic": topic,
            "agent": agent,
            "totalThoughts": total_thoughts,
            "analysisType": analysis_type,
            "model": model,
            "thoughtLevel": thought_level,
            "allowedTools": allowed_tools,
            "maxTurns": max_turns,
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
