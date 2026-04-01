use crate::error::Result;
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

pub(super) fn edge_type_name(edge_type: &EdgeType) -> &'static str {
    match edge_type {
        EdgeType::Success => "success",
        EdgeType::Always => "always",
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
