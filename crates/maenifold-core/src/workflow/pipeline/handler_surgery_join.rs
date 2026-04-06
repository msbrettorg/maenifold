use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::{file_exists, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_yaml::Value as YamlValue;
use std::collections::{HashMap, HashSet};

use super::super::handler_validation::workflow_node_file;
use super::super::session_io::{
    frontmatter_value, node_status_name, read_pipeline_session,
    write_workflow_node_session_with_status,
};
use super::super::view::build_pipeline_view_result;
use super::super::{NodeState, NodeStatus, PipelineDefinition, validate_pipeline};
use super::JoinRequest;

#[allow(clippy::too_many_lines)]
pub(super) fn join_pipeline(
    context: &ToolContext,
    target_session_id: &str,
    join: JoinRequest,
) -> Result<String> {
    AssetManager::initialize_assets(&context.config)?;
    let (target_uri, mut target_frontmatter, target_content) =
        read_pipeline_session(context, target_session_id)?;
    let (_, source_frontmatter, _) = read_pipeline_session(context, &join.source_session_id)?;

    let target_definition: PipelineDefinition =
        frontmatter_value(&target_frontmatter, "definition")?;
    let source_definition: PipelineDefinition =
        frontmatter_value(&source_frontmatter, "definition")?;
    let target_states = resolve_join_node_states(
        context,
        &frontmatter_value(&target_frontmatter, "nodeStates")?,
    )?;
    let source_states = resolve_join_node_states(
        context,
        &frontmatter_value(&source_frontmatter, "nodeStates")?,
    )?;
    reject_join_for_running_pipeline(target_session_id, "target", &target_states)?;
    reject_join_for_running_pipeline(&join.source_session_id, "source", &source_states)?;

    let target_node_ids = target_definition
        .nodes
        .iter()
        .map(|node| node.id().to_string())
        .collect::<HashSet<_>>();
    let source_node_ids = source_definition
        .nodes
        .iter()
        .map(|node| node.id().to_string())
        .collect::<HashSet<_>>();
    let mut collisions = target_node_ids
        .intersection(&source_node_ids)
        .cloned()
        .collect::<Vec<_>>();
    collisions.sort();
    if !collisions.is_empty() {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Cannot join pipelines; conflicting node IDs: {}",
            collisions.join(", ")
        )));
    }

    let merged_definition = PipelineDefinition {
        nodes: target_definition
            .nodes
            .iter()
            .cloned()
            .chain(source_definition.nodes.iter().cloned())
            .collect(),
        edges: target_definition
            .edges
            .iter()
            .cloned()
            .chain(source_definition.edges.iter().cloned())
            .chain(join.edges.iter().cloned())
            .collect(),
    };
    validate_pipeline(&merged_definition)?;
    let merged_states = target_states.into_iter().chain(source_states).collect();
    ensure_workflow_sessions(
        context,
        target_session_id,
        &merged_definition,
        &merged_states,
    )?;

    target_frontmatter.insert(
        "definition".to_string(),
        serde_yaml::to_value(&merged_definition)?,
    );
    target_frontmatter.insert(
        "nodeStates".to_string(),
        serde_yaml::to_value(&merged_states)?,
    );
    target_frontmatter.insert(
        "status".to_string(),
        YamlValue::String(super::super::compute_pipeline_status(&merged_states).to_string()),
    );
    write_markdown(
        &target_uri,
        Some(&target_frontmatter),
        &target_content,
        &context.config,
    )?;

    let ambient_prompt = target_frontmatter.get("prompt").and_then(YamlValue::as_str);
    let result = build_pipeline_view_result(
        context,
        target_session_id,
        &merged_definition,
        &merged_states,
        ambient_prompt,
    )?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn reject_join_for_running_pipeline(
    pipeline_id: &str,
    pipeline_role: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<()> {
    if node_states.values().any(|state| {
        matches!(
            state.status,
            NodeStatus::Ready | NodeStatus::Active | NodeStatus::Retrying
        )
    }) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline join rejected: {pipeline_role} pipeline {pipeline_id} is running; complete or cancel it before join."
        )));
    }

    Ok(())
}

fn ensure_workflow_sessions(
    context: &ToolContext,
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Result<()> {
    for node in &definition.nodes {
        let state = node_states.get(node.id()).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{}'",
                node.id()
            ))
        })?;
        let node_file = workflow_node_file(node, state.session_id.clone());
        let workflow_uri = workflow_session_uri(&state.session_id);
        if file_exists(&workflow_uri, &context.config)? {
            continue;
        }
        write_workflow_node_session_with_status(
            context,
            pipeline_id,
            &node_file,
            node_status_name(&state.status),
        )?;
    }

    Ok(())
}

fn resolve_join_node_states(
    context: &ToolContext,
    node_states: &HashMap<String, NodeState>,
) -> Result<HashMap<String, NodeState>> {
    let mut resolved = node_states.clone();
    for state in resolved.values_mut() {
        let workflow_uri = workflow_session_uri(&state.session_id);
        if !file_exists(&workflow_uri, &context.config)? {
            continue;
        }

        let (frontmatter, _, _) = read_markdown(&workflow_uri, &context.config)?;
        let Some(frontmatter) = frontmatter else {
            return Err(MaenifoldError::Message(format!(
                "ERROR: Workflow session {} has invalid format",
                state.session_id
            )));
        };
        if frontmatter.get("type").and_then(YamlValue::as_str) != Some("workflow") {
            return Err(MaenifoldError::Message(format!(
                "Session {} isn't a workflow session",
                state.session_id
            )));
        }
        if let Some(status) = frontmatter
            .get("status")
            .and_then(YamlValue::as_str)
            .and_then(workflow_status_to_node_status)
        {
            state.status = status;
        }
    }

    Ok(resolved)
}

fn workflow_status_to_node_status(status: &str) -> Option<NodeStatus> {
    match status {
        "pending" => Some(NodeStatus::Pending),
        "ready" => Some(NodeStatus::Ready),
        "active" => Some(NodeStatus::Active),
        "completed" => Some(NodeStatus::Completed),
        "abandoned" | "cancelled" => Some(NodeStatus::Abandoned),
        "stuck" => Some(NodeStatus::Stuck),
        "retrying" => Some(NodeStatus::Retrying),
        _ => None,
    }
}
