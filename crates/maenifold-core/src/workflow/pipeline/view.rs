use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use std::collections::HashMap;

use super::session_io::{derive_pipeline_node_states, frontmatter_value, read_pipeline_session};
use super::supervisor::supervisor_status;
#[cfg(test)]
use super::test_support;
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineNode, compute_pipeline_status,
    compute_ready_set,
};

pub(super) fn view_pipeline_session(context: &ToolContext, session_id: &str) -> Result<String> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let result = build_pipeline_view_result(session_id, &definition, &node_states)?;

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

pub(super) fn node_ids_with_status(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    expected_status: NodeStatus,
) -> Result<Vec<String>> {
    definition
        .nodes
        .iter()
        .map(|node| {
            let state = node_states.get(node.id()).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{}'",
                    node.id()
                ))
            })?;
            Ok((node.id().to_string(), state.status == expected_status))
        })
        .filter_map(|entry| match entry {
            Ok((node_id, true)) => Some(Ok(node_id)),
            Ok((_, false)) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

pub(super) fn build_pipeline_view_result(
    session_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Result<Value> {
    let ready = compute_ready_set(definition, node_states);
    let status = compute_pipeline_status(node_states);
    let active = node_ids_with_status(definition, node_states, NodeStatus::Active)?;
    let completed = node_ids_with_status(definition, node_states, NodeStatus::Completed)?;
    let nodes = pipeline_nodes_json(definition, node_states)?;
    let edges = pipeline_edges_json(definition);
    let mermaid = render_pipeline_mermaid(definition, node_states)?;
    let running = supervisor_status(session_id).is_some();

    Ok(json!({
        "pipelineId": session_id,
        "status": status,
        "nodes": nodes,
        "edges": edges,
        "ready": ready,
        "active": active,
        "completed": completed,
        "mermaid": mermaid,
        "running": running,
    }))
}

fn pipeline_nodes_json(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Result<Vec<Value>> {
    definition
        .nodes
        .iter()
        .map(|node| {
            let state = node_states.get(node.id()).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{}'",
                    node.id()
                ))
            })?;
            let mut node_json = json!({
                "nodeId": node.id(),
                "sessionId": state.session_id,
                "status": state.status,
                "attempts": state.attempts,
                "output": state.output,
                "type": node.node_type_name(),
            });
            add_pipeline_node_view_fields(&mut node_json, node);
            Ok(node_json)
        })
        .collect()
}

fn pipeline_edges_json(definition: &PipelineDefinition) -> Vec<Value> {
    definition
        .edges
        .iter()
        .map(|edge| {
            json!({
                "source": edge.source,
                "target": edge.target,
                "type": edge_type_name(&edge.edge_type),
            })
        })
        .collect()
}

fn render_pipeline_mermaid(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Result<String> {
    let mut lines = vec!["graph LR".to_string()];

    for node in &definition.nodes {
        let state = node_states.get(node.id()).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{}'",
                node.id()
            ))
        })?;
        lines.push(render_pipeline_mermaid_node(node, &state.status));
    }

    for edge in &definition.edges {
        let arrow = match edge.edge_type {
            EdgeType::Always => "-.->",
            EdgeType::Success => "-->",
        };
        lines.push(format!("  {} {} {}", edge.source, arrow, edge.target));
    }

    Ok(lines.join("\n"))
}

fn add_pipeline_node_view_fields(node_json: &mut Value, node: &PipelineNode) {
    let additional_fields = match node {
        PipelineNode::Workflow {
            workflow_id,
            acp_client,
            prompt,
            ..
        } => json!({
            "workflowId": workflow_id,
            "agent": acp_client,
            "prompt": prompt,
        }),
        PipelineNode::Prompt { prompt, agent, .. } => json!({
            "prompt": prompt,
            "agent": agent,
        }),
        PipelineNode::SequentialThinking {
            topic,
            total_thoughts,
            analysis_type,
            agent,
            ..
        } => json!({
            "topic": topic,
            "agent": agent,
            "totalThoughts": total_thoughts,
            "analysisType": analysis_type,
        }),
        PipelineNode::Tool {
            tool_name,
            tool_args,
            agent,
            ..
        } => json!({
            "agent": agent,
            "toolName": tool_name,
            "toolArgs": tool_args,
        }),
    };

    if let (Some(node_object), Some(additional_object)) =
        (node_json.as_object_mut(), additional_fields.as_object())
    {
        node_object.extend(additional_object.clone());
    }
}

