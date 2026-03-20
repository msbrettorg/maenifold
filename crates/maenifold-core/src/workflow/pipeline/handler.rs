use crate::assets::AssetManager;

use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::markdown::{file_exists, write_markdown};
use crate::sessions::load_help_doc;
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use crate::workflow::read_workflow_definition;
use chrono::Utc;
use petgraph::Direction::Outgoing;
use petgraph::algo::is_cyclic_directed;
use serde::Deserialize;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use super::dispatch::dispatch_pipeline;
use super::session_io::{
    NodeFile, allocate_node_session_id, delete_pipeline_session_files, derive_pipeline_node_states,
    frontmatter_value, generate_pipeline_node_session_id, node_status_name,
    persist_pipeline_state_with_status, read_pipeline_session, write_pipeline_session,
    write_workflow_node_session, write_workflow_node_session_with_status,
};
use super::supervisor::{
    supervisor_cancel, supervisor_is_running, supervisor_pause, supervisor_resume, supervisor_run,
};
#[cfg(test)]
use super::test_support;
use super::update::{restart_pipeline_node, update_pipeline_node_status};
use super::view::{build_pipeline_view_result, view_pipeline_session};
use super::{NodeState, NodeStatus, PipelineDefinition, compute_ready_set, validate_pipeline};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinRequest {
    source_session_id: String,
    edges: Vec<super::PipelineEdge>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RewindRequest {
    node_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForkRequest {
    from_node_id: String,
    #[serde(default)]
    new_session_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraftRequest {
    #[serde(default)]
    nodes: Vec<super::PipelineNode>,
    #[serde(default)]
    edges: Vec<super::PipelineEdge>,
}

// T-45.6a: RTM-333 / RTM-334 / RTM-314 / RTM-315 / RTM-316 / RTM-317
// Pipeline create validates graph structure, persists pipeline state, and pre-creates node sessions.
pub fn pipeline(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "Pipeline")
            .unwrap_or_else(|| "ERROR: Help file not found for Pipeline".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let session_id = payload.get("sessionId").and_then(Value::as_str);
    let node_id = payload.get("nodeId").and_then(Value::as_str);
    let status = payload.get("status").and_then(Value::as_str);
    let conclusion = payload.get("conclusion").and_then(Value::as_str);
    let view = payload
        .get("view")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let dispatch = payload
        .get("dispatch")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let restart = payload
        .get("restart")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let run = payload.get("run").and_then(Value::as_bool).unwrap_or(false);
    let pause = payload
        .get("pause")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let resume = payload
        .get("resume")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let cancel = payload
        .get("cancel")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let delete = payload
        .get("delete")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if let (Some(session_id), Some(join_value)) = (session_id, payload.get("join")) {
        let join: JoinRequest = serde_json::from_value(join_value.clone())?;
        return join_pipeline(context, session_id, join);
    }

    if let (Some(session_id), Some(fork_value)) = (session_id, payload.get("fork")) {
        let fork: ForkRequest = serde_json::from_value(fork_value.clone())?;
        return fork_pipeline(context, session_id, fork);
    }

    if let (Some(session_id), Some(graft_value)) = (session_id, payload.get("graft")) {
        let graft: GraftRequest = serde_json::from_value(graft_value.clone())?;
        return graft_pipeline(context, session_id, graft);
    }

    if let (Some(session_id), Some(rewind_value)) = (session_id, payload.get("rewind")) {
        let rewind: RewindRequest = serde_json::from_value(rewind_value.clone())?;
        return rewind_pipeline(context, session_id, rewind);
    }

    if let Some(session_id) = session_id.filter(|_| delete) {
        return delete_pipeline(context, session_id);
    }

    if let Some(session_id) = session_id.filter(|_| run) {
        let prompt = payload.get("prompt").and_then(Value::as_str);
        return run_pipeline(context, session_id, prompt);
    }

    if let Some(session_id) = session_id.filter(|_| pause) {
        return pause_pipeline(context, session_id);
    }

    if let Some(session_id) = session_id.filter(|_| resume) {
        return resume_pipeline(context, session_id);
    }

    if let Some(session_id) = session_id.filter(|_| cancel) {
        return cancel_pipeline(context, session_id);
    }

    if let Some(session_id) = session_id.filter(|_| view) {
        return view_pipeline_session(context, session_id);
    }

    // T-45.7: RTM-337 / RTM-342 / RTM-343 / RTM-344
    // Dispatch computes ready set, atomically claims nodes pending→active, and returns them.
    if let Some(session_id) = session_id.filter(|_| dispatch) {
        return dispatch_pipeline(context, session_id);
    }

    if let (Some(session_id), Some(node_id)) = (session_id.filter(|_| restart), node_id) {
        return restart_pipeline_node(context, session_id, node_id);
    }

    if let (Some(session_id), Some(node_id), Some(status)) = (session_id, node_id, status) {
        match status {
            "completed" | "abandoned" => {
                return update_pipeline_node_status(
                    context, session_id, node_id, status, conclusion,
                );
            }
            _ => {}
        }
    }

    let Some(definition_value) = payload.get("definition") else {
        return Err(MaenifoldError::Message(
            "Invalid parameter combination.\n\n\
             VALID OPERATIONS:\n  \
              Create pipeline: definition\n  \
              Run pipeline: sessionId + run=true\n  \
              Pause pipeline: sessionId + pause=true\n  \
              Resume pipeline: sessionId + resume=true\n  \
              Cancel pipeline: sessionId + cancel=true\n  \
              Rewind pipeline: sessionId + rewind\n  \
              Delete pipeline: sessionId + delete=true\n  \
              Join pipeline: sessionId + join\n  \
              Fork pipeline: sessionId + fork\n  \
              Graft pipeline: sessionId + graft\n  \
             View pipeline: sessionId + view=true\n  \
             Dispatch ready nodes: sessionId + dispatch=true\n  \
              Restart node: sessionId + nodeId + restart=true\n  \
              Update node status: sessionId + nodeId + status\n  \
              Learn tool usage: learn=true\n\n\
              TIP: lifecycle, topology, and view operations all require a sessionId to identify which pipeline session to inspect."
                .to_string(),
        ));
    };

    AssetManager::initialize_assets(&context.config)?;

    let definition: PipelineDefinition = serde_json::from_value(definition_value.clone())?;
    validate_pipeline(&definition)?;

    let max_concurrency = parse_max_concurrency(payload)?;
    let timestamp_millis = Utc::now().timestamp_millis();
    let pipeline_id = format!("pipeline-{timestamp_millis}");

    let mut node_states = HashMap::with_capacity(definition.nodes.len());
    let mut node_states_frontmatter = BTreeMap::new();
    let mut node_files = Vec::with_capacity(definition.nodes.len());
    let mut response_nodes = Vec::with_capacity(definition.nodes.len());

    for node in &definition.nodes {
        read_workflow_definition(&context.config, &node.workflow_id)?;

        let session_id = generate_pipeline_node_session_id(timestamp_millis, &node.id);
        let state = NodeState {
            session_id: session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
        };

        node_states.insert(node.id.clone(), state.clone());
        node_states_frontmatter.insert(node.id.clone(), serde_yaml::to_value(&state)?);
        node_files.push(NodeFile {
            session_id: session_id.clone(),
            node_id: node.id.clone(),
            workflow_id: node.workflow_id.clone(),
        });
        response_nodes.push(json!({
            "nodeId": node.id,
            "workflowId": node.workflow_id,
            "sessionId": session_id,
        }));
    }

    let ready = compute_ready_set(&definition, &node_states);
    write_pipeline_session(
        context,
        &pipeline_id,
        &definition,
        &node_states_frontmatter,
        max_concurrency,
    )?;

    for node_file in &node_files {
        write_workflow_node_session(context, &pipeline_id, node_file)?;
    }

    let result = json!({
        "pipelineId": pipeline_id,
        "ready": ready,
        "nodes": response_nodes,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn delete_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let node_states: HashMap<String, NodeState> = frontmatter_value(&frontmatter, "nodeStates")?;

    // Guard uses stored frontmatter states (not derived from workflow files) so delete
    // works even when workflow session files are missing. Trade-off: frontmatter may lag
    // behind actual workflow file status if a dispatch/view cycle hasn't run recently.
    if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Active))
    {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is currently running; pause or cancel it before deleting"
        )));
    }

    let deleted_files = delete_pipeline_session_files(context, session_id, &node_states)?;
    let result = json!({
        "deletedSessionId": session_id,
        "cleanedUpFiles": deleted_files.cleaned_up_files,
        "skippedFiles": deleted_files.skipped_files,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn run_pipeline(context: &ToolContext, session_id: &str, prompt: Option<&str>) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;

    if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Active))
    {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} already has active nodes"
        )));
    }

    if supervisor_is_running(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    let pid = supervisor_run(
        context.config.maenifold_root.as_path(),
        session_id,
        prompt.unwrap_or(""),
    )?;
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("active"),
        &uri,
        &pipeline_content,
        context,
    )?;

    let result = json!({
        "status": "started",
        "pid": pid,
        "sessionId": session_id,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn pause_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    #[cfg(not(unix))]
    {
        let _ = (context, session_id);
        return Err(MaenifoldError::Message(
            "Pipeline pause is not supported on this platform".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        if !supervisor_is_running(session_id) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline {session_id} isn't running"
            )));
        }

        let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
        let node_states =
            derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
        let pid = supervisor_pause(session_id)?;
        persist_pipeline_state_with_status(
            &mut frontmatter,
            &node_states,
            Some("paused"),
            &uri,
            &pipeline_content,
            context,
        )?;

        let result = json!({
            "status": "paused",
            "sessionId": session_id,
            "pid": pid,
        });

        Ok(match context.output_format {
            OutputFormat::Json => success(result),
            OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
        })
    }
}

