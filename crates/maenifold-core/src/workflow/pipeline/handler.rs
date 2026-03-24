use crate::assets::AssetManager;

use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::markdown::{append_h2_section, file_exists, write_markdown};
use crate::sessions::load_help_doc;
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use crate::workflow::read_workflow_definition;
use chrono::Utc;
use petgraph::Direction::{Incoming, Outgoing};
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

fn validate_workflow_node_definitions(
    context: &ToolContext,
    nodes: &[super::PipelineNode],
) -> Result<()> {
    for node in nodes {
        if let Some(workflow_id) = node.workflow_id() {
            read_workflow_definition(&context.config, workflow_id)?;
        }
    }

    Ok(())
}

fn workflow_node_file(node: &super::PipelineNode, session_id: String) -> Option<NodeFile> {
    node.workflow_id().map(|workflow_id| NodeFile {
        session_id,
        node_id: node.id().to_string(),
        workflow_id: workflow_id.to_string(),
    })
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

    if payload
        .get("list")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return list_pipelines(context);
    }

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
              List pipelines: list=true\n  \
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

    validate_workflow_node_definitions(context, &definition.nodes)?;

    for node in &definition.nodes {
        let session_id = generate_pipeline_node_session_id(timestamp_millis, node.id());
        let state = NodeState {
            session_id: session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
            output: None,
        };

        node_states.insert(node.id().to_string(), state.clone());
        node_states_frontmatter.insert(node.id().to_string(), serde_yaml::to_value(&state)?);
        if let Some(node_file) = workflow_node_file(node, session_id.clone()) {
            node_files.push(node_file);
        }
        response_nodes.push(json!({
            "nodeId": node.id(),
            "workflowId": node.workflow_id(),
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

fn list_pipelines(context: &ToolContext) -> Result<String> {
    let pipelines_dir = context.config.memory_path.join("pipelines");
    let mut pipelines = Vec::new();

    if pipelines_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&pipelines_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let Some(session_id) = path.file_stem().and_then(|name| name.to_str()) else {
                    continue;
                };

                let uri = pipeline_session_uri(session_id);
                let Ok((Some(frontmatter), _, _)) = read_markdown(&uri, &context.config) else {
                    continue;
                };

                if frontmatter.get("type").and_then(YamlValue::as_str) != Some("pipeline") {
                    continue;
                }

                let node_states = frontmatter
                    .get("nodeStates")
                    .and_then(|value| {
                        serde_yaml::from_value::<HashMap<String, NodeState>>(value.clone()).ok()
                    })
                    .unwrap_or_default();
                let completed_count = node_states
                    .values()
                    .filter(|state| matches!(state.status, NodeStatus::Completed))
                    .count();
                let status = frontmatter
                    .get("status")
                    .and_then(YamlValue::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| super::compute_pipeline_status(&node_states).to_string());
                let node_count = frontmatter
                    .get("definition")
                    .and_then(|value| {
                        serde_yaml::from_value::<PipelineDefinition>(value.clone()).ok()
                    })
                    .map(|definition| definition.nodes.len())
                    .unwrap_or(0);

                pipelines.push(json!({
                    "pipelineId": frontmatter
                        .get("permalink")
                        .and_then(YamlValue::as_str)
                        .unwrap_or(session_id),
                    "status": status,
                    "nodeCount": node_count,
                    "completedCount": completed_count,
                }));
            }
        }
    }

    // Sort by pipelineId for deterministic output.
    pipelines.sort_by(|a, b| {
        let a_id = a.get("pipelineId").and_then(Value::as_str).unwrap_or("");
        let b_id = b.get("pipelineId").and_then(Value::as_str).unwrap_or("");
        a_id.cmp(b_id)
    });

    let result = json!({ "pipelines": pipelines });
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
    let ambient_prompt = prompt.map(str::trim).filter(|value| !value.is_empty());
    let previous_status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .map(str::to_string);

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

    if let Some(prompt) = ambient_prompt {
        frontmatter.insert("prompt".to_string(), YamlValue::String(prompt.to_string()));
    } else {
        frontmatter.remove("prompt");
    }
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("active"),
        &uri,
        &pipeline_content,
        context,
    )?;
    if let Err(error) = supervisor_run(
        context.config.maenifold_root.as_path(),
        session_id,
        ambient_prompt.unwrap_or(""),
    ) {
        persist_pipeline_state_with_status(
            &mut frontmatter,
            &node_states,
            previous_status.as_deref(),
            &uri,
            &pipeline_content,
            context,
        )?;
        return Err(error);
    }

    let result = json!({
        "status": "started",
        "sessionId": session_id,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn pause_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    if !supervisor_is_running(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} isn't running"
        )));
    }

    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    supervisor_pause(session_id)?;
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
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn resume_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let ambient_prompt = frontmatter
        .get("prompt")
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let pipeline_status = frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        .unwrap_or_default();

    if pipeline_status != "paused" {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is not paused (status: {pipeline_status})"
        )));
    }

    if supervisor_is_running(session_id) {
        supervisor_resume(session_id)?;
    } else {
        supervisor_run(
            context.config.maenifold_root.as_path(),
            session_id,
            ambient_prompt,
        )?;
    }
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
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
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
    context: &ToolContext,
    session_id: &str,
    rewind: RewindRequest,
) -> Result<String> {
    AssetManager::initialize_assets(&context.config)?;

    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    if supervisor_is_running(session_id) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is running; pause or cancel before rewinding"
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
        if let Some(node_file) = workflow_node_file(node, workflow_session_id) {
            new_node_files.push(node_file);
        }
    }

    for node_file in &new_node_files {
        write_workflow_node_session(context, session_id, node_file)?;
    }

    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some(super::compute_pipeline_status(&node_states)),
        &uri,
        &pipeline_content,
        context,
    )?;

    let ambient_prompt = frontmatter.get("prompt").and_then(YamlValue::as_str);
    let result =
        build_pipeline_view_result(context, session_id, &definition, &node_states, ambient_prompt)?;
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
    let source_node_states: HashMap<String, NodeState> =
        frontmatter_value(&frontmatter, "nodeStates")?;

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

    // Identify fork root nodes: reachable nodes with no incoming edges from
    // within the reachable set.  For each fork root, collect completed
    // predecessor outputs from the source pipeline so the fork begins with
    // the same reasoning context.
    let fork_root_predecessor_context = collect_fork_predecessor_context(
        &graph,
        &reachable,
        &source_node_states,
    );

    let forked_nodes = definition
        .nodes
        .iter()
        .filter(|node| reachable.contains(node.id()))
        .map(|node| match node {
            super::PipelineNode::Workflow {
                workflow_id,
                acp_client,
                prompt,
                ..
            } => super::PipelineNode::Workflow {
                id: node_id_map[node.id()].clone(),
                workflow_id: workflow_id.clone(),
                acp_client: acp_client.clone(),
                prompt: prompt.clone(),
            },
            super::PipelineNode::Prompt {
                prompt,
                agent,
                tool_mode,
                ..
            } => super::PipelineNode::Prompt {
                id: node_id_map[node.id()].clone(),
                prompt: prompt.clone(),
                agent: agent.clone(),
                tool_mode: tool_mode.clone(),
            },
            super::PipelineNode::SequentialThinking {
                topic,
                total_thoughts,
                analysis_type,
                agent,
                ..
            } => super::PipelineNode::SequentialThinking {
                id: node_id_map[node.id()].clone(),
                topic: topic.clone(),
                total_thoughts: *total_thoughts,
                analysis_type: analysis_type.clone(),
                agent: agent.clone(),
            },
            super::PipelineNode::Tool {
                tool_name,
                tool_args,
                agent,
                ..
            } => super::PipelineNode::Tool {
                id: node_id_map[node.id()].clone(),
                tool_name: tool_name.clone(),
                tool_args: tool_args.clone(),
                agent: agent.clone(),
            },
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
            mapping: edge.mapping.clone(),
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
        let workflow_session_id = allocate_node_session_id(context, node.id())?;
        let state = NodeState {
            session_id: workflow_session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
            output: None,
        };
        node_states.insert(node.id().to_string(), state.clone());
        node_states_frontmatter.insert(node.id().to_string(), serde_yaml::to_value(&state)?);
    }

    // Inject predecessor context into node_states_frontmatter for fork root nodes.
    // This stores completed predecessor outputs from the source pipeline alongside
    // each fork root's node state so the fork begins with reasoning context intact.
    for (original_node_id, predecessor_outputs) in &fork_root_predecessor_context {
        let forked_node_id = &node_id_map[original_node_id];
        if let Some(serde_yaml::Value::Mapping(map)) =
            node_states_frontmatter.get_mut(forked_node_id)
        {
            map.insert(
                serde_yaml::Value::String("predecessorContext".to_string()),
                serde_yaml::to_value(predecessor_outputs)?,
            );
        }
    }

    for node in &forked_definition.nodes {
        if let Some(node_file) = workflow_node_file(node, node_states[node.id()].session_id.clone())
        {
            write_workflow_node_session(context, &new_session_id, &node_file)?;
        }
    }

    for (original_node_id, predecessor_outputs) in &fork_root_predecessor_context {
        let forked_node_id = &node_id_map[original_node_id];
        let Some(state) = node_states.get(forked_node_id) else {
            continue;
        };
        let workflow_uri = workflow_session_uri(&state.session_id);
        if !file_exists(&workflow_uri, &context.config)? {
            continue;
        }

        append_h2_section(
            &workflow_uri,
            "Predecessor Context",
            &format_predecessor_context_content(predecessor_outputs),
            &context.config,
        )?;
    }

    write_pipeline_session(
        context,
        &new_session_id,
        &forked_definition,
        &node_states_frontmatter,
        max_concurrency,
    )?;

    let result =
        build_pipeline_view_result(context, &new_session_id, &forked_definition, &node_states, None)?;
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

    validate_workflow_node_definitions(context, &graft.nodes)?;

    let mut new_node_files = Vec::with_capacity(graft.nodes.len());
    for node in &graft.nodes {
        let workflow_session_id = allocate_node_session_id(context, node.id())?;
        let state = NodeState {
            session_id: workflow_session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
            output: None,
        };
        node_states.insert(node.id().to_string(), state);
        if let Some(node_file) = workflow_node_file(node, workflow_session_id) {
            new_node_files.push(node_file);
        }
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

    let ambient_prompt = frontmatter.get("prompt").and_then(YamlValue::as_str);
    let result = build_pipeline_view_result(
        context,
        session_id,
        &merged_definition,
        &node_states,
        ambient_prompt,
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
        let state = node_states.get(node.id()).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{}'",
                node.id()
            ))
        })?;
        let Some(node_file) = workflow_node_file(node, state.session_id.clone()) else {
            continue;
        };
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

