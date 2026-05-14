use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::{append_h2_section, file_exists};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use serde_yml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

use super::super::handler_validation::workflow_node_file;
use super::super::session_io::{
    allocate_node_session_id, frontmatter_value, read_pipeline_session, write_pipeline_session,
    write_workflow_node_session,
};
use super::super::view::build_pipeline_view_result;
use super::super::{NodeState, NodeStatus, PipelineDefinition, validate_pipeline};
use super::ForkRequest;

#[path = "handler_surgery_fork_helpers.rs"]
mod helpers;
use helpers::{
    collect_fork_predecessor_context, format_predecessor_context_content, reachable_node_ids,
};

#[allow(clippy::too_many_lines)]
pub(super) fn fork_pipeline(
    context: &ToolContext,
    session_id: &str,
    fork: ForkRequest,
) -> Result<String> {
    AssetManager::initialize_assets(&context.config)?;
    let new_session_id = fork
        .new_session_id()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("pipeline-{}", Utc::now().timestamp_millis()));

    let new_pipeline_uri = pipeline_session_uri(&new_session_id);
    if file_exists(&new_pipeline_uri, &context.config)? {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Session {} already exists",
            new_session_id
        )));
    }

    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let graph = definition.build_graph();
    let max_concurrency: u64 = frontmatter_value(&frontmatter, "maxConcurrency")?;
    let source_node_states: HashMap<String, NodeState> =
        frontmatter_value(&frontmatter, "nodeStates")?;
    let node_index = graph
        .node_indices()
        .find(|index| graph[*index] == fork.from_node_id())
        .ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!(
                "Pipeline doesn't contain node '{}'",
                fork.from_node_id()
            ))
        })?;

    let reachable = reachable_node_ids(&graph, node_index);
    let node_id_map = reachable
        .iter()
        .map(|node_id| {
            (
                node_id.clone(),
                format!("fork-{}-{}", new_session_id, node_id),
            )
        })
        .collect::<HashMap<_, _>>();
    let fork_root_predecessor_context =
        collect_fork_predecessor_context(&graph, &reachable, &source_node_states);

    let forked_definition = PipelineDefinition {
        nodes: definition
            .nodes
            .iter()
            .filter(|node| reachable.contains(node.id()))
            .map(|node| match node {
                super::super::PipelineNode::Workflow {
                    display_label,
                    workflow_id,
                    acp_client,
                    prompt,
                    cwd,
                    timeout,
                    agent_config,
                    adopt,
                    requires_role,
                    retry_policy,
                    join_policy,
                    ..
                } => super::super::PipelineNode::Workflow {
                    id: node_id_map[node.id()].clone(),
                    display_label: display_label.clone(),
                    workflow_id: workflow_id.clone(),
                    acp_client: acp_client.clone(),
                    prompt: prompt.clone(),
                    cwd: cwd.clone(),
                    timeout: *timeout,
                    agent_config: agent_config.clone(),
                    adopt: adopt.clone(),
                    requires_role: requires_role.clone(),
                    retry_policy: retry_policy.clone(),
                    join_policy: join_policy.clone(),
                },
                super::super::PipelineNode::Prompt {
                    display_label,
                    prompt,
                    agent,
                    tool_mode,
                    model,
                    thought_level,
                    allowed_tools,
                    max_turns,
                    cwd,
                    timeout,
                    agent_config,
                    adopt,
                    requires_role,
                    retry_policy,
                    join_policy,
                    session_ref,
                    ..
                } => super::super::PipelineNode::Prompt {
                    id: node_id_map[node.id()].clone(),
                    display_label: display_label.clone(),
                    prompt: prompt.clone(),
                    agent: agent.clone(),
                    tool_mode: tool_mode.clone(),
                    model: model.clone(),
                    thought_level: thought_level.clone(),
                    allowed_tools: allowed_tools.clone(),
                    max_turns: *max_turns,
                    cwd: cwd.clone(),
                    timeout: *timeout,
                    agent_config: agent_config.clone(),
                    adopt: adopt.clone(),
                    requires_role: requires_role.clone(),
                    retry_policy: retry_policy.clone(),
                    join_policy: join_policy.clone(),
                    session_ref: session_ref.clone(),
                },
                super::super::PipelineNode::SequentialThinking {
                    display_label,
                    topic,
                    total_thoughts,
                    agent,
                    model,
                    thought_level,
                    allowed_tools,
                    max_turns,
                    cwd,
                    timeout,
                    agent_config,
                    adopt,
                    requires_role,
                    retry_policy,
                    join_policy,
                    ..
                } => super::super::PipelineNode::SequentialThinking {
                    id: node_id_map[node.id()].clone(),
                    display_label: display_label.clone(),
                    topic: topic.clone(),
                    total_thoughts: *total_thoughts,
                    agent: agent.clone(),
                    model: model.clone(),
                    thought_level: thought_level.clone(),
                    allowed_tools: allowed_tools.clone(),
                    max_turns: *max_turns,
                    cwd: cwd.clone(),
                    timeout: *timeout,
                    agent_config: agent_config.clone(),
                    adopt: adopt.clone(),
                    requires_role: requires_role.clone(),
                    retry_policy: retry_policy.clone(),
                    join_policy: join_policy.clone(),
                },
                super::super::PipelineNode::Tool {
                    display_label,
                    tool_name,
                    tool_args,
                    agent,
                    model,
                    thought_level,
                    allowed_tools,
                    max_turns,
                    cwd,
                    timeout,
                    agent_config,
                    retry_policy,
                    join_policy,
                    ..
                } => super::super::PipelineNode::Tool {
                    id: node_id_map[node.id()].clone(),
                    display_label: display_label.clone(),
                    tool_name: tool_name.clone(),
                    tool_args: tool_args.clone(),
                    agent: agent.clone(),
                    model: model.clone(),
                    thought_level: thought_level.clone(),
                    allowed_tools: allowed_tools.clone(),
                    max_turns: *max_turns,
                    cwd: cwd.clone(),
                    timeout: *timeout,
                    agent_config: agent_config.clone(),
                    retry_policy: retry_policy.clone(),
                    join_policy: join_policy.clone(),
                },
            })
            .collect(),
        edges: definition
            .edges
            .iter()
            .filter(|edge| reachable.contains(&edge.source) && reachable.contains(&edge.target))
            .map(|edge| super::super::PipelineEdge {
                source: node_id_map[&edge.source].clone(),
                target: node_id_map[&edge.target].clone(),
                edge_type: edge.edge_type.clone(),
                mapping: edge.mapping.clone(),
            })
            .collect(),
        inputs: definition.inputs.clone(),
        template_id: None,
    };
    validate_pipeline(&forked_definition)?;

    let mut node_states = HashMap::with_capacity(forked_definition.nodes.len());
    let mut node_states_frontmatter = BTreeMap::new();
    for node in &forked_definition.nodes {
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
        node_states.insert(node.id().to_string(), state.clone());
        node_states_frontmatter.insert(node.id().to_string(), serde_yml::to_value(&state)?);
    }
    for (original_node_id, predecessor_outputs) in &fork_root_predecessor_context {
        let forked_node_id = &node_id_map[original_node_id];
        if let Some(serde_yml::Value::Mapping(map)) =
            node_states_frontmatter.get_mut(forked_node_id)
        {
            map.insert(
                serde_yml::Value::String("predecessorContext".to_string()),
                serde_yml::to_value(predecessor_outputs)?,
            );
        }
    }

    for node in &forked_definition.nodes {
        let node_file = workflow_node_file(node, node_states[node.id()].session_id.clone());
        write_workflow_node_session(context, &new_session_id, &node_file)?;
    }
    for (original_node_id, predecessor_outputs) in &fork_root_predecessor_context {
        let forked_node_id = &node_id_map[original_node_id];
        let Some(state) = node_states.get(forked_node_id) else {
            continue;
        };
        let workflow_uri = workflow_session_uri(&state.session_id);
        if file_exists(&workflow_uri, &context.config)? {
            append_h2_section(
                &workflow_uri,
                "Predecessor Context",
                &format_predecessor_context_content(predecessor_outputs),
                &context.config,
            )?;
        }
    }

    write_pipeline_session(
        context,
        &new_session_id,
        &forked_definition,
        &node_states_frontmatter,
        max_concurrency,
        frontmatter.get("prompt").and_then(YamlValue::as_str),
    )?;
    let result = build_pipeline_view_result(
        context,
        &new_session_id,
        &forked_definition,
        &node_states,
        None,
        None,
    )?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}
