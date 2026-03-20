use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use std::collections::HashMap;

use super::session_io::{derive_pipeline_node_states, frontmatter_value, read_pipeline_session};
#[cfg(test)]
use super::test_support;
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, compute_pipeline_status, compute_ready_set,
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
            let state = node_states.get(&node.id).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{}'",
                    node.id
                ))
            })?;
            Ok((node.id.clone(), state.status == expected_status))
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

    Ok(json!({
        "pipelineId": session_id,
        "status": status,
        "nodes": nodes,
        "edges": edges,
        "ready": ready,
        "active": active,
        "completed": completed,
        "mermaid": mermaid,
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
            let state = node_states.get(&node.id).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{}'",
                    node.id
                ))
            })?;
            Ok(json!({
                "nodeId": node.id,
                "workflowId": node.workflow_id,
                "sessionId": state.session_id,
                "status": state.status,
                "attempts": state.attempts,
            }))
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
        let state = node_states.get(&node.id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{}'",
                node.id
            ))
        })?;
        lines.push(format!(
            "  {}[\"{} {}\"]",
            node.id,
            status_emoji(&state.status),
            node.id
        ));
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
        assert!(mermaid.contains("root[\"✅ root\"]"));
        assert!(mermaid.contains("cleanup[\"🔄 cleanup\"]"));
        assert!(mermaid.contains("child[\"⏳ child\"]"));
        assert!(mermaid.contains("root --> child"));
        assert!(mermaid.contains("root -.-> cleanup"));
    }
}