/// For each fork root node (a reachable node with no incoming edges from within
/// the reachable set), collect the completed outputs of its predecessors in the
/// source pipeline. Returns a map from original node ID → Vec<PredecessorOutput>.
/// Nodes that are not fork roots or whose predecessors have no output are absent.
fn collect_fork_predecessor_context(
    graph: &petgraph::stable_graph::StableGraph<String, super::EdgeType>,
    reachable: &HashSet<String>,
    source_node_states: &HashMap<String, NodeState>,
) -> HashMap<String, Vec<super::PredecessorOutput>> {
    let mut result = HashMap::new();

    for node_index in graph.node_indices() {
        let node_id = &graph[node_index];
        if !reachable.contains(node_id.as_str()) {
            continue;
        }

        // A fork root has no incoming edges from other reachable nodes.
        let has_reachable_predecessor = graph
            .neighbors_directed(node_index, Incoming)
            .any(|pred_index| reachable.contains(graph[pred_index].as_str()));
        if has_reachable_predecessor {
            continue;
        }

        // This is a fork root. Collect completed predecessor outputs from
        // the source pipeline (predecessors outside the reachable set).
        let mut predecessor_outputs = Vec::new();
        for pred_index in graph.neighbors_directed(node_index, Incoming) {
            let pred_id = &graph[pred_index];
            if let Some(state) = source_node_states.get(pred_id.as_str()) {
                if matches!(state.status, super::NodeStatus::Completed) {
                    if let Some(output) = &state.output {
                        predecessor_outputs.push(super::PredecessorOutput {
                            node_id: pred_id.clone(),
                            output: output.clone(),
                        });
                    }
                }
            }
        }

        if !predecessor_outputs.is_empty() {
            // Sort for deterministic ordering in tests and serialized output.
            predecessor_outputs.sort_by(|a, b| a.node_id.cmp(&b.node_id));
            result.insert(node_id.clone(), predecessor_outputs);
        }
    }

    result
}