fn render_pipeline_mermaid_node(node: &PipelineNode, status: &NodeStatus) -> String {
    let label = mermaid_node_label(node, status);
    let wrapped_label = match node {
        PipelineNode::Workflow { .. } => format!("[\"{label}\"]"),
        PipelineNode::Prompt { .. } => format!("(\"{label}\")"),
        PipelineNode::SequentialThinking { .. } => format!("{{{{\"{label}\"}}}}"),
        PipelineNode::Tool { .. } => format!(">{label}]"),
    };

    format!("  {}{}", node.id(), wrapped_label)
}

fn mermaid_node_label(node: &PipelineNode, status: &NodeStatus) -> String {
    let node_label = match node {
        PipelineNode::Workflow { .. } => format!(
            "wf: {}",
            node.workflow_id()
                .expect("workflow nodes always have workflow ids")
        ),
        PipelineNode::Prompt { .. } => {
            format!(
                "prompt: {}",
                truncated_prompt_label(node.prompt().unwrap_or_default())
            )
        }
        PipelineNode::SequentialThinking { topic, .. } => format!("think: {topic}"),
        PipelineNode::Tool { tool_name, .. } => format!("tool: {tool_name}"),
    };

    format!("{} {}", status_emoji(status), node_label)
}

fn truncated_prompt_label(prompt: &str) -> String {
    let preview: String = prompt.chars().take(40).collect();
    format!("{preview}...")
}

fn status_emoji(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "⏳",
        NodeStatus::Ready => "🟡",
        NodeStatus::Active => "🔄",
        NodeStatus::Completed => "✅",
        NodeStatus::Abandoned => "❌",
        NodeStatus::Stuck => "🚫",
    }
}

fn edge_type_name(edge_type: &EdgeType) -> &'static str {
    match edge_type {
        EdgeType::Success => "success",
        EdgeType::Always => "always",
    }
}

#[cfg(test)]
mod tests {
    use super::super::handler::pipeline;
    use super::*;
    use serde_json::Value;
    use serial_test::serial;
    use std::collections::HashMap;

    use super::test_support::*;

    #[test]
    #[serial(config)]
    fn pipeline_view_returns_structured_json_and_mermaid() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600000";
        let definition = seed_pipeline_view_session(&context, session_id, "pipeline");

