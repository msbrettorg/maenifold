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
                if let Some(retry_after) = &state.retry_after {
                    node_object.insert("retryAfter".to_string(), json!(retry_after));
                }
                if let Some(retry_count) = state.retry_count {
                    node_object.insert("retryCount".to_string(), json!(retry_count));
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
                        .and_then(|thinking_session_id| parse_thinking_trace(context, thinking_session_id))
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use crate::util::json::OutputFormat;

    use super::*;
    use crate::workflow::pipeline::test_support::pipeline_test_context;
    use crate::workflow::pipeline::{PipelineEdge, ToolProvenance, types::EdgeType};

    #[test]
    fn pipeline_nodes_json_keeps_local_session_id_and_surfaces_provenance_links() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::SequentialThinking {
                    id: "think".to_string(),
                    topic: "Topic".to_string(),
                    total_thoughts: 3,
                    analysis_type: None,
                    agent: "agent-think".to_string(),
                    model: None,
                    thought_level: None,
                    allowed_tools: None,
                    max_turns: None,
                    timeout: None,
                    agent_config: None,
                },
                PipelineNode::Tool {
                    id: "tool".to_string(),
                    tool_name: "example.echo".to_string(),
                    tool_args: None,
                    agent: None,
                    model: None,
                    thought_level: None,
                    allowed_tools: None,
                    max_turns: None,
                    timeout: None,
                    agent_config: None,
                },
            ],
            edges: vec![PipelineEdge {
                source: "think".to_string(),
                target: "tool".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            }],
        };
        let node_states = HashMap::from([
            (
                "think".to_string(),
                NodeState {
                    session_id: "workflow-think-local".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 1,
                    acp_session_id: Some("acp-think-remote".to_string()),
                    thinking_session_id: Some("thinking-remote".to_string()),
                    tool_provenance: None,
                    output: None,
                    failure_reason: None,
                    retry_after: None,
                    retry_count: None,
                },
            ),
            (
                "tool".to_string(),
                NodeState {
                    session_id: "workflow-tool-local".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    acp_session_id: None,
                    thinking_session_id: None,
                    tool_provenance: Some(ToolProvenance {
                        tool_name: "example.echo".to_string(),
                        resolved_args: Some(json!({ "value": 1 })),
                        response_summary: "{\"value\":1}".to_string(),
                    }),
                    output: Some("{\"value\":1}".to_string()),
                    failure_reason: None,
                    retry_after: None,
                    retry_count: None,
                },
            ),
        ]);

        let nodes = pipeline_nodes_json(&context, &definition, &node_states, "")
            .expect("node view should serialize");

        let think = nodes
            .iter()
            .find(|node| node["nodeId"] == "think")
            .expect("think node");
        assert_eq!(think["sessionId"], "workflow-think-local");
        assert_eq!(think["acpSessionId"], "acp-think-remote");
        assert_eq!(think["thinkingSessionId"], "thinking-remote");

        let tool = nodes
            .iter()
            .find(|node| node["nodeId"] == "tool")
            .expect("tool node");
        assert_eq!(tool["sessionId"], "workflow-tool-local");
        assert_eq!(tool["toolProvenance"]["toolName"], "example.echo");
        assert_eq!(tool["toolProvenance"]["resolvedArgs"]["value"], 1);
        assert_eq!(tool["toolProvenance"]["responseSummary"], "{\"value\":1}");
    }
}
