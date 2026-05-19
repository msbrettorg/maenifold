use crate::error::{MaenifoldError, Result};
use petgraph::algo::is_cyclic_directed;
use std::collections::{HashMap, HashSet};

use super::{EdgeDataMode, PipelineDefinition, PipelineNode, validate_pipeline_inputs};

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
    let mut session_refs = HashMap::new();
    for node in &definition.nodes {
        if !node_ids.insert(node.id()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline contains duplicate node ID '{}'",
                node.id()
            )));
        }

        validate_unique_session_ref(node, &mut session_refs)?;
        validate_node_semantics(node)?;
    }

    let mut edge_pairs = HashSet::with_capacity(definition.edges.len());
    for edge in &definition.edges {
        validate_edge_nodes_exist(edge, &node_ids)?;
        validate_conditional_edge(edge)?;
        validate_edge_data_mode(edge)?;

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

    if let Some(schema) = &definition.input_schema {
        let inputs = definition.inputs.clone().unwrap_or_default();
        validate_pipeline_inputs(schema, &inputs)?;
    }

    Ok(())
}

fn validate_unique_session_ref<'a>(
    node: &'a PipelineNode,
    session_refs: &mut HashMap<(&'a str, &'a str), &'a str>,
) -> Result<()> {
    let PipelineNode::Prompt {
        id,
        session_ref: Some(session_ref),
        ..
    } = node
    else {
        return Ok(());
    };

    let key = (session_ref.agent.as_str(), session_ref.session_id.as_str());
    if let Some(existing_node_id) = session_refs.insert(key, id.as_str()) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Nodes '{}' and '{}' share the same sessionRef (agent: '{}', sessionId: '{}'). Each session can only be referenced by one node.",
            existing_node_id, id, session_ref.agent, session_ref.session_id
        )));
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

fn validate_edge_data_mode(edge: &super::PipelineEdge) -> Result<()> {
    if edge.mode == EdgeDataMode::None
        && edge
            .mapping
            .as_ref()
            .is_some_and(|mapping| !mapping.is_empty())
    {
        return Err(MaenifoldError::Message(format!(
            "None mode forbids mapping on edge '{} -> {}'",
            edge.source, edge.target
        )));
    }

    if edge.mode == EdgeDataMode::Binding {
        return Err(MaenifoldError::Message(format!(
            "Binding edge requires declared schemas on both endpoints for edge '{} -> {}'",
            edge.source, edge.target
        )));
    }

    Ok(())
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