        let output = pipeline(&context, &json!({ "sessionId": session_id, "view": true }))
            .expect("view pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("pipelineId").and_then(Value::as_str),
            Some(session_id)
        );
        assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("child".to_string())])
        );
        assert_eq!(
            data.get("active").and_then(Value::as_array),
            Some(&vec![Value::String("cleanup".to_string())])
        );
        assert_eq!(
            data.get("completed").and_then(Value::as_array),
            Some(&vec![Value::String("root".to_string())])
        );

        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        assert_eq!(nodes.len(), definition.nodes.len());
        assert_eq!(nodes[0]["nodeId"], "root");
        assert_eq!(nodes[0]["type"], "workflow");
        assert_eq!(nodes[0]["workflowId"], "test-workflow-a");
        assert_eq!(nodes[0]["sessionId"], "workflow-1735689600000-root");
        assert_eq!(nodes[0]["status"], "completed");
        assert_eq!(nodes[0]["attempts"], 1);
        assert_eq!(nodes[1]["status"], "active");
        assert_eq!(nodes[2]["status"], "pending");

        let edges = data
            .get("edges")
            .and_then(Value::as_array)
            .expect("edges array");
        assert_eq!(edges.len(), definition.edges.len());
        assert_eq!(edges[0]["type"], "success");
        assert_eq!(edges[1]["type"], "always");

        let mermaid = data
            .get("mermaid")
            .and_then(Value::as_str)
            .expect("mermaid string");
        assert!(mermaid.starts_with("graph LR\n"));
        assert!(mermaid.contains("root[\"✅ wf: test-workflow-a\"]"));
        assert!(mermaid.contains("cleanup[\"🔄 wf: test-workflow-b\"]"));
        assert!(mermaid.contains("child[\"⏳ wf: test-workflow-b\"]"));
        assert!(mermaid.contains("root --> child"));
        assert!(mermaid.contains("root -.-> cleanup"));
        assert_eq!(data.get("running").and_then(Value::as_bool), Some(false));
        assert!(data.get("pid").is_none() || data.get("pid") == Some(&Value::Null));
        assert!(data.get("attempts").is_none() || data.get("attempts") == Some(&Value::Null));
    }

    #[test]
    fn pipeline_view_renders_mixed_node_types() {
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "workflow-node".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Prompt {
                    id: "prompt-node".to_string(),
                    prompt: "Summarize the pipeline status and propose next steps for the operator"
                        .to_string(),
                    agent: "agent-prompt".to_string(),
                    tool_mode: "default".to_string(),
                },
                PipelineNode::SequentialThinking {
                    id: "thinking-node".to_string(),
                    topic: "pipeline triage".to_string(),
                    total_thoughts: 3,
                    analysis_type: Some("diagnostic".to_string()),
                    agent: "agent-think".to_string(),
                },
                PipelineNode::Tool {
                    id: "tool-node".to_string(),
                    tool_name: "ReadMemory".to_string(),
                    tool_args: Some(json!({"identifier": "memory://build/test"})),
                    agent: None,
                },
            ],
            edges: vec![
                super::super::PipelineEdge {
                    source: "workflow-node".to_string(),
                    target: "prompt-node".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "prompt-node".to_string(),
                    target: "thinking-node".to_string(),
                    edge_type: EdgeType::Always,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "thinking-node".to_string(),
                    target: "tool-node".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
            ],
        };
        let node_states = HashMap::from([
            (
                "workflow-node".to_string(),
                NodeState {
                    session_id: "workflow-session".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
                },
            ),
            (
                "prompt-node".to_string(),
                NodeState {
                    session_id: "prompt-session".to_string(),
                    status: NodeStatus::Active,
                    attempts: 2,
                    output: Some("prompt output".to_string()),
                },
            ),
            (
                "thinking-node".to_string(),
                NodeState {
                    session_id: "thinking-session".to_string(),
                    status: NodeStatus::Ready,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "tool-node".to_string(),
                NodeState {
                    session_id: "tool-session".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);

        let result = build_pipeline_view_result("pipeline-mixed-nodes", &definition, &node_states)
            .expect("build pipeline view");
        let nodes = result
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");

        assert_eq!(nodes[0]["type"], "workflow");
        assert_eq!(nodes[0]["workflowId"], "test-workflow-a");
        assert_eq!(nodes[0]["agent"], Value::Null);
        assert_eq!(nodes[0]["prompt"], Value::Null);
        assert!(nodes[0].get("topic").is_none());
        assert_eq!(nodes[0]["status"], "completed");
        assert_eq!(nodes[0]["attempts"], 1);
        assert_eq!(nodes[0]["output"], Value::Null);

        assert_eq!(nodes[1]["type"], "prompt");
        assert_eq!(nodes[1]["agent"], "agent-prompt");
        assert_eq!(
            nodes[1]["prompt"],
            "Summarize the pipeline status and propose next steps for the operator"
        );
        assert!(nodes[1].get("workflowId").is_none());
        assert!(nodes[1].get("topic").is_none());
        assert_eq!(nodes[1]["status"], "active");
        assert_eq!(nodes[1]["attempts"], 2);
        assert_eq!(nodes[1]["output"], "prompt output");

        assert_eq!(nodes[2]["type"], "sequentialThinking");
        assert_eq!(nodes[2]["agent"], "agent-think");
        assert_eq!(nodes[2]["topic"], "pipeline triage");
        assert_eq!(nodes[2]["totalThoughts"], 3);
        assert_eq!(nodes[2]["analysisType"], "diagnostic");
        assert!(nodes[2].get("workflowId").is_none());
        assert_eq!(nodes[2]["status"], "ready");
        assert_eq!(nodes[2]["attempts"], 0);
        assert_eq!(nodes[2]["output"], Value::Null);

        assert_eq!(nodes[3]["type"], "tool");
        assert_eq!(nodes[3]["agent"], Value::Null);
        assert_eq!(nodes[3]["toolName"], "ReadMemory");
        assert_eq!(
            nodes[3]["toolArgs"],
            json!({"identifier": "memory://build/test"})
        );
        assert!(nodes[3].get("workflowId").is_none());
        assert_eq!(nodes[3]["status"], "pending");
        assert_eq!(nodes[3]["attempts"], 0);
        assert_eq!(nodes[3]["output"], Value::Null);

        let mermaid = result
            .get("mermaid")
            .and_then(Value::as_str)
            .expect("mermaid string");
        assert!(mermaid.contains("workflow-node[\"✅ wf: test-workflow-a\"]"));
        assert!(
            mermaid.contains(
                "prompt-node(\"🔄 prompt: Summarize the pipeline status and propos...\")"
            )
        );
        assert!(mermaid.contains("thinking-node{{\"🟡 think: pipeline triage\"}}"));
        assert!(mermaid.contains("tool-node>⏳ tool: ReadMemory]"));
        assert!(mermaid.contains("workflow-node --> prompt-node"));
        assert!(mermaid.contains("prompt-node -.-> thinking-node"));
        assert!(mermaid.contains("thinking-node --> tool-node"));
    }
}