fn resume_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    #[cfg(not(unix))]
    {
        let _ = (context, session_id);
        return Err(MaenifoldError::Message(
            "Pipeline resume is not supported on this platform".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
        let node_states =
            derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
        let pipeline_status = frontmatter
            .get("status")
            .and_then(YamlValue::as_str)
            .unwrap_or_default();

        if pipeline_status != "paused" {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline {session_id} is not paused (status: {pipeline_status})"
            )));
        }

        if !supervisor_is_running(session_id) {
            return Err(MaenifoldError::InvalidArgument(
                "Pipeline process is no longer running; use run to restart".to_string(),
            ));
        }

        let pid = supervisor_resume(session_id)?;
        persist_pipeline_state_with_status(
            &mut frontmatter,
            &node_states,
            Some("active"),
            &uri,
            &pipeline_content,
            context,
        )?;

        let result = json!({
            "status": "resumed",
            "sessionId": session_id,
            "pid": pid,
        });

        Ok(match context.output_format {
            OutputFormat::Json => success(result),
            OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
        })
    }
}

fn cancel_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    if !supervisor_is_running(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} isn't running"
        )));
    }

    supervisor_cancel(session_id)?;

    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("cancelled"),
        &uri,
        &pipeline_content,
        context,
    )?;

    let result = json!({
        "status": "cancelled",
        "sessionId": session_id,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn rewind_pipeline(
    _context: &ToolContext,
    _session_id: &str,
    rewind: RewindRequest,
) -> Result<String> {
    let _ = rewind.node_ids;
    Err(MaenifoldError::Message(
        "Pipeline rewind operation isn't implemented yet".to_string(),
    ))
}

fn reject_join_for_running_pipeline(
    pipeline_id: &str,
    pipeline_role: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<()> {
    if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Ready | NodeStatus::Active))
    {
        return Err(MaenifoldError::Message(format!(
            "Pipeline join rejected: {pipeline_role} pipeline {pipeline_id} is running; pause, complete, or cancel it before join."
        )));
    }

    Ok(())
}

fn join_pipeline(
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
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    let source_node_ids = source_definition
        .nodes
        .iter()
        .map(|node| node.id.clone())
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
    if is_cyclic_directed(&merged_definition.build_graph()) {
        return Err(MaenifoldError::InvalidArgument(
            "Joined pipeline contains a cycle".to_string(),
        ));
    }

    let merged_states = target_states
        .into_iter()
        .chain(source_states)
        .collect::<HashMap<_, _>>();

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
        YamlValue::String(super::compute_pipeline_status(&merged_states).to_string()),
    );
    write_markdown(
        &target_uri,
        Some(&target_frontmatter),
        &target_content,
        &context.config,
    )?;

    let result = build_pipeline_view_result(target_session_id, &merged_definition, &merged_states)?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn fork_pipeline(context: &ToolContext, session_id: &str, fork: ForkRequest) -> Result<String> {
    AssetManager::initialize_assets(&context.config)?;

    let new_session_id = fork
        .new_session_id
        .clone()
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

    let node_index = graph
        .node_indices()
        .find(|index| graph[*index] == fork.from_node_id)
        .ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!(
                "Pipeline doesn't contain node '{}'",
                fork.from_node_id
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

    let forked_nodes = definition
        .nodes
        .iter()
        .filter(|node| reachable.contains(&node.id))
        .map(|node| super::PipelineNode {
            id: node_id_map[&node.id].clone(),
            workflow_id: node.workflow_id.clone(),
            acp_client: node.acp_client.clone(),
        })
        .collect::<Vec<_>>();
    let forked_edges = definition
        .edges
        .iter()
        .filter(|edge| reachable.contains(&edge.source) && reachable.contains(&edge.target))
        .map(|edge| super::PipelineEdge {
            source: node_id_map[&edge.source].clone(),
            target: node_id_map[&edge.target].clone(),
            edge_type: edge.edge_type.clone(),
        })
        .collect::<Vec<_>>();

    let forked_definition = PipelineDefinition {
        nodes: forked_nodes,
        edges: forked_edges,
    };
    validate_pipeline(&forked_definition)?;

    let mut node_states = HashMap::with_capacity(forked_definition.nodes.len());
    let mut node_states_frontmatter = BTreeMap::new();
    for node in &forked_definition.nodes {
        let workflow_session_id = allocate_node_session_id(context, &node.id)?;
        let state = NodeState {
            session_id: workflow_session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
        };
        node_states.insert(node.id.clone(), state.clone());
        node_states_frontmatter.insert(node.id.clone(), serde_yaml::to_value(&state)?);
    }

    for node in &forked_definition.nodes {
        write_workflow_node_session(
            context,
            &new_session_id,
            &NodeFile {
                session_id: node_states[&node.id].session_id.clone(),
                node_id: node.id.clone(),
                workflow_id: node.workflow_id.clone(),
            },
        )?;
    }

    write_pipeline_session(
        context,
        &new_session_id,
        &forked_definition,
        &node_states_frontmatter,
        max_concurrency,
    )?;

    let result = build_pipeline_view_result(&new_session_id, &forked_definition, &node_states)?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn graft_pipeline(context: &ToolContext, session_id: &str, graft: GraftRequest) -> Result<String> {
    AssetManager::initialize_assets(&context.config)?;

    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let pipeline_status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .unwrap_or_else(|| super::compute_pipeline_status(&node_states))
        .to_string();
    let has_active_nodes = node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Active));
    let all_nodes_pending = !node_states.is_empty()
        && node_states
            .values()
            .all(|state| matches!(state.status, NodeStatus::Pending));
    let running_frontmatter = matches!(pipeline_status.as_str(), "active" | "running");

    if has_active_nodes || (running_frontmatter && !all_nodes_pending) {
        return Err(MaenifoldError::Message(
            "Pipeline graft rejected: running pipelines must be paused, completed, or cancelled before graft."
                .to_string(),
        ));
    }

    if !all_nodes_pending
        && !matches!(
            pipeline_status.as_str(),
            "paused" | "completed" | "cancelled" | "stuck"
        )
    {
        return Err(MaenifoldError::Message(format!(
            "Pipeline graft rejected: pipeline status must be paused, completed, cancelled, or stuck; current status is {pipeline_status}."
        )));
    }

    let merged_definition = merge_graft_definition(&definition, &graft)?;

    for node in &graft.nodes {
        read_workflow_definition(&context.config, &node.workflow_id)?;
    }

    let mut new_node_files = Vec::with_capacity(graft.nodes.len());
    for node in &graft.nodes {
        let workflow_session_id = allocate_node_session_id(context, &node.id)?;
        let state = NodeState {
            session_id: workflow_session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
        };
        node_states.insert(node.id.clone(), state);
        new_node_files.push(NodeFile {
            session_id: workflow_session_id,
            node_id: node.id.clone(),
            workflow_id: node.workflow_id.clone(),
        });
    }

    frontmatter.insert(
        "definition".to_string(),
        serde_yaml::to_value(&merged_definition)?,
    );
    let status_override = match pipeline_status.as_str() {
        "paused" | "cancelled" => Some(pipeline_status.as_str()),
        _ => None,
    };

    for node_file in &new_node_files {
        write_workflow_node_session(context, session_id, node_file)?;
    }

    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        status_override,
        &uri,
        &pipeline_content,
        context,
    )?;

    let result = build_pipeline_view_result(session_id, &merged_definition, &node_states)?;
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
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    let mut graft_node_ids = HashSet::with_capacity(graft.nodes.len());

    for node in &graft.nodes {
        if existing_node_ids.contains(node.id.as_str()) || !graft_node_ids.insert(node.id.as_str())
        {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline graft contains duplicate node ID '{}'",
                node.id
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
    };
    let merged_node_ids = merged_definition
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();

    for edge in &merged_definition.edges {
        if !merged_node_ids.contains(edge.source.as_str()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline edge references unknown source node '{}'",
                edge.source
            )));
        }

        if !merged_node_ids.contains(edge.target.as_str()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline edge references unknown target node '{}'",
                edge.target
            )));
        }
    }

    let graph = merged_definition.build_graph();
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
    graph: &petgraph::stable_graph::StableGraph<String, super::EdgeType>,
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
    graph: &petgraph::stable_graph::StableGraph<String, super::EdgeType>,
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

