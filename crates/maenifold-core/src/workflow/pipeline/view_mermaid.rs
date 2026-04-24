use crate::error::{MaenifoldError, Result};
use std::collections::HashMap;

use super::view::node_state_for;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineNode};

pub(super) fn render_pipeline_mermaid(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Result<String> {
    let mut lines = vec!["graph LR".to_string()];

    for node in &definition.nodes {
        let state = node_state_for(node_states, node.id())?;
        lines.push(render_pipeline_mermaid_node(node, &state.status)?);
    }
    for edge in &definition.edges {
        let line = match &edge.edge_type {
            EdgeType::Always => format!("  {} -.-> {}", edge.source, edge.target),
            EdgeType::Success => format!("  {} --> {}", edge.source, edge.target),
            EdgeType::Conditional { path, equals } => {
                let value_text = match equals {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let escaped_path = escape_mermaid_edge_label(path);
                let escaped_value_text = escape_mermaid_edge_label(&value_text);
                format!(
                    "  {} -->|?{}={}| {}",
                    edge.source, escaped_path, escaped_value_text, edge.target
                )
            }
        };
        lines.push(line);
    }

    Ok(lines.join("\n"))
}

pub(super) fn edge_type_name(edge_type: &EdgeType) -> &'static str {
    match edge_type {
        EdgeType::Success => "success",
        EdgeType::Always => "always",
        EdgeType::Conditional { .. } => "conditional",
    }
}

fn render_pipeline_mermaid_node(node: &PipelineNode, status: &NodeStatus) -> Result<String> {
    let label = mermaid_node_label(node, status)?;
    let wrapped_label = match node {
        PipelineNode::Workflow { .. } => format!(r#"[\"{label}\"]"#),
        PipelineNode::Prompt { .. } => format!(r#"(\"{label}\")"#),
        PipelineNode::SequentialThinking { .. } => format!(r#"{{{{\"{label}\"}}}}"#),
        PipelineNode::Tool { .. } => format!(">{label}]"),
    };

    Ok(format!("  {}{}", node.id(), wrapped_label))
}

fn mermaid_node_label(node: &PipelineNode, status: &NodeStatus) -> Result<String> {
    if let Some(display_label) = node.display_label() {
        return Ok(format!("{} {}", status_emoji(status), display_label));
    }

    let node_label = match node {
        PipelineNode::Workflow { .. } => {
            let workflow_id = node.workflow_id().ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Workflow pipeline node '{}' is missing a workflow id",
                    node.id()
                ))
            })?;
            format!("wf: {workflow_id}")
        }
        PipelineNode::Prompt { .. } => {
            format!(
                "prompt: {}",
                truncated_prompt_label(node.prompt().unwrap_or_default())
            )
        }
        PipelineNode::SequentialThinking { topic, .. } => format!("think: {topic}"),
        PipelineNode::Tool { tool_name, .. } => format!("tool: {tool_name}"),
    };

    Ok(format!("{} {}", status_emoji(status), node_label))
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
        NodeStatus::Retrying => "⏱️",
    }
}

fn escape_mermaid_edge_label(value: &str) -> String {
    value
        .replace('|', "&#124;")
        .replace('{', "&#123;")
        .replace('}', "&#125;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
        .replace('(', "&#40;")
        .replace(')', "&#41;")
}

#[cfg(test)]
mod tests {
    use super::render_pipeline_mermaid;
    use crate::workflow::pipeline::{
        EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    };
    use std::collections::HashMap;

    #[test]
    fn conditional_edge_label_escapes_mermaid_special_characters() {
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "source".to_string(),
                    display_label: None,
                    workflow_id: "wf-source".to_string(),
                    acp_client: None,
                    prompt: None,
                    cwd: None,
                    timeout: None,
                    agent_config: None,
                    adopt: None,
                    requires_role: None,
                    retry_policy: None,
                },
                PipelineNode::Workflow {
                    id: "target".to_string(),
                    display_label: None,
                    workflow_id: "wf-target".to_string(),
                    acp_client: None,
                    prompt: None,
                    cwd: None,
                    timeout: None,
                    agent_config: None,
                    adopt: None,
                    requires_role: None,
                    retry_policy: None,
                },
            ],
            edges: vec![PipelineEdge {
                source: "source".to_string(),
                target: "target".to_string(),
                edge_type: EdgeType::Conditional {
                    path: "a|b".to_string(),
                    equals: serde_json::Value::String("c|d".to_string()),
                },
                mapping: None,
            }],
            inputs: None,
            template_id: None,
        };

        let node_states = HashMap::from([
            ("source".to_string(), completed_state("source")),
            ("target".to_string(), completed_state("target")),
        ]);

        let mermaid = render_pipeline_mermaid(&definition, &node_states)
            .expect("mermaid should render for conditional edge");
        assert!(mermaid.contains("  source -->|?a&#124;b=c&#124;d| target"));
        assert!(!mermaid.contains("?a|b=c|d"));
    }

    fn completed_state(session_id: &str) -> NodeState {
        NodeState {
            session_id: session_id.to_string(),
            status: NodeStatus::Completed,
            attempts: 1,
            started_at: None,
            completed_at: None,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            input_tokens: None,
            output_tokens: None,
            model: None,
            output: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        }
    }
}