fn format_predecessor_context_content(
    predecessor_outputs: &[super::PredecessorOutput],
) -> String {
    predecessor_outputs
        .iter()
        .map(|predecessor| format!("### From {}\n\n{}", predecessor.node_id, predecessor.output))
        .collect::<Vec<_>>()
        .join("\n\n")
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
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
    fn list_pipelines_returns_empty_when_no_pipelines() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

        let output = pipeline(&context, &json!({ "list": true })).expect("list should succeed");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let pipelines = data
            .get("pipelines")
            .and_then(Value::as_array)
            .expect("pipelines array");

        assert!(
            pipelines.is_empty(),
            "expected empty list, got {pipelines:?}"
        );
    }

    #[test]
    #[serial(config)]
    fn list_pipelines_returns_seeded_sessions() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600001";
        seed_pipeline_view_session(&context, session_id, "pipeline");

        let output = pipeline(&context, &json!({ "list": true })).expect("list should succeed");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let pipelines = data
            .get("pipelines")
            .and_then(Value::as_array)
            .expect("pipelines array");

        assert_eq!(pipelines.len(), 1);
        let entry = &pipelines[0];
        assert_eq!(
            entry.get("pipelineId").and_then(Value::as_str),
            Some(session_id)
        );
        assert_eq!(entry.get("nodeCount").and_then(Value::as_u64), Some(3));
        assert_eq!(entry.get("completedCount").and_then(Value::as_u64), Some(1));
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
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600201-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
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
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: missing_workflow_id.to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                "Session pipeline-missing doesn't exist",
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
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600202-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
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
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600203-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600204-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600205-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
    fn pipeline_rewind_resets_specified_node_to_pending() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600061";
        let root_session_id = "workflow-1735689600061-a".to_string();
        let middle_session_id = "workflow-1735689600061-b".to_string();
        let leaf_session_id = "workflow-1735689600061-c".to_string();
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode::Workflow {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "c".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![
                    super::super::PipelineEdge {
                        source: "a".to_string(),
                        target: "b".to_string(),
                        edge_type: super::super::EdgeType::Success,
                        mapping: None,
                    },
                    super::super::PipelineEdge {
                        source: "b".to_string(),
                        target: "c".to_string(),
                        edge_type: super::super::EdgeType::Success,
                        mapping: None,
                    },
                ],
            },
            HashMap::from([
                (
                    "a".to_string(),
                    NodeState {
                        session_id: root_session_id.clone(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "b".to_string(),
                    NodeState {
                        session_id: middle_session_id.clone(),
                        status: NodeStatus::Completed,
                        attempts: 2,
                        output: None,
                    },
                ),
                (
                    "c".to_string(),
                    NodeState {
                        session_id: leaf_session_id.clone(),
                        status: NodeStatus::Pending,
                        attempts: 3,
                        output: None,
                    },
                ),
            ]),
            2,
        );

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": ["b"] } }),
        )
        .expect("rewind succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let nodes = response["data"]["nodes"].as_array().expect("nodes array");
        let find_node = |node_id: &str| {
            nodes
                .iter()
                .find(|node| node.get("nodeId") == Some(&Value::String(node_id.to_string())))
                .expect("node exists")
        };

        let a = find_node("a");
        assert_eq!(a["sessionId"], root_session_id);
        assert_eq!(a["status"], "completed");
        assert_eq!(a["attempts"], 1);

        let b = find_node("b");
        let b_session_id = b["sessionId"].as_str().expect("b session");
        assert_ne!(b_session_id, middle_session_id);
        assert_eq!(b["status"], "pending");
        assert_eq!(b["attempts"], 0);
        assert!(
            file_exists(&workflow_session_uri(b_session_id), &context.config).expect("b workflow")
        );

        let c = find_node("c");
        let c_session_id = c["sessionId"].as_str().expect("c session");
        assert_ne!(c_session_id, leaf_session_id);
        assert_eq!(c["status"], "pending");
        assert_eq!(c["attempts"], 0);
        assert!(
            file_exists(&workflow_session_uri(c_session_id), &context.config).expect("c workflow")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_rewind_cascades_downstream() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600062";
        let old_session_ids = HashMap::from([
            ("a".to_string(), "workflow-1735689600062-a".to_string()),
            ("b".to_string(), "workflow-1735689600062-b".to_string()),
            ("c".to_string(), "workflow-1735689600062-c".to_string()),
        ]);
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode::Workflow {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "c".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![
                    super::super::PipelineEdge {
                        source: "a".to_string(),
                        target: "b".to_string(),
                        edge_type: super::super::EdgeType::Success,
                        mapping: None,
                    },
                    super::super::PipelineEdge {
                        source: "b".to_string(),
                        target: "c".to_string(),
                        edge_type: super::super::EdgeType::Success,
                        mapping: None,
                    },
                ],
            },
            HashMap::from([
                (
                    "a".to_string(),
                    NodeState {
                        session_id: old_session_ids["a"].clone(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "b".to_string(),
                    NodeState {
                        session_id: old_session_ids["b"].clone(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "c".to_string(),
                    NodeState {
                        session_id: old_session_ids["c"].clone(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
            ]),
            2,
        );
        set_pipeline_status(&context, pipeline_id, "completed");

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": ["a"] } }),
        )
        .expect("rewind succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");

        assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
        for node in nodes {
            let node_id = node["nodeId"].as_str().expect("node id");
            let session_id = node["sessionId"].as_str().expect("session id");
            assert_ne!(session_id, old_session_ids[node_id]);
            assert_eq!(node["status"], "pending");
            assert_eq!(node["attempts"], 0);
            assert!(
                file_exists(&workflow_session_uri(session_id), &context.config)
                    .expect("rewound workflow exists")
            );
        }
    }

    #[test]
    #[serial(config)]
    fn pipeline_rewind_rejects_running_pipeline() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        super::super::supervisor::supervisor_clear_all();
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);

        let pipeline_id = "pipeline-1735689600063";
        let workflow_session_id = "workflow-1735689600063-root";
        write_acpx_stub_config(
            &context.config.maenifold_root,
            json!({
                workflow_session_id: { "delay": 30.0, "text": "still running" }
            }),
        );
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("runner-agent".to_string()),
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: workflow_session_id.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            )]),
            1,
        );

        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "run": true, "prompt": "keep running" }),
        )
        .expect("start pipeline");
        assert!(super::super::supervisor::supervisor_is_running(pipeline_id));
        super::super::supervisor::supervisor_pause(pipeline_id).expect("pause running process");
        assert_eq!(
            super::super::supervisor::supervisor_is_paused(pipeline_id),
            Some(true)
        );
        let error = rewind_pipeline(
            &context,
            pipeline_id,
            RewindRequest {
                node_ids: vec!["root".to_string()],
            },
        )
        .expect_err("running pipeline rewind should fail");

        assert!(
            error.to_string().contains("running") || error.to_string().contains("pause"),
            "unexpected error: {error}"
        );

        super::super::supervisor::supervisor_cancel(pipeline_id).expect("cancel running process");
        super::super::supervisor::supervisor_clear_all();
    }

    #[test]
    #[serial(config)]
    fn pipeline_rewind_rejects_unknown_node_id() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600064";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600064-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
                },
            )]),
            1,
        );

        let error = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": ["missing"] } }),
        )
        .expect_err("missing node should fail");

        assert!(
            error.to_string().contains("Unknown node ID: missing"),
            "unexpected error: {error}"
        );
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
                    super::super::PipelineNode::Workflow {
                        id: "root".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "child".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "root".to_string(),
                    target: "child".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                (
                    "root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600990-root".to_string(),
                        status: NodeStatus::Active,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "child".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600990-child".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                        output: None,
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
    fn pipeline_run_starts_supervisor_and_reports_running_state() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        super::super::supervisor::supervisor_clear_all();
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);

        let pipeline_id = "pipeline-1735689600991";
        let workflow_session_id = "workflow-1735689600991-root";
        write_acpx_stub_config(
            &context.config.maenifold_root,
            json!({
                workflow_session_id: { "delay": 30.0, "text": "still running" }
            }),
        );
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: workflow_session_id.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
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

        assert_eq!(data.get("status").and_then(Value::as_str), Some("started"));
        assert_eq!(
            data.get("sessionId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert!(data.get("pid").is_none() || data.get("pid") == Some(&Value::Null));
        assert!(super::super::supervisor::supervisor_is_running(pipeline_id));

        let view_output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
            .expect("view running pipeline");
        let view_response: Value = serde_json::from_str(&view_output).expect("parse response");
        let view_data = view_response.get("data").expect("view data");
        assert_eq!(
            view_data.get("running").and_then(Value::as_bool),
            Some(true)
        );
        assert!(view_data.get("pid").is_none() || view_data.get("pid") == Some(&Value::Null));
        assert!(
            view_data.get("attempts").is_none() || view_data.get("attempts") == Some(&Value::Null)
        );

        super::super::supervisor::supervisor_cancel(pipeline_id).expect("cancel running process");
    }

    #[test]
    #[serial(config)]
    fn pipeline_cancel_stops_supervisor_and_marks_pipeline_cancelled() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        super::super::supervisor::supervisor_clear_all();
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);

        let pipeline_id = "pipeline-1735689600992";
        let workflow_session_id = "workflow-1735689600992-root";
        write_acpx_stub_config(
            &context.config.maenifold_root,
            json!({
                workflow_session_id: { "delay": 30.0, "text": "still running" }
            }),
        );
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: workflow_session_id.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
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

    #[test]
    #[serial(config)]
    fn pipeline_pause_and_pipeline_resume_updates_status() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        super::super::supervisor::supervisor_clear_all();
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);

        let pipeline_id = "pipeline-1735689600993";
        let workflow_session_id = "workflow-1735689600993-root";
        write_acpx_stub_config(
            &context.config.maenifold_root,
            json!({
                workflow_session_id: { "delay": 30.0, "text": "still running" }
            }),
        );
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: workflow_session_id.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
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
        assert!(pause_data.get("pid").is_none() || pause_data.get("pid") == Some(&Value::Null));

        let read_pipeline_status = || {
            for _ in 0..20 {
                let (frontmatter, _, _) =
                    read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                        .expect("read file");
                if let Some(frontmatter) = frontmatter {
                    return frontmatter
                        .get("status")
                        .and_then(YamlValue::as_str)
                        .map(str::to_string);
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            None
        };
        assert_eq!(read_pipeline_status().as_deref(), Some("paused"));

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
        assert!(resume_data.get("pid").is_none() || resume_data.get("pid") == Some(&Value::Null));

        assert_eq!(read_pipeline_status().as_deref(), Some("active"));

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
    fn pipeline_create_accepts_mixed_node_types() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let payload = json!({
            "definition": {
                "nodes": [
                    { "id": "root", "workflowId": "test-workflow-a" },
                    {
                        "id": "prompt",
                        "type": "prompt",
                        "prompt": "Summarize the workflow result",
                        "agent": "prompt-agent"
                    },
                    {
                        "id": "thinking",
                        "type": "sequentialThinking",
                        "topic": "Review the workflow output",
                        "totalThoughts": 3,
                        "agent": "thinking-agent"
                    },
                    {
                        "id": "tool",
                        "type": "tool",
                        "toolName": "ReadMemory",
                        "toolArgs": { "identifier": "memory://build/test" },
                        "agent": "prompt-agent"
                    }
                ],
                "edges": [
                    { "source": "root", "target": "prompt" },
                    { "source": "prompt", "target": "thinking" },
                    { "source": "thinking", "target": "tool" }
                ]
            }
        });

        let output = pipeline(&context, &payload).expect("create mixed pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let pipeline_id = data
            .get("pipelineId")
            .and_then(Value::as_str)
            .expect("pipeline id");
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");

        assert_eq!(nodes.len(), 4);
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("root".to_string()))
                && node.get("workflowId") == Some(&Value::String("test-workflow-a".to_string()))
        }));
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("prompt".to_string()))
                && node.get("workflowId") == Some(&Value::Null)
        }));
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("thinking".to_string()))
                && node.get("workflowId") == Some(&Value::Null)
        }));
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("tool".to_string()))
                && node.get("workflowId") == Some(&Value::Null)
        }));

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("pipeline definition");

        assert_eq!(definition.nodes.len(), 4);
        assert!(definition.nodes.iter().any(
            |node| matches!(node, super::super::PipelineNode::Prompt { id, .. } if id == "prompt")
        ));
        assert!(definition.nodes.iter().any(|node| matches!(
            node,
            super::super::PipelineNode::SequentialThinking { id, total_thoughts, .. }
                if id == "thinking" && *total_thoughts == 3
        )));
        assert!(definition.nodes.iter().any(|node| matches!(
            node,
            super::super::PipelineNode::Tool { id, tool_name, tool_args, .. }
                if id == "tool"
                    && tool_name == "ReadMemory"
                    && tool_args.as_ref() == Some(&json!({"identifier": "memory://build/test"}))
        )));
    }

    #[test]
    #[serial(config)]
    fn pipeline_view_includes_prompt_node_prompt_and_agent_fields() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600053";
        let prompt_session_id = "workflow-1735689600053-prompt";
        let definition = PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Prompt {
                id: "prompt".to_string(),
                prompt: "Summarize the workflow result".to_string(),
                agent: "prompt-agent".to_string(),
                tool_mode: "default".to_string(),
            }],
            edges: vec![],
        };
        let node_state = NodeState {
            session_id: prompt_session_id.to_string(),
            status: NodeStatus::Pending,
            attempts: 0,
            output: None,
        };
        let node_states = BTreeMap::from([(
            "prompt".to_string(),
            serde_yaml::to_value(&node_state).expect("serialize node state"),
        )]);
        write_pipeline_session(&context, pipeline_id, &definition, &node_states, 1)
            .expect("write prompt pipeline session");

        let mut workflow_frontmatter =
            crate::workflow::initial_workflow_frontmatter(prompt_session_id, &[], "pending");
        workflow_frontmatter.insert(
            "pipelineId".to_string(),
            YamlValue::String(pipeline_id.to_string()),
        );
        workflow_frontmatter.insert(
            "nodeId".to_string(),
            YamlValue::String("prompt".to_string()),
        );
        write_markdown(
            &workflow_session_uri(prompt_session_id),
            Some(&workflow_frontmatter),
            "# Workflow Session\n",
            &context.config,
        )
        .expect("write prompt workflow session");
        assert!(
            file_exists(&workflow_session_uri(prompt_session_id), &context.config)
                .expect("check prompt workflow session"),
            "expected prompt workflow session to exist"
        );

        let view_output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
            .expect("view prompt pipeline");
        let view_response: Value = serde_json::from_str(&view_output).expect("parse view response");
        let prompt_node = view_response
            .get("data")
            .and_then(|data| data.get("nodes"))
            .and_then(Value::as_array)
            .and_then(|nodes| {
                nodes
                    .iter()
                    .find(|node| node.get("nodeId") == Some(&Value::String("prompt".to_string())))
            })
            .expect("prompt node");

        assert_eq!(
            prompt_node.get("type").and_then(Value::as_str),
            Some("prompt")
        );
        assert_eq!(
            prompt_node.get("prompt").and_then(Value::as_str),
            Some("Summarize the workflow result")
        );
        assert_eq!(
            prompt_node.get("agent").and_then(Value::as_str),
            Some("prompt-agent")
        );
        assert_eq!(
            prompt_node.get("status").and_then(Value::as_str),
            Some("pending")
        );
        assert_eq!(prompt_node.get("attempts").and_then(Value::as_u64), Some(0));
        assert_eq!(prompt_node.get("output"), Some(&Value::Null));
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

            let error =
                pipeline(&context, &payload).expect_err("invalid maxConcurrency should fail");
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
    fn pipeline_create_surfaces_type_specific_validation_errors() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let payload = json!({
            "definition": {
                "nodes": [
                    {
                        "id": "bad-prompt",
                        "type": "prompt",
                        "prompt": "   ",
                        "agent": "prompt-agent"
                    }
                ],
                "edges": []
            }
        });

        let error = pipeline(&context, &payload).expect_err("invalid prompt node should fail");
        let message = error.to_string();

        assert!(
            message.contains("bad-prompt"),
            "unexpected error: {message}"
        );
        assert!(message.contains("prompt"), "unexpected error: {message}");
    }

    #[test]
    #[serial(config)]
    fn pipeline_create_rejects_tool_node_with_empty_tool_name() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let payload = json!({
            "definition": {
                "nodes": [
                    {
                        "id": "bad-tool",
                        "type": "tool",
                        "toolName": "   ",
                        "agent": "prompt-agent"
                    }
                ],
                "edges": []
            }
        });

        let error = pipeline(&context, &payload).expect_err("invalid tool node should fail");
        let message = error.to_string();

        assert!(message.contains("bad-tool"), "unexpected error: {message}");
        assert!(message.contains("toolName"), "unexpected error: {message}");
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600051-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
    fn pipeline_graft_accepts_new_node_types() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600051-mixed";
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600051-mixed-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                        {
                            "id": "prompt",
                            "type": "prompt",
                            "prompt": "Summarize the root output",
                            "agent": "prompt-agent"
                        },
                        {
                            "id": "thinking",
                            "type": "sequentialThinking",
                            "topic": "Review the root result",
                            "totalThoughts": 2,
                            "agent": "thinking-agent"
                        },
                        {
                            "id": "tool",
                            "type": "tool",
                            "toolName": "ReadMemory",
                            "toolArgs": { "identifier": "memory://build/test" },
                            "agent": "prompt-agent"
                        }
                    ],
                    "edges": [
                        { "source": "root", "target": "prompt" },
                        { "source": "prompt", "target": "thinking" },
                        { "source": "thinking", "target": "tool" }
                    ]
                }
            }),
        )
        .expect("graft mixed nodes succeeds");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let nodes = response
            .get("data")
            .and_then(|data| data.get("nodes"))
            .and_then(Value::as_array)
            .expect("nodes array");

        assert_eq!(nodes.len(), 4);
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("prompt".to_string()))
                && node.get("agent") == Some(&Value::String("prompt-agent".to_string()))
        }));
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("thinking".to_string()))
                && node.get("agent") == Some(&Value::String("thinking-agent".to_string()))
                && node.get("topic") == Some(&Value::String("Review the root result".to_string()))
        }));
        assert!(nodes.iter().any(|node| {
            node.get("nodeId") == Some(&Value::String("tool".to_string()))
                && node.get("agent") == Some(&Value::String("prompt-agent".to_string()))
                && node.get("toolName") == Some(&Value::String("ReadMemory".to_string()))
                && node.get("toolArgs") == Some(&json!({"identifier": "memory://build/test"}))
        }));

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        let definition: PipelineDefinition =
            frontmatter_value(&frontmatter, "definition").expect("pipeline definition");

        assert_eq!(definition.nodes.len(), 4);
        assert!(definition.nodes.iter().any(
            |node| matches!(node, super::super::PipelineNode::Prompt { id, .. } if id == "prompt")
        ));
        assert!(definition.nodes.iter().any(|node| matches!(
            node,
            super::super::PipelineNode::SequentialThinking { id, total_thoughts, .. }
                if id == "thinking" && *total_thoughts == 2
        )));
        assert!(definition.nodes.iter().any(|node| matches!(
            node,
            super::super::PipelineNode::Tool { id, tool_name, tool_args, .. }
                if id == "tool"
                    && tool_name == "ReadMemory"
                    && tool_args.as_ref() == Some(&json!({"identifier": "memory://build/test"}))
        )));
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
                    super::super::PipelineNode::Workflow {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                (
                    "a".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600052-a".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "b".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600052-b".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600053-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600054-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600055-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 0,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600056-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600057-root".to_string(),
                    status: NodeStatus::Stuck,
                    attempts: 2,
                    output: None,
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
                        output: None,
                    },
                ),
                (
                    "child".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600058-child".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                        output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600059-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600060-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "target-root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "target-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600100-target-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
                },
            )]),
            3,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode::Workflow {
                        id: "source-root".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "source-leaf".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "source-root".to_string(),
                    target: "source-leaf".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                (
                    "source-root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600200-source-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 2,
                        output: None,
                    },
                ),
                (
                    "source-leaf".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600200-source-leaf".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
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
                    nodes: vec![super::super::PipelineNode::Workflow {
                        id: "shared".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    }],
                    edges: vec![],
                },
                HashMap::from([(
                    "shared".to_string(),
                    NodeState {
                        session_id: format!("workflow-{session_id}-shared"),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
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
                    super::super::PipelineNode::Workflow {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                (
                    "a".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600500-a".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 0,
                        output: None,
                    },
                ),
                (
                    "b".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600500-b".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
            ]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "c".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "c".to_string(),
                NodeState {
                    session_id: "workflow-1735689600600-c".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "target-root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "target-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600601-target-root".to_string(),
                    status: NodeStatus::Ready,
                    attempts: 0,
                    output: None,
                },
            )]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "source-root".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "source-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600602-source-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "target-root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "target-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600603-target-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
                },
            )]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "source-root".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "source-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600604-source-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                    output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "target-root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "target-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600605-target-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
                },
            )]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode::Workflow {
                        id: "source-root".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "source-leaf".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "source-root".to_string(),
                    target: "source-leaf".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                (
                    "source-root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600606-source-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "source-leaf".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600606-source-leaf".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
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
                    super::super::PipelineNode::Workflow {
                        id: "root".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "child".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "root".to_string(),
                    target: "child".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                (
                    "root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600698-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "child".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600698-child".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
            ]),
            2,
        );
        seed_pipeline_session_with_workflows(
            &context,
            existing_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "existing".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "existing".to_string(),
                NodeState {
                    session_id: "workflow-1735689600699-existing".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
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
                    super::super::PipelineNode::Workflow {
                        id: "root".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "middle".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "left".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "right".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![
                    super::super::PipelineEdge {
                        source: "root".to_string(),
                        target: "middle".to_string(),
                        edge_type: super::super::EdgeType::Success,
                        mapping: None,
                    },
                    super::super::PipelineEdge {
                        source: "middle".to_string(),
                        target: "left".to_string(),
                        edge_type: super::super::EdgeType::Success,
                        mapping: None,
                    },
                    super::super::PipelineEdge {
                        source: "middle".to_string(),
                        target: "right".to_string(),
                        edge_type: super::super::EdgeType::Always,
                        mapping: None,
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
                        output: None,
                    },
                ),
                (
                    "middle".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600700-middle".to_string(),
                        status: NodeStatus::Active,
                        attempts: 2,
                        output: None,
                    },
                ),
                (
                    "left".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600700-left".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 3,
                        output: None,
                    },
                ),
                (
                    "right".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600700-right".to_string(),
                        status: NodeStatus::Stuck,
                        attempts: 4,
                        output: None,
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
                .all(|node| node.id().starts_with(&format!("fork-{new_id}-")))
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
                    super::super::PipelineNode::Workflow {
                        id: "root".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "leaf".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![super::super::PipelineEdge {
                    source: "root".to_string(),
                    target: "leaf".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                (
                    "root".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600800-root".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "leaf".to_string(),
                    NodeState {
                        session_id: "workflow-1735689600800-leaf".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: None,
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
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "solo".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "solo".to_string(),
                NodeState {
                    session_id: "workflow-1735689600900-solo".to_string(),
                    status: NodeStatus::Active,
                    attempts: 9,
                    output: None,
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

    #[test]
    #[serial(config)]
    fn pipeline_fork_inherits_predecessor_context() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let source_id = "pipeline-1735689601000";
        let new_id = "pipeline-1735689601001";

        // Source pipeline: A → B → C
        // A is completed with output, B is active, C is pending.
        // Forking from B should give B' the output of A as predecessor context.
        seed_pipeline_session_with_workflows(
            &context,
            source_id,
            PipelineDefinition {
                nodes: vec![
                    super::super::PipelineNode::Workflow {
                        id: "A".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "B".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                    super::super::PipelineNode::Workflow {
                        id: "C".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: None,
                        prompt: None,
                    },
                ],
                edges: vec![
                    super::super::PipelineEdge {
                        source: "A".to_string(),
                        target: "B".to_string(),
                        edge_type: super::super::EdgeType::Success,
                        mapping: None,
                    },
                    super::super::PipelineEdge {
                        source: "B".to_string(),
                        target: "C".to_string(),
                        edge_type: super::super::EdgeType::Success,
                        mapping: None,
                    },
                ],
            },
            HashMap::from([
                (
                    "A".to_string(),
                    NodeState {
                        session_id: "workflow-1735689601000-A".to_string(),
                        status: NodeStatus::Completed,
                        attempts: 1,
                        output: Some("A produced this reasoning output".to_string()),
                    },
                ),
                (
                    "B".to_string(),
                    NodeState {
                        session_id: "workflow-1735689601000-B".to_string(),
                        status: NodeStatus::Active,
                        attempts: 1,
                        output: None,
                    },
                ),
                (
                    "C".to_string(),
                    NodeState {
                        session_id: "workflow-1735689601000-C".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                        output: None,
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
                    "fromNodeId": "B",
                    "newSessionId": new_id
                }
            }),
        )
        .expect("fork succeeds");

        // Parse the fork response — should contain B' and C' (not A).
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        assert_eq!(nodes.len(), 2, "forked subgraph should have B and C");

        // Read the forked pipeline session's YAML and verify predecessor context
        // was injected into B' (the fork root).
        let (frontmatter, _, _) = read_markdown(&pipeline_session_uri(new_id), &context.config)
            .expect("read forked pipeline");
        let frontmatter = frontmatter.expect("frontmatter");
        let node_states_yaml = frontmatter
            .get("nodeStates")
            .expect("nodeStates in frontmatter");

        // The fork root is B' (remapped as fork-{new_id}-B).
        let fork_root_id = format!("fork-{new_id}-B");
        let fork_root_state = node_states_yaml
            .get(YamlValue::String(fork_root_id.clone()))
            .unwrap_or_else(|| panic!("node state for {fork_root_id} should exist"));

        let pred_context = fork_root_state
            .get("predecessorContext")
            .expect("fork root should have predecessorContext");
        let pred_list = pred_context.as_sequence().expect("predecessorContext should be a list");
        assert_eq!(pred_list.len(), 1, "B had one completed predecessor (A)");
        let pred = &pred_list[0];
        assert_eq!(
            pred.get("nodeId").and_then(YamlValue::as_str),
            Some("A"),
            "predecessor should reference source node A"
        );
        assert_eq!(
            pred.get("output").and_then(YamlValue::as_str),
            Some("A produced this reasoning output"),
            "predecessor should carry A's output"
        );

        let fork_root_session_id = fork_root_state
            .get("sessionId")
            .and_then(YamlValue::as_str)
            .expect("fork root session id");
        let (_, fork_root_content, _) = read_markdown(
            &workflow_session_uri(fork_root_session_id),
            &context.config,
        )
        .expect("read fork root workflow");
        assert!(
            fork_root_content.contains("## Predecessor Context"),
            "fork root workflow session should materialize predecessor context"
        );
        assert!(
            fork_root_content.contains("### From A\n\nA produced this reasoning output"),
            "fork root workflow session should include inherited predecessor output"
        );

        // C' (not a fork root — has B' as predecessor in the forked subgraph)
        // should NOT have predecessorContext.
        let fork_child_id = format!("fork-{new_id}-C");
        let fork_child_state = node_states_yaml
            .get(YamlValue::String(fork_child_id.clone()))
            .unwrap_or_else(|| panic!("node state for {fork_child_id} should exist"));
        assert!(
            fork_child_state.get("predecessorContext").is_none(),
            "non-root node C' should not have predecessorContext"
        );
    }
}
