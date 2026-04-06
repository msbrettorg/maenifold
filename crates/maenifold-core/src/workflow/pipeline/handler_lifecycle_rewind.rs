use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use petgraph::Direction::Outgoing;
use serde::Deserialize;
use serde_json::Value;
use serde_yaml::Value as YamlValue;
use std::collections::{HashSet, VecDeque};

use super::super::handler_validation::workflow_node_file;
use super::super::lease;
use super::super::session_io::{
    allocate_node_session_id, derive_pipeline_node_states, frontmatter_value,
    persist_pipeline_state_with_status, read_pipeline_session, write_workflow_node_session,
};
use super::super::view::build_pipeline_view_result;
use super::super::{NodeStatus, PipelineDefinition};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RewindRequest {
    node_ids: Vec<String>,
}

pub(super) fn handle_rewind_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<String> {
    let rewind: RewindRequest = serde_json::from_value(payload["rewind"].clone())?;
    rewind_pipeline(context, session_id, rewind)
}

#[allow(clippy::too_many_lines)]
fn rewind_pipeline(
    context: &ToolContext,
    session_id: &str,
    rewind: RewindRequest,
) -> Result<String> {
    AssetManager::initialize_assets(&context.config)?;
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    // T-93.4: Replace in-process supervisor check with OS lock check.
    // An external pipeline-driver holds the fd-lock advisory lock while running;
    // if the lock is held, refuse the rewind to prevent concurrent state mutation.
    if lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is running; cancel it before rewinding"
        )));
    }

    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let graph = definition.build_graph();
    let mut reset_node_ids = HashSet::new();

    for node_id in rewind.node_ids {
        let node_index = graph
            .node_indices()
            .find(|index| graph[*index] == node_id)
            .ok_or_else(|| MaenifoldError::Message(format!("Unknown node ID: {node_id}")))?;
        reset_node_ids.extend(reachable_node_ids(&graph, node_index));
    }

    let mut new_node_files = Vec::with_capacity(reset_node_ids.len());
    for node in definition
        .nodes
        .iter()
        .filter(|node| reset_node_ids.contains(node.id()))
    {
        let workflow_session_id = allocate_node_session_id(context, node.id())?;
        let state = node_states.get_mut(node.id()).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{}'",
                node.id()
            ))
        })?;
        state.session_id = workflow_session_id.clone();
        state.status = NodeStatus::Pending;
        state.attempts = 0;
        state.output = None;
        state.thinking_session_id = None;
        state.failure_reason = None;
        state.retry_after = None;
        state.retry_count = None;
        new_node_files.push(workflow_node_file(node, workflow_session_id));
    }

    for node_file in &new_node_files {
        write_workflow_node_session(context, session_id, node_file)?;
    }
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some(super::super::compute_pipeline_status(&node_states)),
        &uri,
        &pipeline_content,
        context,
    )?;

    let ambient_prompt = frontmatter.get("prompt").and_then(YamlValue::as_str);
    let result = build_pipeline_view_result(
        context,
        session_id,
        &definition,
        &node_states,
        ambient_prompt,
    )?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn reachable_node_ids(
    graph: &petgraph::stable_graph::StableGraph<String, super::super::EdgeType>,
    start: petgraph::prelude::NodeIndex,
) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::from([start]);

    while let Some(node_index) = queue.pop_front() {
        let node_id = graph[node_index].clone();
        if !visited.insert(node_id) {
            continue;
        }
        queue.extend(graph.neighbors_directed(node_index, Outgoing));
    }

    visited
}
