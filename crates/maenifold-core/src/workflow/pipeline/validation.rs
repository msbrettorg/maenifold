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
        if !node_ids.contains(edge.source.as_str()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline edge references unknown source node '{}'",
                edge.source
            )));
        }

        if !node_ids.contains(edge.target.as_str()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline edge references unknown target node '{}'",
                edge.target
            )));
        }

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
