use crate::error::{MaenifoldError, Result};
use petgraph::algo::is_cyclic_directed;
use std::collections::HashSet;

use super::{PipelineDefinition, PipelineNode};

fn validate_non_empty_node_field(node_id: &str, field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline node '{node_id}' field '{field}' must not be empty"
        )));
    }

    Ok(())
}

fn validate_node_semantics(node: &PipelineNode) -> Result<()> {
    match node {
        PipelineNode::Workflow {
            id,
            workflow_id,
            acp_client,
            ..
        } => {
            validate_non_empty_node_field(id, "workflowId", workflow_id)?;
            validate_non_empty_node_field(
                id,
                "acpClient",
                acp_client.as_deref().ok_or_else(|| {
                    MaenifoldError::InvalidArgument(format!(
                        "Pipeline node '{id}' field 'acpClient' is required"
                    ))
                })?,
            )?;
            Ok(())
        }
        PipelineNode::Prompt {
            id, prompt, agent, ..
        } => {
            validate_non_empty_node_field(id, "prompt", prompt)?;
            validate_non_empty_node_field(id, "agent", agent)
        }
        PipelineNode::SequentialThinking {
            id,
            topic,
            total_thoughts,
            agent,
            ..
        } => {
            validate_non_empty_node_field(id, "topic", topic)?;
            validate_non_empty_node_field(id, "agent", agent)?;

            if *total_thoughts == 0 {
                return Err(MaenifoldError::InvalidArgument(format!(
                    "Pipeline node '{id}' field 'totalThoughts' must be greater than 0"
                )));
            }

            Ok(())
        }
        PipelineNode::Tool { id, tool_name, .. } => {
            validate_non_empty_node_field(id, "toolName", tool_name)
        }
    }
}

pub fn validate_pipeline(definition: &PipelineDefinition) -> Result<()> {
    if definition.nodes.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "Pipeline definition must contain at least one node".to_string(),
        ));
    }

    let mut node_ids = HashSet::with_capacity(definition.nodes.len());
    for node in &definition.nodes {
        if !node_ids.insert(node.id()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline contains duplicate node ID '{}'",
                node.id()
            )));
        }

        validate_node_semantics(node)?;
    }

    let mut edge_pairs = HashSet::with_capacity(definition.edges.len());
    for edge in &definition.edges {
        validate_edge_nodes_exist(edge, &node_ids)?;
        validate_conditional_edge(edge)?;

        if !edge_pairs.insert((edge.source.as_str(), edge.target.as_str())) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline contains duplicate edge '{} -> {}'",
                edge.source, edge.target
            )));
        }
    }

    if is_cyclic_directed(&definition.build_graph()) {
        return Err(MaenifoldError::InvalidArgument(
            "Pipeline definition contains a cycle".to_string(),
        ));
    }

    Ok(())
}

fn validate_edge_nodes_exist(edge: &super::PipelineEdge, node_ids: &HashSet<&str>) -> Result<()> {
    if !node_ids.contains(edge.source.as_str()) {
        let reason = match edge.edge_type {
            super::EdgeType::Conditional { .. } => "Conditional edge",
            _ => "Pipeline edge",
        };
        return Err(MaenifoldError::InvalidArgument(format!(
            "{reason} '{} -> {}' references unknown source node '{}'",
            edge.source, edge.target, edge.source
        )));
    }

    if !node_ids.contains(edge.target.as_str()) {
        let reason = match edge.edge_type {
            super::EdgeType::Conditional { .. } => "Conditional edge",
            _ => "Pipeline edge",
        };
        return Err(MaenifoldError::InvalidArgument(format!(
            "{reason} '{} -> {}' references unknown target node '{}'",
            edge.source, edge.target, edge.target
        )));
    }

    Ok(())
}

fn validate_conditional_edge(edge: &super::PipelineEdge) -> Result<()> {
    if let super::EdgeType::Conditional { path, .. } = &edge.edge_type
        && path.trim().is_empty()
    {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Conditional edge '{} -> {}' field 'path' must not be empty",
            edge.source, edge.target
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::pipeline::{EdgeType, PipelineDefinition, PipelineEdge, PipelineNode};

    fn make_node(id: &str) -> PipelineNode {
        PipelineNode::SequentialThinking {
            id: id.to_string(),
            display_label: None,
            topic: "topic".to_string(),
            total_thoughts: 1,
            analysis_type: None,
            agent: "agent".to_string(),
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            cwd: None,
            timeout: None,
            agent_config: None,
            retry_policy: None,
        }
    }

    fn base_definition() -> PipelineDefinition {
        PipelineDefinition {
            nodes: vec![make_node("source"), make_node("target")],
            edges: vec![],
            inputs: None,
            template_id: None,
        }
    }

    #[test]
    fn validate_pipeline_accepts_conditional_edge_with_non_empty_path() {
        let mut definition = base_definition();
        definition.edges.push(PipelineEdge {
            source: "source".to_string(),
            target: "target".to_string(),
            edge_type: EdgeType::Conditional {
                path: "result.verdict".to_string(),
                equals: serde_json::json!("PASS"),
            },
            mapping: None,
        });

        let result = validate_pipeline(&definition);
        assert!(result.is_ok(), "expected conditional edge to validate");
    }

    #[test]
    fn validate_pipeline_rejects_conditional_edge_with_empty_path() {
        let mut definition = base_definition();
        definition.edges.push(PipelineEdge {
            source: "source".to_string(),
            target: "target".to_string(),
            edge_type: EdgeType::Conditional {
                path: "   ".to_string(),
                equals: serde_json::json!("PASS"),
            },
            mapping: None,
        });

        let error = validate_pipeline(&definition).expect_err("expected validation error");
        assert_eq!(
            error.to_string(),
            "Conditional edge 'source -> target' field 'path' must not be empty"
        );
    }

    #[test]
    fn validate_pipeline_rejects_conditional_edge_with_unknown_source_node() {
        let mut definition = base_definition();
        definition.edges.push(PipelineEdge {
            source: "missing".to_string(),
            target: "target".to_string(),
            edge_type: EdgeType::Conditional {
                path: "verdict".to_string(),
                equals: serde_json::json!("PASS"),
            },
            mapping: None,
        });

        let error = validate_pipeline(&definition).expect_err("expected validation error");
        assert_eq!(
            error.to_string(),
            "Conditional edge 'missing -> target' references unknown source node 'missing'"
        );
    }
}
