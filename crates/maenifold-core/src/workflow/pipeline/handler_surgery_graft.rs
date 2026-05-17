use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::write_markdown;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use petgraph::Direction::Outgoing;
use petgraph::algo::is_cyclic_directed;
use serde_yml::Value as YamlValue;
use std::collections::HashSet;

use super::super::handler_validation::{validate_workflow_node_definitions, workflow_node_file};
use super::super::resource_notifications::emit_pipeline_resource_updated;
use super::super::session_io::{
    allocate_node_session_id, derive_pipeline_node_states, frontmatter_value,
    read_pipeline_session, write_workflow_node_session,
};
use super::super::view::build_pipeline_view_result;
use super::super::{NodeState, NodeStatus, PipelineDefinition, validate_pipeline};
use super::GraftRequest;

#[allow(clippy::too_many_lines)]
pub(super) fn graft_pipeline(
    context: &ToolContext,
    session_id: &str,
    graft: GraftRequest,
) -> Result<String> {
    AssetManager::initialize_assets(&context.config)?;
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let pipeline_status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .unwrap_or_else(|| super::super::compute_pipeline_status(&node_states))
        .to_string();

    if super::super::has_remaining_work(&node_states) {
        return Err(MaenifoldError::Message(
            "Pipeline graft rejected: running pipelines must be completed or cancelled before graft."
                .to_string(),
        ));
    }

    let merged_definition = merge_graft_definition(&definition, &graft)?;

    let mut new_node_files = Vec::with_capacity(graft.nodes.len());
    for node in &graft.nodes {
        let workflow_session_id = allocate_node_session_id(context, node.id())?;
        let state = NodeState {
            session_id: workflow_session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
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
        };
        node_states.insert(node.id().to_string(), state);
        new_node_files.push(workflow_node_file(node, workflow_session_id));
    }

    {
        // asset-store lock — held across validate+write, NOT across resource notification,
        // NOT while holding a pipeline lease.
        let _guard = crate::workflow::asset_store::lock_asset_store()?;
        validate_workflow_node_definitions(context, &graft.nodes)?;
        for node_file in &new_node_files {
            write_workflow_node_session(context, session_id, node_file)?;
        }
        frontmatter.insert(
            "definition".to_string(),
            serde_yml::to_value(&merged_definition)?,
        );
        frontmatter.insert("nodeStates".to_string(), serde_yml::to_value(&node_states)?);
        let status = match pipeline_status.as_str() {
            "paused" | "cancelled" => pipeline_status.clone(),
            _ => super::super::compute_pipeline_status_with_definition(
                &merged_definition,
                &node_states,
            )
            .to_string(),
        };
        frontmatter.insert("status".to_string(), YamlValue::String(status));
        write_markdown(&uri, Some(&frontmatter), &pipeline_content, &context.config)?;
    }
    emit_pipeline_resource_updated(session_id);

    let ambient_prompt = frontmatter.get("prompt").and_then(YamlValue::as_str);
    let result = build_pipeline_view_result(
        context,
        session_id,
        &merged_definition,
        &node_states,
        ambient_prompt,
        None,
    )?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn merge_graft_definition(
    existing_definition: &PipelineDefinition,
    graft: &GraftRequest,
) -> Result<PipelineDefinition> {
    let existing_node_ids = existing_definition
        .nodes
        .iter()
        .map(|node| node.id())
        .collect::<HashSet<_>>();
    let mut graft_node_ids = HashSet::with_capacity(graft.nodes.len());

    for node in &graft.nodes {
        if existing_node_ids.contains(node.id()) || !graft_node_ids.insert(node.id()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline graft contains duplicate node ID '{}'",
                node.id()
            )));
        }
    }

    let merged_definition = PipelineDefinition {
        nodes: existing_definition
            .nodes
            .iter()
            .cloned()
            .chain(graft.nodes.iter().cloned())
            .collect(),
        edges: existing_definition
            .edges
            .iter()
            .cloned()
            .chain(graft.edges.iter().cloned())
            .collect(),
        inputs: existing_definition.inputs.clone(),
        template_id: None,
    };
    let graph = merged_definition.build_graph();
    if let Err(error) = validate_pipeline(&merged_definition) {
        if is_cyclic_directed(&graph) {
            let cycle = find_cycle_nodes(&graph).unwrap_or_else(|| vec!["<unknown>".to_string()]);
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline graft would create a cycle: {}",
                cycle.join(" -> ")
            )));
        }
        return Err(error);
    }
    if is_cyclic_directed(&graph) {
        let cycle = find_cycle_nodes(&graph).unwrap_or_else(|| vec!["<unknown>".to_string()]);
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline graft would create a cycle: {}",
            cycle.join(" -> ")
        )));
    }

    Ok(merged_definition)
}

fn find_cycle_nodes(
    graph: &petgraph::stable_graph::StableGraph<String, super::super::EdgeType>,
) -> Option<Vec<String>> {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut stack = Vec::new();

    for node_index in graph.node_indices() {
        if let Some(cycle) =
            dfs_cycle_nodes(graph, node_index, &mut visiting, &mut visited, &mut stack)
        {
            return Some(cycle);
        }
    }

    None
}

fn dfs_cycle_nodes(
    graph: &petgraph::stable_graph::StableGraph<String, super::super::EdgeType>,
    node_index: petgraph::prelude::NodeIndex,
    visiting: &mut HashSet<petgraph::prelude::NodeIndex>,
    visited: &mut HashSet<petgraph::prelude::NodeIndex>,
    stack: &mut Vec<petgraph::prelude::NodeIndex>,
) -> Option<Vec<String>> {
    if visited.contains(&node_index) {
        return None;
    }
    visiting.insert(node_index);
    stack.push(node_index);

    for neighbor in graph.neighbors_directed(node_index, Outgoing) {
        if let Some(start) = stack.iter().position(|index| *index == neighbor) {
            let mut cycle = stack[start..]
                .iter()
                .map(|index| graph[*index].clone())
                .collect::<Vec<_>>();
            cycle.push(graph[neighbor].clone());
            return Some(cycle);
        }
        if visiting.contains(&neighbor) {
            continue;
        }
        if let Some(cycle) = dfs_cycle_nodes(graph, neighbor, visiting, visited, stack) {
            return Some(cycle);
        }
    }

    stack.pop();
    visiting.remove(&node_index);
    visited.insert(node_index);
    None
}