fn ensure_workflow_sessions(
    context: &ToolContext,
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Result<()> {
    for node in &definition.nodes {
        let state = node_states.get(&node.id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{}'",
                node.id
            ))
        })?;
        let workflow_uri = workflow_session_uri(&state.session_id);
        if file_exists(&workflow_uri, &context.config)? {
            continue;
        }

        write_workflow_node_session_with_status(
            context,
            pipeline_id,
            &NodeFile {
                session_id: state.session_id.clone(),
                node_id: node.id.clone(),
                workflow_id: node.workflow_id.clone(),
            },
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
        _ => None,
    }
}

fn reachable_node_ids(
    graph: &petgraph::stable_graph::StableGraph<String, super::EdgeType>,
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

fn parse_max_concurrency(payload: &Value) -> Result<u64> {
    match payload.get("maxConcurrency") {
        None => Ok(8),
        Some(Value::Number(value)) => value.as_u64().filter(|value| *value > 0).ok_or_else(|| {
            MaenifoldError::InvalidArgument("maxConcurrency must be a positive integer".to_string())
        }),
        Some(_) => Err(MaenifoldError::InvalidArgument(
            "maxConcurrency must be a positive integer".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::markdown::{delete_file, file_exists};
    use crate::registry::ToolRegistry;
    use serde_json::json;
    use serial_test::serial;
    use std::fs;

    use super::test_support::*;

    fn set_pipeline_status(context: &ToolContext, session_id: &str, status: &str) {
        let uri = pipeline_session_uri(session_id);
        let (frontmatter, content, _) =
            read_markdown(&uri, &context.config).expect("read pipeline");
        let mut frontmatter = frontmatter.expect("pipeline frontmatter");
        frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
        write_markdown(&uri, Some(&frontmatter), &content, &context.config)
            .expect("write pipeline");
    }

    #[test]
    #[serial(config)]
    fn pipeline_delete_completed_pipeline_removes_session_and_workflow_files() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600200";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 2);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "delete": true }),
        )
        .expect("delete completed pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("deletedSessionId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert_eq!(data.get("cleanedUpFiles").and_then(Value::as_u64), Some(3));
        assert_eq!(data.get("skippedFiles").and_then(Value::as_u64), Some(0));
        assert!(
            !file_exists(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("pipeline removed")
        );
        assert!(
            !file_exists(
                &workflow_session_uri("workflow-1735689600200-root"),
                &context.config,
            )
            .expect("root workflow removed")
        );
        assert!(
            !file_exists(
                &workflow_session_uri("workflow-1735689600200-child"),
                &context.config,
            )
            .expect("child workflow removed")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_delete_running_pipeline_rejects_active_nodes() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600201";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600201-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600201-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 2);

        let error = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "delete": true }),
        )
        .expect_err("running pipeline delete should fail");

        assert!(
            error
                .to_string()
                .contains("pause or cancel it before deleting"),
            "unexpected error: {error}"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_delete_skips_missing_workflow_session_files() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600202";
        let missing_workflow_id = "workflow-1735689600202-child";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600202-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: missing_workflow_id.to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 2);
        delete_file(&workflow_session_uri(missing_workflow_id), &context.config)
            .expect("remove child workflow session");

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "delete": true }),
        )
        .expect("delete pipeline with missing workflow session");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("deletedSessionId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert_eq!(data.get("cleanedUpFiles").and_then(Value::as_u64), Some(2));
        assert_eq!(data.get("skippedFiles").and_then(Value::as_u64), Some(1));
        assert!(
            !file_exists(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("pipeline removed")
        );
        assert!(
            !file_exists(
                &workflow_session_uri("workflow-1735689600202-root"),
                &context.config,
            )
            .expect("root workflow removed")
        );
        assert!(
            !file_exists(&workflow_session_uri(missing_workflow_id), &context.config)
                .expect("missing workflow still absent")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_schema_learn_output_documents_all_new_operations() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Markdown);

        let output = pipeline(&context, &json!({ "learn": true })).expect("learn output");

        for keyword in [
            "run", "pause", "resume", "cancel", "rewind", "delete", "graft", "join", "fork",
        ] {
            assert!(
                output.contains(keyword),
                "expected help output to mention {keyword}: {output}"
            );
        }
    }

    #[test]
    #[serial(config)]
    fn pipeline_schema_routes_new_operation_keywords_to_handlers() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

        for (payload, expected_error) in [
            (
                json!({ "sessionId": "pipeline-missing", "run": true }),
                "Session pipeline-missing doesn't exist",
            ),
            (
                json!({ "sessionId": "pipeline-missing", "pause": true }),
                "Pipeline pipeline-missing isn't running",
            ),
            (
                json!({ "sessionId": "pipeline-missing", "resume": true }),
                "Session pipeline-missing doesn't exist",
            ),
            (
                json!({ "sessionId": "pipeline-missing", "cancel": true }),
                "Pipeline pipeline-missing isn't running",
            ),
            (
                json!({ "sessionId": "pipeline-missing", "rewind": { "nodeIds": ["root"] } }),
                "Pipeline rewind operation isn't implemented yet",
            ),
        ] {
            let error = pipeline(&context, &payload).expect_err("operation should route");
            assert_eq!(error.to_string(), expected_error);
        }

        let graft_pipeline_id = "pipeline-1735689600202";
        let graft_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600202-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600202-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, graft_pipeline_id, graft_states, 2);
        let graft_error = pipeline(
            &context,
            &json!({
                "sessionId": graft_pipeline_id,
                "graft": {
                    "nodes": [{ "id": "new-node", "workflowId": "test-workflow-a" }],
                    "edges": []
                }
            }),
        )
        .expect_err("graft should route");
        assert!(
            graft_error
                .to_string()
                .contains("Pipeline graft rejected: running pipelines")
        );

        let join_target_id = "pipeline-1735689600203";
        let join_source_id = "pipeline-1735689600204";
        let join_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600203-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600203-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
        ]);
        let join_source_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600204-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600204-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, join_target_id, join_states, 2);
        seed_dispatch_pipeline(&context, join_source_id, join_source_states, 2);
        let join_error = pipeline(
            &context,
            &json!({
                "sessionId": join_target_id,
                "join": {
                    "sourceSessionId": join_source_id,
                    "edges": []
                }
            }),
        )
        .expect_err("join should route");
        assert!(
            join_error
                .to_string()
                .contains("Cannot join pipelines; conflicting node IDs")
        );

        let fork_pipeline_id = "pipeline-1735689600205";
        let fork_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600205-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600205-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, fork_pipeline_id, fork_states, 2);
        let fork_error = pipeline(
            &context,
            &json!({
                "sessionId": fork_pipeline_id,
                "fork": {
                    "fromNodeId": "missing",
                    "newSessionId": "pipeline-forked"
                }
            }),
        )
        .expect_err("fork should route");
        assert!(
            fork_error
                .to_string()
                .contains("Pipeline doesn't contain node 'missing'")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_tool_registration_includes_pipeline_aliases() {
        let _reset = Config::reset_overrides();
        let registry = ToolRegistry::new();

        assert!(registry.contains("Pipeline"));
        assert!(registry.contains("pipeline"));
        assert!(registry.canonical_names().contains(&"Pipeline"));
    }

    #[test]
    #[serial(config)]
    fn pipeline_run_rejects_pipeline_with_active_nodes() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600990";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode {
                        id: "root".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "child".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "root".to_string(),
                    target: "child".to_string(),
                    edge_type: super::super::EdgeType::Success,
                }],
            },
            HashMap::from([
                (
                    "root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600990-root".to_string(),
                        status: NodeStatus::Active,
                        attempts: 1,
                    },
                ),
                (
                    "child".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600990-child".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                    },
                ),
            ]),
            1,
        );

        let error = run_pipeline(&context, pipeline_id, None).expect_err("run should fail");

        assert!(
            error.to_string().contains("already has active nodes"),
            "unexpected error: {error}"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_run_starts_supervisor_and_reports_pid() {
        let (tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        super::super::supervisor::supervisor_clear_all();
        std::fs::write(
            tmp.path().join("maenifold-acp"),
            "#!/usr/bin/env bash
sleep 30
",
        )
        .expect("write maenifold-acp stub");

        let pipeline_id = "pipeline-1735689600991";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600991-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            )]),
            1,
        );

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "run": true, "prompt": "ship it" }),
        )
        .expect("run pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let pid = data.get("pid").and_then(Value::as_u64).expect("pid");

        assert_eq!(data.get("status").and_then(Value::as_str), Some("started"));
        assert_eq!(
            data.get("sessionId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert!(pid > 0);
        assert!(super::super::supervisor::supervisor_is_running(pipeline_id));

        let view_output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
            .expect("view running pipeline");
        let view_response: Value = serde_json::from_str(&view_output).expect("parse response");
        let view_data = view_response.get("data").expect("view data");
        assert_eq!(
            view_data.get("running").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(view_data.get("pid").and_then(Value::as_u64), Some(pid));
        assert_eq!(view_data.get("attempts").and_then(Value::as_u64), Some(0));

        super::super::supervisor::supervisor_cancel(pipeline_id).expect("cancel running process");
    }

    #[test]
    #[serial(config)]
    fn pipeline_cancel_stops_supervisor_and_marks_pipeline_cancelled() {
        let (tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        super::super::supervisor::supervisor_clear_all();
        std::fs::write(
            tmp.path().join("maenifold-acp"),
            "#!/usr/bin/env bash
sleep 30
",
        )
        .expect("write maenifold-acp stub");

        let pipeline_id = "pipeline-1735689600992";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600992-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            )]),
            1,
        );

        pipeline(&context, &json!({ "sessionId": pipeline_id, "run": true }))
            .expect("start pipeline");
        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "cancel": true }),
        )
        .expect("cancel pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("status").and_then(Value::as_str),
            Some("cancelled")
        );
        assert_eq!(
            data.get("sessionId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert!(!super::super::supervisor::supervisor_is_running(
            pipeline_id
        ));

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read file");
        let frontmatter = frontmatter.expect("frontmatter");
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("cancelled")
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial(config)]
    fn pipeline_pause_and_pipeline_resume_updates_status() {
        let (tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        super::super::supervisor::supervisor_clear_all();
        std::fs::write(
            tmp.path().join("maenifold-acp"),
            "#!/usr/bin/env bash
sleep 30
",
        )
        .expect("write maenifold-acp stub");

        let pipeline_id = "pipeline-1735689600993";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600993-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            )]),
            1,
        );

        pipeline(&context, &json!({ "sessionId": pipeline_id, "run": true }))
            .expect("start pipeline");

        let pause_output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "pause": true }),
        )
        .expect("pause pipeline");
        let pause_response: Value = serde_json::from_str(&pause_output).expect("parse response");
        let pause_data = pause_response.get("data").expect("pause data");

        assert_eq!(
            pause_data.get("status").and_then(Value::as_str),
            Some("paused")
        );
        assert_eq!(
            pause_data.get("sessionId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert!(
            pause_data
                .get("pid")
                .and_then(Value::as_u64)
                .unwrap_or_default()
                > 0,
            "pause should return pid"
        );

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read file");
        let frontmatter = frontmatter.expect("frontmatter");
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("paused")
        );

        let resume_output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "resume": true }),
        )
        .expect("resume pipeline");
        let resume_response: Value = serde_json::from_str(&resume_output).expect("parse response");
        let resume_data = resume_response.get("data").expect("resume data");

        assert_eq!(
            resume_data.get("status").and_then(Value::as_str),
            Some("resumed")
        );
        assert_eq!(
            resume_data.get("sessionId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert!(
            resume_data
                .get("pid")
                .and_then(Value::as_u64)
                .unwrap_or_default()
                > 0,
            "resume should return pid"
        );

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read file");
        let frontmatter = frontmatter.expect("frontmatter");
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("active")
        );

        super::super::supervisor::supervisor_cancel(pipeline_id).expect("cleanup running process");
    }

    #[test]
    #[serial(config)]
    fn pipeline_create_persists_pipeline_and_workflow_sessions() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let payload = json!({
            "definition": {
                "nodes": [
                    { "id": "root", "workflowId": "test-workflow-a" },
                    { "id": "child", "workflowId": "test-workflow-b", "acpClient": "agent-b" }
                ],
                "edges": [
                    { "source": "root", "target": "child" }
                ]
            },
            "maxConcurrency": 4
        });

        let output = pipeline(&context, &payload).expect("create pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let pipeline_id = data
            .get("pipelineId")
            .and_then(Value::as_str)
            .expect("pipeline id");
        let ready = data
            .get("ready")
            .and_then(Value::as_array)
            .expect("ready array");
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");

        assert!(pipeline_id.starts_with("pipeline-"));
        assert_eq!(ready, &vec![Value::String("root".to_string())]);
        assert_eq!(nodes.len(), 2);

        let pipeline_uri = pipeline_session_uri(pipeline_id);
        let (pipeline_frontmatter, pipeline_content, _) =
            read_markdown(&pipeline_uri, &context.config).expect("read pipeline session");
        let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");

        assert_eq!(
            pipeline_frontmatter.get("type").and_then(YamlValue::as_str),
            Some("pipeline")
        );
        assert_eq!(
            pipeline_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("active")
        );
        assert_eq!(
            pipeline_frontmatter
                .get("maxConcurrency")
                .and_then(YamlValue::as_u64),
            Some(4)
        );
        assert_eq!(
            pipeline_frontmatter
                .get("first")
                .and_then(YamlValue::as_u64),
            Some(0)
        );
        assert_eq!(
            pipeline_frontmatter.get("last").and_then(YamlValue::as_u64),
            Some(0)
        );
        assert_eq!(pipeline_content, "# Pipeline Session\n");

        let definition = pipeline_frontmatter
            .get("definition")
            .and_then(YamlValue::as_mapping)
            .expect("definition mapping");
        let definition_nodes = definition
            .get(&YamlValue::String("nodes".to_string()))
            .and_then(YamlValue::as_sequence)
            .expect("definition nodes");
        assert_eq!(definition_nodes.len(), 2);

        let node_states = pipeline_frontmatter
            .get("nodeStates")
            .and_then(YamlValue::as_mapping)
            .expect("node states");
        assert_eq!(node_states.len(), 2);

        for node_value in nodes {
            let node_id = node_value
                .get("nodeId")
                .and_then(Value::as_str)
                .expect("node id");
            let workflow_id = node_value
                .get("workflowId")
                .and_then(Value::as_str)
                .expect("workflow id");
            let session_id = node_value
                .get("sessionId")
                .and_then(Value::as_str)
                .expect("session id");

            assert!(session_id.starts_with("workflow-"));
            assert!(session_id.ends_with(&format!("-{node_id}")));

            let state = node_states
                .get(&YamlValue::String(node_id.to_string()))
                .and_then(YamlValue::as_mapping)
                .expect("node state");
            assert_eq!(
                state
                    .get(&YamlValue::String("status".to_string()))
                    .and_then(YamlValue::as_str),
                Some("pending")
            );
            assert_eq!(
                state
                    .get(&YamlValue::String("attempts".to_string()))
                    .and_then(YamlValue::as_u64),
                Some(0)
            );
            assert_eq!(
                state
                    .get(&YamlValue::String("sessionId".to_string()))
                    .and_then(YamlValue::as_str),
                Some(session_id)
            );

            let workflow_uri = workflow_session_uri(session_id);
            let (workflow_frontmatter, workflow_content, _) =
                read_markdown(&workflow_uri, &context.config).expect("read workflow session");
            let workflow_frontmatter = workflow_frontmatter.expect("workflow frontmatter");

            assert_eq!(
                workflow_frontmatter.get("type").and_then(YamlValue::as_str),
                Some("workflow")
            );
            assert_eq!(
                workflow_frontmatter
                    .get("status")
                    .and_then(YamlValue::as_str),
                Some("pending")
            );
            assert_eq!(
                workflow_frontmatter
                    .get("pipelineId")
                    .and_then(YamlValue::as_str),
                Some(pipeline_id)
            );
            assert_eq!(
                workflow_frontmatter
                    .get("nodeId")
                    .and_then(YamlValue::as_str),
                Some(node_id)
            );
            assert_eq!(
                workflow_frontmatter
                    .get("workflowId")
                    .and_then(YamlValue::as_str),
                Some(workflow_id)
            );
            assert_eq!(
                workflow_frontmatter
                    .get("queue")
                    .and_then(YamlValue::as_sequence)
                    .and_then(|queue| queue.first())
                    .and_then(YamlValue::as_str),
                Some(workflow_id)
            );
            assert_eq!(
                workflow_frontmatter
                    .get("currentWorkflow")
                    .and_then(YamlValue::as_u64),
                Some(0)
            );
            assert_eq!(
                workflow_frontmatter
                    .get("currentStep")
                    .and_then(YamlValue::as_u64),
                Some(0)
            );
            assert_eq!(
                workflow_frontmatter
                    .get("phase")
                    .and_then(YamlValue::as_str),
                Some("running")
            );
            assert_eq!(workflow_content, "# Workflow Session\n");
        }
    }

    #[test]
    #[serial(config)]
    fn pipeline_create_defaults_max_concurrency_to_eight() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let payload = json!({
            "definition": {
                "nodes": [
                    { "id": "solo", "workflowId": "test-workflow-a" }
                ],
                "edges": []
            }
        });

        let output = pipeline(&context, &payload).expect("create pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let pipeline_id = response
            .get("data")
            .and_then(|data| data.get("pipelineId"))
            .and_then(Value::as_str)
            .expect("pipeline id");
        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read file");
        let frontmatter = frontmatter.expect("frontmatter");

        assert_eq!(
            frontmatter
                .get("maxConcurrency")
                .and_then(YamlValue::as_u64),
            Some(8)
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_create_rejects_non_positive_max_concurrency() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

        for max_concurrency in [json!(0), json!(-1)] {
            let payload = json!({
                "definition": {
                    "nodes": [
                        { "id": "solo", "workflowId": "test-workflow-a" }
                    ],
                    "edges": []
                },
                "maxConcurrency": max_concurrency
            });

            let error = pipeline(&context, &payload).expect_err("invalid maxConcurrency should fail");
            assert!(
                error
                    .to_string()
                    .contains("maxConcurrency must be a positive integer"),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    #[serial(config)]
    fn pipeline_learn_returns_help_text() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Markdown);

        let output = pipeline(&context, &json!({ "learn": true })).expect("learn output");

        assert!(output.contains("# Pipeline"));
        assert!(
            output.contains("Creates and manages pipeline sessions composed of workflow nodes.")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_create_rejects_invalid_definition() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let payload = json!({
            "definition": {
                "nodes": [
                    { "id": "a", "workflowId": "test-workflow-a" },
                    { "id": "b", "workflowId": "test-workflow-b" }
                ],
                "edges": [
                    { "source": "a", "target": "b" },
                    { "source": "b", "target": "a", "type": "always" }
                ]
            }
        });

        let error = pipeline(&context, &payload).expect_err("cycle should fail");

        assert!(error.to_string().contains("contains a cycle"));
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_completed_pipeline_adds_node_and_persists_sessions() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600051";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600051-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "completed");

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "child", "workflowId": "test-workflow-b" }
                    ],
                    "edges": [
                        { "source": "root", "target": "child" }
                    ]
                }
            }),
        )
        .expect("graft succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        let child = nodes
            .iter()
            .find(|node| node.get("nodeId") == Some(&Value::String("child".to_string())))
            .expect("child node");
        let child_session_id = child
            .get("sessionId")
            .and_then(Value::as_str)
            .expect("child session id");

        assert_eq!(nodes.len(), 2);
        assert_eq!(
            child.get("workflowId").and_then(Value::as_str),
            Some("test-workflow-b")
        );
        assert_eq!(child.get("status").and_then(Value::as_str), Some("pending"));
        assert!(
            file_exists(&workflow_session_uri(child_session_id), &context.config)
                .expect("workflow exists")
        );

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("pipeline definition");
        assert_eq!(definition.nodes.len(), 2);
        assert_eq!(definition.edges.len(), 1);
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_rejects_cycles_without_mutation() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600052";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: super::super::EdgeType::Success,
                }],
            },
            HashMap::from([
                (
                    "a".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600052-a".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
                (
                    "b".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600052-b".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
            ]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "completed");

        let error = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "c", "workflowId": "test-workflow-a" }
                    ],
                    "edges": [
                        { "source": "b", "target": "c" },
                        { "source": "c", "target": "a" }
                    ]
                }
            }),
        )
        .expect_err("cycle should fail");

        assert!(error.to_string().contains("cycle"));
        assert!(error.to_string().contains("a"));
        assert!(error.to_string().contains("b"));
        assert!(error.to_string().contains("c"));

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("pipeline definition");
        assert_eq!(definition.nodes.len(), 2);
        assert_eq!(definition.edges.len(), 1);
        assert!(
            !file_exists(
                &workflow_session_uri("workflow-1735689600052-c"),
                &context.config
            )
            .expect("workflow file check")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_rejects_duplicate_node_ids() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600053";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600053-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                },
            )]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "completed");

        let error = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "root", "workflowId": "test-workflow-b" }
                    ],
                    "edges": []
                }
            }),
        )
        .expect_err("duplicate node id should fail");

        assert!(error.to_string().contains("duplicate node ID 'root'"));

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("pipeline definition");
        assert_eq!(definition.nodes.len(), 1);
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_completed_dependency_is_immediately_ready() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600054";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600054-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                },
            )]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "completed");

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "downstream", "workflowId": "test-workflow-b" }
                    ],
                    "edges": [
                        { "source": "root", "target": "downstream" }
                    ]
                }
            }),
        )
        .expect("graft succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("downstream".to_string())])
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_rejects_running_pipeline() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600055";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600055-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 0,
                },
            )]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "active");

        let error = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "child", "workflowId": "test-workflow-b" }
                    ],
                    "edges": [
                        { "source": "root", "target": "child" }
                    ]
                }
            }),
        )
        .expect_err("running pipeline should fail");

        assert!(error.to_string().contains("running pipelines"));

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("pipeline definition");
        assert_eq!(definition.nodes.len(), 1);
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_paused_pipeline_is_allowed_and_preserves_status() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600056";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600056-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                },
            )]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "paused");

        pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "child", "workflowId": "test-workflow-b" }
                    ],
                    "edges": [
                        { "source": "root", "target": "child" }
                    ]
                }
            }),
        )
        .expect("paused graft succeeds");

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("paused")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_stuck_pipeline_is_allowed() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600057";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600057-root".to_string(),
                    status: NodeStatus::Stuck,
                    attempts: 2,
                },
            )]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "stuck");

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "recovery", "workflowId": "test-workflow-b" }
                    ],
                    "edges": [
                        { "source": "root", "target": "recovery", "type": "always" }
                    ]
                }
            }),
        )
        .expect("stuck graft succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let nodes = response
            .get("data")
            .and_then(|data| data.get("nodes"))
            .and_then(Value::as_array)
            .expect("nodes array");

        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("recovery".to_string()))
                && node.get("status") == Some(&Value::String("pending".to_string()))
        }));
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_all_pending_pipeline_is_allowed_despite_active_frontmatter() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600058";
        seed_dispatch_pipeline(
            &context,
            pipeline_id,
            HashMap::from([
                (
                    "root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600058-root".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                    },
                ),
                (
                    "child".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600058-child".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                    },
                ),
            ]),
            2,
        );

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "tail", "workflowId": "test-workflow-a" }
                    ],
                    "edges": [
                        { "source": "child", "target": "tail" }
                    ]
                }
            }),
        )
        .expect("all-pending graft succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let nodes = response
            .get("data")
            .and_then(|data| data.get("nodes"))
            .and_then(Value::as_array)
            .expect("nodes array");

        assert_eq!(nodes.len(), 3);
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("tail".to_string()))
                && node.get("status") == Some(&Value::String("pending".to_string()))
        }));
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_duplicate_node_id_wins_over_missing_workflow_definition() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600059";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600059-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "completed");

        let error = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "root", "workflowId": "missing-workflow" }
                    ],
                    "edges": []
                }
            }),
        )
        .expect_err("duplicate node id should be reported first");

        assert!(error.to_string().contains("duplicate node ID 'root'"));
        assert!(!error.to_string().contains("missing-workflow"));
    }

    #[test]
    #[serial(config)]
    fn pipeline_graft_persists_pipeline_only_after_new_workflow_sessions_exist() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600060";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600060-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "completed");

        let workflow_root = context.config.memory_path.join("thinking").join("workflow");
        let original_permissions = fs::metadata(&workflow_root)
            .expect("workflow root metadata")
            .permissions();
        let mut read_only_permissions = original_permissions.clone();
        read_only_permissions.set_readonly(true);
        fs::set_permissions(&workflow_root, read_only_permissions)
            .expect("set workflow root read-only");

        let result = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "graft": {
                    "nodes": [
                        { "id": "child", "workflowId": "test-workflow-b" }
                    ],
                    "edges": [
                        { "source": "root", "target": "child" }
                    ]
                }
            }),
        );

        fs::set_permissions(&workflow_root, original_permissions)
            .expect("restore workflow root permissions");

        let error = result.expect_err("workflow session write should fail");
        assert!(!error.to_string().is_empty(), "expected an I/O error");

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("pipeline definition");

        assert_eq!(definition.nodes.len(), 1);
        assert_eq!(definition.edges.len(), 0);
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_join_merges_two_simple_dags() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let target_id = "pipeline-1735689600100";
        let source_id = "pipeline-1735689600200";

        seed_pipeline_session_with_workflows(
            &context,
            target_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "target-root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "target-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600100-target-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            3,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode {
                        id: "source-root".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "source-leaf".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "source-root".to_string(),
                    target: "source-leaf".to_string(),
                    edge_type: super::super::EdgeType::Success,
                }],
            },
            HashMap::from([
                (
                    "source-root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600200-source-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 2,
                    },
                ),
                (
                    "source-leaf".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600200-source-leaf".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
            ]),
            3,
        );

        let output = pipeline(
            &context,
            &json!({
                "sessionId": target_id,
                "join": {
                    "sourceSessionId": source_id,
                    "edges": [
                        { "source": "target-root", "target": "source-root" }
                    ]
                }
            }),
        )
        .expect("join succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("pipelineId").and_then(Value::as_str),
            Some(target_id)
        );
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&Vec::new())
        );
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        assert_eq!(nodes.len(), 3);
        let source_root = nodes
            .iter()
            .find(|node| node.get("nodeId") == Some(&Value::String("source-root".to_string())))
            .expect("source root node");
        assert_eq!(
            source_root.get("status").and_then(Value::as_str),
            Some("completed")
        );
        assert_eq!(source_root.get("attempts").and_then(Value::as_u64), Some(2));

        let (frontmatter, _, _) = read_markdown(&pipeline_session_uri(target_id), &context.config)
            .expect("read pipeline");
        let frontmatter = frontmatter.expect("frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("definition");
        assert_eq!(definition.nodes.len(), 3);
        assert_eq!(definition.edges.len(), 2);
        assert!(
            definition
                .edges
                .iter()
                .any(|edge| edge.source == "target-root" && edge.target == "source-root")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_join_rejects_node_id_collision() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let target_id = "pipeline-1735689600300";
        let source_id = "pipeline-1735689600400";

        for session_id in [target_id, source_id] {
            seed_pipeline_session_with_workflows(
                &context,
                session_id,
                PipelineDefinition {
                    nodes: vec![super::super::PipelineNode {
                        id: "shared".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    }],
                    edges: vec![],
                },
                HashMap::from([(
                    "shared".to_string(),
                    NodeState {
                        session_id: format!("workflow-{session_id}-shared"),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                )]),
                2,
            );
        }

        let error = pipeline(
            &context,
            &json!({
                "sessionId": target_id,
                "join": {
                    "sourceSessionId": source_id,
                    "edges": []
                }
            }),
        )
        .expect_err("collision should fail");

        assert!(error.to_string().contains("conflicting node IDs: shared"));
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_join_rejects_cycle() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let target_id = "pipeline-1735689600500";
        let source_id = "pipeline-1735689600600";

        seed_pipeline_session_with_workflows(
            &context,
            target_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: super::super::EdgeType::Success,
                }],
            },
            HashMap::from([
                (
                    "a".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600500-a".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 0,
                    },
                ),
                (
                    "b".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600500-b".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
            ]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "c".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "c".to_string(),
                NodeState {
                    session_id: "workflow-1735689600600-c".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            2,
        );

        let error = pipeline(
            &context,
            &json!({
                "sessionId": target_id,
                "join": {
                    "sourceSessionId": source_id,
                    "edges": [
                        { "source": "b", "target": "c" },
                        { "source": "c", "target": "a" }
                    ]
                }
            }),
        )
        .expect_err("cycle should fail");

        assert!(error.to_string().contains("contains a cycle"));
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_join_rejects_running_target_pipeline() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let target_id = "pipeline-1735689600601";
        let source_id = "pipeline-1735689600602";

        seed_pipeline_session_with_workflows(
            &context,
            target_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "target-root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "target-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600601-target-root".to_string(),
                    status: NodeStatus::Ready,
                    attempts: 0,
                },
            )]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "source-root".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "source-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600602-source-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            2,
        );

        let error = pipeline(
            &context,
            &json!({
                "sessionId": target_id,
                "join": {
                    "sourceSessionId": source_id,
                    "edges": []
                }
            }),
        )
        .expect_err("running target should fail");

        assert!(error.to_string().contains("target pipeline"));
        assert!(error.to_string().contains("before join"));
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_join_rejects_running_source_pipeline() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let target_id = "pipeline-1735689600603";
        let source_id = "pipeline-1735689600604";

        seed_pipeline_session_with_workflows(
            &context,
            target_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "target-root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "target-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600603-target-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "source-root".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "source-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600604-source-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            )]),
            2,
        );

        let error = pipeline(
            &context,
            &json!({
                "sessionId": target_id,
                "join": {
                    "sourceSessionId": source_id,
                    "edges": []
                }
            }),
        )
        .expect_err("running source should fail");

        assert!(error.to_string().contains("source pipeline"));
        assert!(error.to_string().contains(source_id));
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_join_rejects_duplicate_bridge_edge() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let target_id = "pipeline-1735689600605";
        let source_id = "pipeline-1735689600606";

        seed_pipeline_session_with_workflows(
            &context,
            target_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "target-root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "target-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600605-target-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode {
                        id: "source-root".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "source-leaf".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "source-root".to_string(),
                    target: "source-leaf".to_string(),
                    edge_type: super::super::EdgeType::Success,
                }],
            },
            HashMap::from([
                (
                    "source-root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600606-source-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
                (
                    "source-leaf".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600606-source-leaf".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
            ]),
            2,
        );

        let error = pipeline(
            &context,
            &json!({
                "sessionId": target_id,
                "join": {
                    "sourceSessionId": source_id,
                    "edges": [
                        { "source": "source-root", "target": "source-leaf", "type": "always" }
                    ]
                }
            }),
        )
        .expect_err("duplicate bridge edge should fail");

        assert!(
            error
                .to_string()
                .contains("duplicate edge 'source-root -> source-leaf'")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_fork_rejects_existing_new_session_id() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let source_id = "pipeline-1735689600698";
        let existing_id = "pipeline-1735689600699";

        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode {
                        id: "root".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "child".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "root".to_string(),
                    target: "child".to_string(),
                    edge_type: super::super::EdgeType::Success,
                }],
            },
            HashMap::from([
                (
                    "root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600698-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
                (
                    "child".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600698-child".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
            ]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            existing_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "existing".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "existing".to_string(),
                NodeState {
                    session_id: "workflow-1735689600699-existing".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            )]),
            1,
        );

        let error = pipeline(
            &context,
            &json!({
                "sessionId": source_id,
                "fork": {
                    "fromNodeId": "root",
                    "newSessionId": existing_id
                }
            }),
        )
        .expect_err("fork should reject existing session id");

        assert!(
            error.to_string().contains("already exists"),
            "unexpected error: {error}"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_fork_from_middle_includes_all_downstream() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let source_id = "pipeline-1735689600700";
        let new_id = "pipeline-1735689600701";

        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode {
                        id: "root".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "middle".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "left".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "right".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                ],
                edges: vec![
                    super::super::PipelineEdge {
                        source: "root".to_string(),
                        target: "middle".to_string(),
                        edge_type: super::super::EdgeType::Success,
                    },
                    super::super::PipelineEdge {
                        source: "middle".to_string(),
                        target: "left".to_string(),
                        edge_type: super::super::EdgeType::Success,
                    },
                    super::super::PipelineEdge {
                        source: "middle".to_string(),
                        target: "right".to_string(),
                        edge_type: super::super::EdgeType::Always,
                    },
                ],
            },
            HashMap::from([
                (
                    "root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600700-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
                (
                    "middle".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600700-middle".to_string(),
                        status: NodeStatus::Active,
                        attempts: 2,
                    },
                ),
                (
                    "left".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600700-left".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 3,
                    },
                ),
                (
                    "right".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600700-right".to_string(),
                        status: NodeStatus::Stuck,
                        attempts: 4,
                    },
                ),
            ]),
            5,
        );

        let output = pipeline(
            &context,
            &json!({
                "sessionId": source_id,
                "fork": {
                    "fromNodeId": "middle",
                    "newSessionId": new_id
                }
            }),
        )
        .expect("fork succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        let node_ids = nodes
            .iter()
            .map(|node| {
                node.get("nodeId")
                    .and_then(Value::as_str)
                    .expect("node id")
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 3);
        assert!(!node_ids.iter().any(|id| id.ends_with("root")));
        assert!(node_ids.iter().any(|id| id.ends_with("middle")));
        assert!(node_ids.iter().any(|id| id.ends_with("left")));
        assert!(node_ids.iter().any(|id| id.ends_with("right")));
        assert!(
            nodes
                .iter()
                .all(|node| node.get("status").and_then(Value::as_str) == Some("pending"))
        );
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String(format!("fork-{new_id}-middle"))])
        );

        let (frontmatter, _, _) = read_markdown(&pipeline_session_uri(new_id), &context.config)
            .expect("read forked pipeline");
        let frontmatter = frontmatter.expect("frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("definition");
        assert_eq!(definition.nodes.len(), 3);
        assert_eq!(definition.edges.len(), 2);
        assert!(
            definition
                .nodes
                .iter()
                .all(|node| node.id.starts_with(&format!("fork-{new_id}-")))
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_fork_from_leaf_creates_single_node_pipeline() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let source_id = "pipeline-1735689600800";
        let new_id = "pipeline-1735689600801";

        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode {
                        id: "root".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                    },
                    super::super::PipelineNode {
                        id: "leaf".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "root".to_string(),
                    target: "leaf".to_string(),
                    edge_type: super::super::EdgeType::Success,
                }],
            },
            HashMap::from([
                (
                    "root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600800-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
                (
                    "leaf".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600800-leaf".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                    },
                ),
            ]),
            4,
        );

        let output = pipeline(
            &context,
            &json!({
                "sessionId": source_id,
                "fork": {
                    "fromNodeId": "leaf",
                    "newSessionId": new_id
                }
            }),
        )
        .expect("fork succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        let edges = data
            .get("edges")
            .and_then(Value::as_array)
            .expect("edges array");

        assert_eq!(nodes.len(), 1);
        assert_eq!(edges.len(), 0);
        assert_eq!(
            nodes[0].get("nodeId").and_then(Value::as_str),
            Some(format!("fork-{new_id}-leaf").as_str())
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_join_fork_fork_prefixes_ids_and_creates_workflow_sessions() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let source_id = "pipeline-1735689600900";
        let new_id = "pipeline-1735689600901";

        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode {
                    id: "solo".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "solo".to_string(),
                NodeState {
                    session_id: "workflow-1735689600900-solo".to_string(),
                    status: NodeStatus::Active,
                    attempts: 9,
                },
            )]),
            1,
        );

        let output = pipeline(
            &context,
            &json!({
                "sessionId": source_id,
                "fork": {
                    "fromNodeId": "solo",
                    "newSessionId": new_id
                }
            }),
        )
        .expect("fork succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let node = data
            .get("nodes")
            .and_then(Value::as_array)
            .and_then(|nodes| nodes.first())
            .expect("node");
        let node_id = node.get("nodeId").and_then(Value::as_str).expect("node id");
        let workflow_session_id = node
            .get("sessionId")
            .and_then(Value::as_str)
            .expect("workflow session id");

        assert_eq!(node_id, format!("fork-{new_id}-solo"));
        assert_eq!(node.get("status").and_then(Value::as_str), Some("pending"));
        assert_eq!(node.get("attempts").and_then(Value::as_u64), Some(0));
        assert!(
            file_exists(&workflow_session_uri(workflow_session_id), &context.config)
                .expect("workflow file exists")
        );

        let (workflow_frontmatter, _, _) =
            read_markdown(&workflow_session_uri(workflow_session_id), &context.config)
                .expect("read workflow");
        let workflow_frontmatter = workflow_frontmatter.expect("frontmatter");
        assert_eq!(
            workflow_frontmatter
                .get("pipelineId")
                .and_then(YamlValue::as_str),
            Some(new_id)
        );
        assert_eq!(
            workflow_frontmatter
                .get("nodeId")
                .and_then(YamlValue::as_str),
            Some(node_id)
        );
        assert_eq!(
            workflow_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("pending")
        );
    }
}
