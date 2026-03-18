use crate::assets::AssetManager;

use crate::error::{MaenifoldError, Result};
use crate::markdown::{append_h2_section, file_exists, read_markdown, write_markdown, Frontmatter};
use crate::sessions::{
    date_parts_from_session_id, load_help_doc, pipeline_session_uri, workflow_session_uri,
};
use crate::tools::ToolContext;
use crate::util::json::{success, OutputFormat};
use crate::workflow::{
    format_workflow_step, initial_workflow_frontmatter, read_workflow_definition, workflow_steps,
};
use chrono::Utc;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;

use super::{
    compute_pipeline_status, compute_ready_set, validate_pipeline, EdgeType, NodeState, NodeStatus,
    PipelineDefinition,
};

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
             View pipeline: sessionId + view=true\n  \
             Dispatch ready nodes: sessionId + dispatch=true\n  \
             Restart node: sessionId + nodeId + restart=true\n  \
             Update node status: sessionId + nodeId + status\n  \
             Learn tool usage: learn=true\n\n\
             TIP: view=true and restart=true require a sessionId to identify which pipeline session to inspect."
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

fn view_pipeline_session(context: &ToolContext, session_id: &str) -> Result<String> {
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

fn update_pipeline_node_status(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    status: &str,
    conclusion: Option<&str>,
) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    definition
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline definition missing node '{node_id}'"
            ))
        })?;
    let workflow_session_id = {
        let state = node_states.get(node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{node_id}'"
            ))
        })?;
        if state.status != NodeStatus::Active {
            return Err(MaenifoldError::Message(format!(
                "ERROR: Node '{node_id}' must be active to transition to {status}; current status is {:?}",
                state.status
            )));
        }
        state.session_id.clone()
    };

    match status {
        "completed" => {
            let conclusion = conclusion
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    MaenifoldError::Message(
                        "ERROR: Conclusion required when marking pipeline node completed"
                            .to_string(),
                    )
                })?;
            update_workflow_node_frontmatter(
                context,
                &workflow_session_id,
                "completed",
                Some(conclusion),
            )?;
            let state = node_states.get_mut(node_id).expect("checked above");
            state.status = NodeStatus::Completed;
            inject_predecessor_context(context, &definition, &node_states, node_id)?;
        }
        "abandoned" => {
            update_workflow_node_frontmatter(context, &workflow_session_id, "abandoned", None)?;
            let state = node_states.get_mut(node_id).expect("checked above");
            state.status = NodeStatus::Abandoned;
        }
        _ => {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Unsupported pipeline node status '{status}'"
            )));
        }
    }

    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;

    let result = build_pipeline_view_result(session_id, &definition, &node_states)?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn restart_pipeline_node(context: &ToolContext, session_id: &str, node_id: &str) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let max_attempts = frontmatter
        .get("maxAttempts")
        .and_then(YamlValue::as_u64)
        .unwrap_or(3) as u32;
    let node = definition
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline definition missing node '{node_id}'"
            ))
        })?;

    {
        let state = node_states.get_mut(node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{node_id}'"
            ))
        })?;

        if !matches!(state.status, NodeStatus::Abandoned | NodeStatus::Stuck) {
            return Err(MaenifoldError::Message(format!(
                "Node {node_id} is {} — only abandoned or stuck nodes can be restarted",
                node_status_name(&state.status)
            )));
        }

        if state.attempts >= max_attempts {
            state.status = NodeStatus::Stuck;
            persist_pipeline_state(
                &mut frontmatter,
                &node_states,
                &uri,
                &pipeline_content,
                context,
            )?;
            return Err(MaenifoldError::Message(format!(
                "Node {node_id} has reached max attempts ({max_attempts})"
            )));
        }

        state.session_id = allocate_node_session_id(context, node_id)?;
        state.status = NodeStatus::Pending;
        state.attempts += 1;
    }

    let state = node_states.get(node_id).expect("node state checked above");
    write_workflow_node_session(
        context,
        session_id,
        &NodeFile {
            session_id: state.session_id.clone(),
            node_id: node.id.clone(),
            workflow_id: node.workflow_id.clone(),
        },
    )?;
    inject_predecessor_context_for_node(context, &definition, &node_states, node_id)?;
    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;

    let result = build_pipeline_view_result(session_id, &definition, &node_states)?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

// T-45.7: RTM-337 / RTM-342 / RTM-343 / RTM-344
// Dispatch computes the ready set, atomically claims eligible nodes (pending→active),
// renders the first workflow step into each dispatched session, and returns the result.
fn dispatch_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let max_concurrency: u64 = frontmatter
        .get("maxConcurrency")
        .and_then(YamlValue::as_u64)
        .unwrap_or(8);

    // Compute ready set and apply maxConcurrency limit.
    let ready = compute_ready_set(&definition, &node_states);
    let active_count = node_states
        .values()
        .filter(|state| matches!(state.status, NodeStatus::Active))
        .count() as u64;
    let dispatch_slots = max_concurrency.saturating_sub(active_count) as usize;
    let to_dispatch: Vec<String> = ready.into_iter().take(dispatch_slots).collect();

    // Track the dispatch attempt up-front so any partial persistence on error records
    // the same dispatch sequence number as a fully successful run.
    let last = frontmatter
        .get("last")
        .and_then(YamlValue::as_u64)
        .unwrap_or(0);
    frontmatter.insert("last".to_string(), serde_yaml::to_value(last + 1)?);

    // Best-effort dispatch: only include nodes activated in this call, but persist
    // progress after each node so retries observe already-activated sessions.
    let mut dispatched = Vec::with_capacity(to_dispatch.len());
    for node_id in &to_dispatch {
        let workflow_session_id = node_states
            .get(node_id)
            .ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{node_id}'"
                ))
            })?
            .session_id
            .clone();

        let node = definition
            .nodes
            .iter()
            .find(|n| n.id == *node_id)
            .ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline definition missing node '{node_id}'"
                ))
            })?;

        // Activate the workflow session: atomically update status to active and render first step.
        let activated =
            activate_workflow_node_session(context, &node.workflow_id, &workflow_session_id)?;

        let workflow_id = {
            let state = node_states.get_mut(node_id).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{node_id}'"
                ))
            })?;
            state.status = NodeStatus::Active;
            node.workflow_id.clone()
        };

        persist_pipeline_state(
            &mut frontmatter,
            &node_states,
            &uri,
            &pipeline_content,
            context,
        )?;

        if activated {
            dispatched.push(json!({
                "nodeId": node_id,
                "sessionId": workflow_session_id,
                "workflowId": workflow_id,
            }));
        }
    }

    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;

    // Build response with current pipeline state.
    let active_nodes = node_ids_with_status(&definition, &node_states, NodeStatus::Active)?;
    let completed_nodes = node_ids_with_status(&definition, &node_states, NodeStatus::Completed)?;
    let remaining = node_states
        .values()
        .filter(|state| {
            matches!(
                state.status,
                NodeStatus::Pending | NodeStatus::Active | NodeStatus::Ready
            )
        })
        .count();

    let result = json!({
        "dispatched": dispatched,
        "active": active_nodes,
        "completed": completed_nodes,
        "remaining": remaining,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

/// Activate a workflow node session with a single write that updates its frontmatter
/// status to `active` and renders the first workflow step into the session body.
///
/// Returns `Ok(true)` when activation performed a write, or `Ok(false)` when the
/// session was already active and should be skipped.
fn activate_workflow_node_session(
    context: &ToolContext,
    workflow_id: &str,
    session_id: &str,
) -> Result<bool> {
    let workflow_uri = workflow_session_uri(session_id);
    let (session_frontmatter, existing_content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut session_frontmatter = session_frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Workflow session {session_id} has invalid format"
        ))
    })?;

    if session_frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        == Some("active")
    {
        return Ok(false);
    }

    let workflow_value = read_workflow_definition(&context.config, workflow_id)?;
    let steps = workflow_steps(&workflow_value, workflow_id)?;

    // Transition status from pending to active and render the first step into the
    // session content before performing a single write.
    session_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    let updated_content = if let Some(first_step) = steps.first() {
        let heading = format!("Step 1/{}", steps.len());
        let rendered_step = format_workflow_step(first_step)?;
        update_activation_section_frontmatter(
            &mut session_frontmatter,
            &existing_content,
            &heading,
            &rendered_step,
        );
        append_step_section(&existing_content, &heading, &rendered_step)
    } else {
        existing_content
    };

    write_markdown(
        &workflow_uri,
        Some(&session_frontmatter),
        &updated_content,
        &context.config,
    )?;

    Ok(true)
}

fn append_step_section(existing_content: &str, heading: &str, content: &str) -> String {
    let mut new_content = existing_content.to_string();
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&format!("\n## {heading}\n\n{content}\n"));
    new_content
}

fn update_activation_section_frontmatter(
    frontmatter: &mut Frontmatter,
    existing_content: &str,
    heading: &str,
    content: &str,
) {
    let section_summary: String = format!("{heading}: {content}").chars().take(256).collect();
    if existing_content.trim().is_empty() && !frontmatter.contains_key("first") {
        frontmatter.insert(
            "first".to_string(),
            YamlValue::String(section_summary.clone()),
        );
    }
    frontmatter.insert("last".to_string(), YamlValue::String(section_summary));
}

fn read_pipeline_session(
    context: &ToolContext,
    session_id: &str,
) -> Result<(String, Frontmatter, String)> {
    let uri = pipeline_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} doesn't exist"
        )));
    }

    let (frontmatter, content, _) = read_markdown(&uri, &context.config)?;
    let frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;

    if frontmatter.get("type").and_then(YamlValue::as_str) != Some("pipeline") {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} isn't a pipeline session"
        )));
    }

    Ok((uri, frontmatter, content))
}

fn derive_pipeline_node_states(
    context: &ToolContext,
    node_states: &HashMap<String, NodeState>,
) -> Result<HashMap<String, NodeState>> {
    let mut derived = node_states.clone();

    for state in derived.values_mut() {
        let workflow_uri = workflow_session_uri(&state.session_id);
        if !file_exists(&workflow_uri, &context.config)? {
            return Err(MaenifoldError::Message(format!(
                "Workflow session {} doesn't exist",
                state.session_id
            )));
        }

        let (frontmatter, _, _) = read_markdown(&workflow_uri, &context.config)?;
        let frontmatter = frontmatter.ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Workflow session {} has invalid format",
                state.session_id
            ))
        })?;

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

    Ok(derived)
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

struct NodeFile {
    session_id: String,
    node_id: String,
    workflow_id: String,
}

fn frontmatter_value<T>(frontmatter: &Frontmatter, key: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let value = frontmatter.get(key).cloned().ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing frontmatter '{key}'"
        ))
    })?;
    serde_yaml::from_value(value).map_err(Into::into)
}

fn node_ids_with_status(
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

fn build_pipeline_view_result(
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

fn generate_pipeline_node_session_id(timestamp_millis: i64, node_id: &str) -> String {
    format!("workflow-{timestamp_millis}-{node_id}")
}

fn allocate_node_session_id(context: &ToolContext, node_id: &str) -> Result<String> {
    let mut timestamp_millis = Utc::now().timestamp_millis();

    loop {
        let session_id = generate_pipeline_node_session_id(timestamp_millis, node_id);
        let workflow_uri = workflow_session_uri(&session_id);
        if !file_exists(&workflow_uri, &context.config)? {
            return Ok(session_id);
        }
        timestamp_millis += 1;
    }
}

fn persist_pipeline_state(
    frontmatter: &mut Frontmatter,
    node_states: &HashMap<String, NodeState>,
    uri: &str,
    pipeline_content: &str,
    context: &ToolContext,
) -> Result<()> {
    frontmatter.insert("nodeStates".to_string(), serde_yaml::to_value(node_states)?);
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String(compute_pipeline_status(node_states).to_string()),
    );
    write_markdown(uri, Some(frontmatter), pipeline_content, &context.config)
}

fn node_status_name(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "pending",
        NodeStatus::Ready => "ready",
        NodeStatus::Active => "active",
        NodeStatus::Completed => "completed",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Stuck => "stuck",
    }
}

fn update_workflow_node_frontmatter(
    context: &ToolContext,
    workflow_session_id: &str,
    status: &str,
    conclusion: Option<&str>,
) -> Result<()> {
    let workflow_uri = workflow_session_uri(workflow_session_id);
    let (frontmatter, content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Workflow session {workflow_session_id} has invalid format"
        ))
    })?;

    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    if let Some(conclusion) = conclusion {
        frontmatter.insert(
            "conclusion".to_string(),
            YamlValue::String(conclusion.to_string()),
        );
    }

    write_markdown(&workflow_uri, Some(&frontmatter), &content, &context.config)
}

fn inject_predecessor_context(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    completed_node_id: &str,
) -> Result<()> {
    let downstream_node_ids: BTreeSet<&str> = definition
        .edges
        .iter()
        .filter(|edge| {
            edge.source == completed_node_id
                && matches!(edge.edge_type, EdgeType::Success | EdgeType::Always)
        })
        .map(|edge| edge.target.as_str())
        .collect();

    for downstream_node_id in downstream_node_ids {
        let Some(content) =
            predecessor_context_content(context, definition, node_states, downstream_node_id)?
        else {
            continue;
        };

        let downstream_state = node_states.get(downstream_node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{downstream_node_id}'"
            ))
        })?;
        upsert_h2_section(
            context,
            &workflow_session_uri(&downstream_state.session_id),
            "Predecessor Context",
            &content,
        )?;
    }

    Ok(())
}

fn inject_predecessor_context_for_node(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
) -> Result<()> {
    let Some(content) = predecessor_context_content(context, definition, node_states, node_id)?
    else {
        return Ok(());
    };
    let state = node_states.get(node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        ))
    })?;
    upsert_h2_section(
        context,
        &workflow_session_uri(&state.session_id),
        "Predecessor Context",
        &content,
    )
}

fn predecessor_context_content(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    downstream_node_id: &str,
) -> Result<Option<String>> {
    let mut sections = Vec::new();

    for edge in definition.edges.iter().filter(|edge| {
        edge.target == downstream_node_id
            && matches!(edge.edge_type, EdgeType::Success | EdgeType::Always)
    }) {
        let Some(upstream_state) = node_states.get(&edge.source) else {
            continue;
        };
        if upstream_state.status != NodeStatus::Completed {
            continue;
        }

        let (frontmatter, _, _) = read_markdown(
            &workflow_session_uri(&upstream_state.session_id),
            &context.config,
        )?;
        let Some(last) = frontmatter
            .as_ref()
            .and_then(|frontmatter| frontmatter.get("last"))
            .and_then(YamlValue::as_str)
            .map(str::trim)
            .filter(|last| !last.is_empty())
        else {
            continue;
        };

        sections.push(format!("### From {}\n\n{}", edge.source, last));
    }

    if sections.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sections.join("\n\n")))
    }
}

fn upsert_h2_section(context: &ToolContext, uri: &str, heading: &str, content: &str) -> Result<()> {
    let (frontmatter, existing_content, _) = read_markdown(uri, &context.config)?;
    let frontmatter = frontmatter.unwrap_or_default();

    if has_h2_section(&existing_content, heading) {
        let updated_content = replace_h2_section(&existing_content, heading, content)?;
        write_markdown(uri, Some(&frontmatter), &updated_content, &context.config)
    } else {
        append_h2_section(uri, heading, content, &context.config)
    }
}

fn has_h2_section(existing_content: &str, heading: &str) -> bool {
    existing_content.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("## ") && trimmed.trim_start_matches('#').trim() == heading
    })
}

fn replace_h2_section(existing_content: &str, heading: &str, new_content: &str) -> Result<String> {
    let mut line_start = 0usize;
    let mut section_start = None;
    let mut section_end = existing_content.len();

    for line in existing_content.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
            let heading_text = trimmed.trim_end().trim_start_matches('#').trim();
            if section_start.is_some() {
                section_end = line_start;
                break;
            }
            if heading_text == heading {
                section_start = Some(line_start);
            }
        }
        line_start += line.len();
    }

    let section_start = section_start.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Section not found: '{heading}'. Cannot replace a section that doesn't exist."
        ))
    })?;

    let replacement = format!("## {heading}\n\n{}\n", new_content.trim_end());
    Ok(format!(
        "{}{}{}",
        &existing_content[..section_start],
        replacement,
        &existing_content[section_end..]
    ))
}

fn write_pipeline_session(
    context: &ToolContext,
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &BTreeMap<String, YamlValue>,
    max_concurrency: u64,
) -> Result<()> {
    let uri = pipeline_session_uri(pipeline_id);
    let (year, month, day) = date_parts_from_session_id(pipeline_id);
    fs::create_dir_all(
        context
            .config
            .memory_path
            .join("pipelines")
            .join(format!("{year:04}"))
            .join(format!("{month:02}"))
            .join(format!("{day:02}")),
    )?;
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Pipeline Session {pipeline_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(pipeline_id.to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("pipeline".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert("definition".to_string(), serde_yaml::to_value(definition)?);
    frontmatter.insert(
        "nodeStates".to_string(),
        YamlValue::Mapping(
            node_states
                .iter()
                .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                .collect(),
        ),
    );
    frontmatter.insert(
        "maxConcurrency".to_string(),
        serde_yaml::to_value(max_concurrency)?,
    );
    frontmatter.insert("first".to_string(), serde_yaml::to_value(0u64)?);
    frontmatter.insert("last".to_string(), serde_yaml::to_value(0u64)?);

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
}

fn write_workflow_node_session(
    context: &ToolContext,
    pipeline_id: &str,
    node_file: &NodeFile,
) -> Result<()> {
    let uri = workflow_session_uri(&node_file.session_id);
    let mut frontmatter = initial_workflow_frontmatter(
        &node_file.session_id,
        std::slice::from_ref(&node_file.workflow_id),
        "pending",
    );
    frontmatter.insert(
        "pipelineId".to_string(),
        YamlValue::String(pipeline_id.to_string()),
    );
    frontmatter.insert(
        "nodeId".to_string(),
        YamlValue::String(node_file.node_id.to_string()),
    );
    frontmatter.insert(
        "workflowId".to_string(),
        YamlValue::String(node_file.workflow_id.to_string()),
    );

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Workflow Session\n",
        &context.config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::markdown::write_markdown;
    use crate::registry::ToolRegistry;
    use once_cell::sync::Lazy;
    use serde_json::json;
    use std::fs;
    use std::sync::{Mutex, MutexGuard};
    use tempfile::TempDir;

    static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    struct ConfigResetGuard;

    impl Drop for ConfigResetGuard {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    fn pipeline_test_context(
        output_format: OutputFormat,
    ) -> (
        TempDir,
        ToolContext,
        ConfigResetGuard,
        MutexGuard<'static, ()>,
    ) {
        let test_lock = TEST_MUTEX.lock().expect("test mutex");
        let tmp = TempDir::new().expect("temp dir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        fs::create_dir_all(config.assets_path.join("workflows")).expect("create workflows dir");
        fs::create_dir_all(config.assets_path.join("usage/tools")).expect("create usage dir");
        fs::write(
            config.assets_path.join("usage/tools/pipeline.md"),
            "# Pipeline\n\nCreate and inspect pipelines.\n",
        )
        .expect("write help");

        for workflow_id in ["test-workflow-a", "test-workflow-b"] {
            fs::write(
                config
                    .assets_path
                    .join("workflows")
                    .join(format!("{workflow_id}.json")),
                serde_json::to_string_pretty(&json!({
                    "steps": [
                        {
                            "id": "step-1",
                            "name": "Step 1",
                            "description": format!("Run [[{workflow_id}]]")
                        }
                    ]
                }))
                .expect("serialize workflow"),
            )
            .expect("write workflow");
        }

        (
            tmp,
            ToolContext::new(config, output_format),
            ConfigResetGuard,
            test_lock,
        )
    }

    fn seed_pipeline_view_session(
        context: &ToolContext,
        session_id: &str,
        session_type: &str,
    ) -> PipelineDefinition {
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "cleanup".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: Some("agent-b".to_string()),
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "child".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![
                crate::workflow::pipeline::PipelineEdge {
                    source: "root".to_string(),
                    target: "child".to_string(),
                    edge_type: EdgeType::Success,
                },
                crate::workflow::pipeline::PipelineEdge {
                    source: "root".to_string(),
                    target: "cleanup".to_string(),
                    edge_type: EdgeType::Always,
                },
            ],
        };
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "cleanup".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-cleanup".to_string(),
                    status: NodeStatus::Active,
                    attempts: 2,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);

        let mut frontmatter = Frontmatter::new();
        frontmatter.insert(
            "title".to_string(),
            YamlValue::String(format!("Pipeline Session {session_id}")),
        );
        frontmatter.insert(
            "permalink".to_string(),
            YamlValue::String(session_id.to_string()),
        );
        frontmatter.insert(
            "type".to_string(),
            YamlValue::String(session_type.to_string()),
        );
        frontmatter.insert(
            "definition".to_string(),
            serde_yaml::to_value(&definition).expect("serialize definition"),
        );
        frontmatter.insert(
            "nodeStates".to_string(),
            serde_yaml::to_value(&node_states).expect("serialize node states"),
        );

        write_markdown(
            &pipeline_session_uri(session_id),
            Some(&frontmatter),
            "# Pipeline Session\n",
            &context.config,
        )
        .expect("write pipeline session");

        for (node_id, state) in &node_states {
            let workflow_id = definition
                .nodes
                .iter()
                .find(|node| node.id == *node_id)
                .expect("workflow node")
                .workflow_id
                .clone();
            let mut workflow_frontmatter = crate::workflow::initial_workflow_frontmatter(
                &state.session_id,
                std::slice::from_ref(&workflow_id),
                match state.status {
                    NodeStatus::Pending => "pending",
                    NodeStatus::Ready => "ready",
                    NodeStatus::Active => "active",
                    NodeStatus::Completed => "completed",
                    NodeStatus::Abandoned => "abandoned",
                    NodeStatus::Stuck => "stuck",
                },
            );
            workflow_frontmatter.insert(
                "pipelineId".to_string(),
                YamlValue::String(session_id.to_string()),
            );
            workflow_frontmatter.insert("nodeId".to_string(), YamlValue::String(node_id.clone()));
            workflow_frontmatter.insert("workflowId".to_string(), YamlValue::String(workflow_id));
            write_markdown(
                &workflow_session_uri(&state.session_id),
                Some(&workflow_frontmatter),
                "# Workflow Session\n",
                &context.config,
            )
            .expect("write workflow session");
        }

        definition
    }

    #[test]
    fn pipeline_tool_registration_includes_pipeline_aliases() {
        let _test_lock = TEST_MUTEX.lock().expect("test mutex");
        let registry = ToolRegistry::new();

        assert!(registry.contains("Pipeline"));
        assert!(registry.contains("pipeline"));
        assert!(registry.canonical_names().contains(&"Pipeline"));
    }

    #[test]
    fn pipeline_create_persists_pipeline_and_workflow_sessions() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
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
    fn pipeline_create_defaults_max_concurrency_to_eight() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
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
    fn pipeline_learn_returns_help_text() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Markdown);

        let output = pipeline(&context, &json!({ "learn": true })).expect("learn output");

        assert!(output.contains("# Pipeline"));
        assert!(output.contains("Create and inspect pipelines."));
    }

    #[test]
    fn pipeline_create_rejects_invalid_definition() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
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
    fn pipeline_view_returns_structured_json_and_mermaid() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
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

    #[test]
    fn pipeline_view_rejects_missing_session() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let error = pipeline(
            &context,
            &json!({ "sessionId": "pipeline-1735689600999", "view": true }),
        )
        .expect_err("missing session should fail");

        assert_eq!(
            error.to_string(),
            "Session pipeline-1735689600999 doesn't exist"
        );
    }

    #[test]
    fn pipeline_view_rejects_non_pipeline_session() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600001";
        seed_pipeline_view_session(&context, session_id, "workflow");

        let error = pipeline(&context, &json!({ "sessionId": session_id, "view": true }))
            .expect_err("non-pipeline session should fail");

        assert_eq!(
            error.to_string(),
            format!("Session {session_id} isn't a pipeline session")
        );
    }

    #[test]
    fn pipeline_resume_view_derives_state_from_workflow_sessions() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600002";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600002-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600002-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
        update_workflow_node_frontmatter(
            &context,
            "workflow-1735689600002-root",
            "completed",
            Some("Completed [[root]]"),
        )
        .expect("mark workflow completed");

        let output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
            .expect("resume via view");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("child".to_string())])
        );
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        let root = nodes
            .iter()
            .find(|node| node.get("nodeId") == Some(&Value::String("root".to_string())))
            .expect("root node");
        assert_eq!(
            root.get("status").and_then(Value::as_str),
            Some("completed")
        );
    }

    /// Create a pipeline session with pre-created workflow node sessions, mimicking
    /// the state after `pipeline(definition=...)`. Returns the pipeline definition.
    fn seed_dispatch_pipeline(
        context: &ToolContext,
        pipeline_id: &str,
        node_states: HashMap<String, NodeState>,
        max_concurrency: u64,
    ) -> PipelineDefinition {
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "child".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Success,
            }],
        };

        seed_pipeline_session_with_workflows(
            context,
            pipeline_id,
            definition.clone(),
            node_states,
            max_concurrency,
        );

        definition
    }

    fn seed_pipeline_session_with_workflows(
        context: &ToolContext,
        pipeline_id: &str,
        definition: PipelineDefinition,
        node_states: HashMap<String, NodeState>,
        max_concurrency: u64,
    ) {
        let node_status_string = |status: &NodeStatus| match status {
            NodeStatus::Pending => "pending",
            NodeStatus::Active => "active",
            NodeStatus::Completed => "completed",
            NodeStatus::Abandoned => "abandoned",
            NodeStatus::Ready => "ready",
            NodeStatus::Stuck => "stuck",
        };

        // Write pipeline session.
        let node_states_yaml: BTreeMap<String, YamlValue> = node_states
            .iter()
            .map(|(key, value)| (key.clone(), serde_yaml::to_value(value).expect("yaml")))
            .collect();
        let mut frontmatter = Frontmatter::new();
        frontmatter.insert(
            "title".to_string(),
            YamlValue::String(format!("Pipeline Session {pipeline_id}")),
        );
        frontmatter.insert(
            "permalink".to_string(),
            YamlValue::String(pipeline_id.to_string()),
        );
        frontmatter.insert(
            "type".to_string(),
            YamlValue::String("pipeline".to_string()),
        );
        frontmatter.insert(
            "status".to_string(),
            YamlValue::String("active".to_string()),
        );
        frontmatter.insert(
            "definition".to_string(),
            serde_yaml::to_value(&definition).expect("serialize definition"),
        );
        frontmatter.insert(
            "nodeStates".to_string(),
            YamlValue::Mapping(
                node_states_yaml
                    .iter()
                    .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                    .collect(),
            ),
        );
        frontmatter.insert(
            "maxConcurrency".to_string(),
            serde_yaml::to_value(max_concurrency).expect("yaml"),
        );

        write_markdown(
            &pipeline_session_uri(pipeline_id),
            Some(&frontmatter),
            "# Pipeline Session\n",
            &context.config,
        )
        .expect("write pipeline session");

        // Write workflow node sessions for each node.
        for node in &definition.nodes {
            let state = node_states.get(&node.id).expect("node state");
            let uri = workflow_session_uri(&state.session_id);
            let mut wf_fm = crate::workflow::initial_workflow_frontmatter(
                &state.session_id,
                std::slice::from_ref(&node.workflow_id),
                node_status_string(&state.status),
            );
            wf_fm.insert(
                "pipelineId".to_string(),
                YamlValue::String(pipeline_id.to_string()),
            );
            wf_fm.insert("nodeId".to_string(), YamlValue::String(node.id.to_string()));
            wf_fm.insert(
                "workflowId".to_string(),
                YamlValue::String(node.workflow_id.to_string()),
            );
            write_markdown(&uri, Some(&wf_fm), "# Workflow Session\n", &context.config)
                .expect("write workflow session");
        }
    }

    #[test]
    fn pipeline_dispatch_transitions_ready_nodes_to_active() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600000";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        // Only root should be dispatched (child depends on root).
        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0]["nodeId"], "root");
        assert_eq!(dispatched[0]["sessionId"], "workflow-1735689600000-root");
        assert_eq!(dispatched[0]["workflowId"], "test-workflow-a");

        // Active should include root.
        let active = data
            .get("active")
            .and_then(Value::as_array)
            .expect("active array");
        assert!(active.contains(&Value::String("root".to_string())));

        // Completed should be empty.
        let completed = data
            .get("completed")
            .and_then(Value::as_array)
            .expect("completed array");
        assert!(completed.is_empty());

        // Remaining = 2 (root=active, child=pending — both non-completed).
        assert_eq!(data.get("remaining").and_then(Value::as_u64), Some(2));

        // Verify pipeline session was updated: root now active.
        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);
        assert_eq!(updated_states["child"].status, NodeStatus::Pending);

        // Verify `last` field was incremented.
        assert_eq!(pipeline_fm.get("last").and_then(YamlValue::as_u64), Some(1));

        // Verify workflow session was updated: status=active and first step rendered.
        let (wf_fm, wf_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600000-root"),
            &context.config,
        )
        .expect("read workflow session");
        let wf_fm = wf_fm.expect("workflow frontmatter");
        assert_eq!(
            wf_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert!(
            wf_content.contains("## Step 1/1"),
            "first step should be rendered in session body, got: {wf_content}"
        );
        assert!(
            wf_content.contains("step-1"),
            "rendered step should contain step ID, got: {wf_content}"
        );
    }

    #[test]
    fn pipeline_dispatch_respects_max_concurrency() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600100";

        // Create a 3-node pipeline (root→child, plus a sibling with no deps).
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "sibling".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "child".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Success,
            }],
        };

        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600100-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "sibling".to_string(),
                NodeState {
                    session_id: "workflow-1735689600100-sibling".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600100-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);

        // Write custom pipeline session with maxConcurrency=1.
        let node_states_yaml: BTreeMap<String, YamlValue> = node_states
            .iter()
            .map(|(key, value)| (key.clone(), serde_yaml::to_value(value).expect("yaml")))
            .collect();
        let mut frontmatter = Frontmatter::new();
        frontmatter.insert(
            "title".to_string(),
            YamlValue::String(format!("Pipeline Session {pipeline_id}")),
        );
        frontmatter.insert(
            "permalink".to_string(),
            YamlValue::String(pipeline_id.to_string()),
        );
        frontmatter.insert(
            "type".to_string(),
            YamlValue::String("pipeline".to_string()),
        );
        frontmatter.insert(
            "status".to_string(),
            YamlValue::String("active".to_string()),
        );
        frontmatter.insert(
            "definition".to_string(),
            serde_yaml::to_value(&definition).expect("yaml"),
        );
        frontmatter.insert(
            "nodeStates".to_string(),
            YamlValue::Mapping(
                node_states_yaml
                    .iter()
                    .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                    .collect(),
            ),
        );
        frontmatter.insert(
            "maxConcurrency".to_string(),
            serde_yaml::to_value(1u64).expect("yaml"),
        );

        write_markdown(
            &pipeline_session_uri(pipeline_id),
            Some(&frontmatter),
            "# Pipeline Session\n",
            &context.config,
        )
        .expect("write pipeline session");

        // Write workflow sessions for all nodes.
        for node in &definition.nodes {
            let state = node_states.get(&node.id).expect("state");
            let uri = workflow_session_uri(&state.session_id);
            let mut wf_fm = crate::workflow::initial_workflow_frontmatter(
                &state.session_id,
                std::slice::from_ref(&node.workflow_id),
                "pending",
            );
            wf_fm.insert(
                "pipelineId".to_string(),
                YamlValue::String(pipeline_id.to_string()),
            );
            wf_fm.insert("nodeId".to_string(), YamlValue::String(node.id.to_string()));
            wf_fm.insert(
                "workflowId".to_string(),
                YamlValue::String(node.workflow_id.to_string()),
            );
            write_markdown(&uri, Some(&wf_fm), "# Workflow Session\n", &context.config)
                .expect("write workflow session");
        }

        // Both root and sibling are ready (no deps), but maxConcurrency=1.
        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert_eq!(
            dispatched.len(),
            1,
            "maxConcurrency=1 should dispatch only 1 node"
        );
    }

    #[test]
    fn pipeline_dispatch_only_reports_nodes_transitioned_in_this_call() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600150";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "sibling".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![],
        };
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600150-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "sibling".to_string(),
                NodeState {
                    session_id: "workflow-1735689600150-sibling".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

        activate_workflow_node_session(&context, "test-workflow-a", "workflow-1735689600150-root")
            .expect("activate root workflow session");

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("dispatched").and_then(Value::as_array),
            Some(&vec![json!({
                "nodeId": "sibling",
                "sessionId": "workflow-1735689600150-sibling",
                "workflowId": "test-workflow-b",
            })])
        );
        assert_eq!(
            data.get("active").and_then(Value::as_array),
            Some(&vec![
                Value::String("root".to_string()),
                Value::String("sibling".to_string()),
            ])
        );

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);
        assert_eq!(updated_states["sibling"].status, NodeStatus::Active);

        let (_, root_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600150-root"),
            &context.config,
        )
        .expect("read root workflow");
        assert_eq!(root_content.matches("## Step 1/1").count(), 1);
    }

    #[test]
    fn pipeline_dispatch_returns_empty_when_no_nodes_ready() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600200";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert!(
            dispatched.is_empty(),
            "no nodes should be dispatched when none are ready"
        );

        // Active list should still contain root.
        let active = data
            .get("active")
            .and_then(Value::as_array)
            .expect("active array");
        assert!(active.contains(&Value::String("root".to_string())));
    }

    #[test]
    fn pipeline_dispatch_returns_empty_when_at_max_concurrency() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600300";
        // Root is already active; child is pending but blocked by root (success edge).
        // maxConcurrency=1, 1 already active → 0 dispatch slots.
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600300-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600300-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 1);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert!(
            dispatched.is_empty(),
            "should dispatch nothing when at max concurrency"
        );
    }

    #[test]
    fn pipeline_dispatch_renders_first_step_into_session_body() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600400";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600400-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600400-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");

        // Verify the dispatched node's workflow session has the first step rendered.
        let (wf_fm, wf_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600400-root"),
            &context.config,
        )
        .expect("read workflow session");
        let wf_fm = wf_fm.expect("workflow frontmatter");

        // Status should be active.
        assert_eq!(
            wf_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );

        // Body should contain rendered step with H2 heading and step content.
        assert!(
            wf_content.contains("## Step 1/1"),
            "body must contain step heading, got: {wf_content}"
        );
        assert!(
            wf_content.contains("\"id\": \"step-1\"") || wf_content.contains("step-1"),
            "body must contain step-1 from workflow JSON, got: {wf_content}"
        );
        assert!(
            wf_content.contains("Run [[test-workflow-a]]"),
            "body must contain step description, got: {wf_content}"
        );

        // Un-dispatched child should still be pending with no step content.
        let (child_fm, child_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600400-child"),
            &context.config,
        )
        .expect("read child workflow session");
        let child_fm = child_fm.expect("child frontmatter");
        assert_eq!(
            child_fm.get("status").and_then(YamlValue::as_str),
            Some("pending")
        );
        assert!(
            !child_content.contains("## Step"),
            "child should not have steps rendered"
        );
    }

    #[test]
    fn pipeline_dispatch_persists_partial_progress_when_activation_fails() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600460";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "broken".to_string(),
                    workflow_id: "missing-workflow".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![],
        };
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600460-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "broken".to_string(),
                NodeState {
                    session_id: "workflow-1735689600460-broken".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

        let error = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect_err("dispatch should fail when a workflow asset is missing");
        assert!(
            error.to_string().contains("missing-workflow"),
            "unexpected error: {error}"
        );

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);
        assert_eq!(updated_states["broken"].status, NodeStatus::Pending);
        assert_eq!(pipeline_fm.get("last").and_then(YamlValue::as_u64), Some(1));

        let (root_fm, root_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600460-root"),
            &context.config,
        )
        .expect("read root workflow");
        let root_fm = root_fm.expect("root frontmatter");
        assert_eq!(
            root_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert_eq!(root_content.matches("## Step 1/1").count(), 1);
    }

    #[test]
    fn activate_workflow_node_session_skips_active_session_without_duplication() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600450";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600450-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600450-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let first_activation = activate_workflow_node_session(
            &context,
            "test-workflow-a",
            "workflow-1735689600450-root",
        )
        .expect("activate workflow session");
        assert!(first_activation, "pending session should activate");

        let second_activation = activate_workflow_node_session(
            &context,
            "test-workflow-a",
            "workflow-1735689600450-root",
        )
        .expect("skip already active session");
        assert!(
            !second_activation,
            "already active session should be skipped without rewriting"
        );

        let (wf_fm, wf_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600450-root"),
            &context.config,
        )
        .expect("read workflow session");
        let wf_fm = wf_fm.expect("workflow frontmatter");
        assert_eq!(
            wf_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert_eq!(
            wf_content.matches("## Step 1/1").count(),
            1,
            "step heading must only be rendered once"
        );
    }

    #[test]
    fn pipeline_dispatch_skips_already_active_workflow_sessions() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600475";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600475-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600475-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let activated = activate_workflow_node_session(
            &context,
            "test-workflow-a",
            "workflow-1735689600475-root",
        )
        .expect("activate workflow session");
        assert!(activated, "setup should pre-activate workflow session");

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert!(
            dispatched.is_empty(),
            "pre-activated sessions should be skipped from dispatched response"
        );

        let active = data
            .get("active")
            .and_then(Value::as_array)
            .expect("active array");
        assert_eq!(active, &vec![Value::String("root".to_string())]);

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);

        let (_, wf_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600475-root"),
            &context.config,
        )
        .expect("read workflow session");
        assert_eq!(
            wf_content.matches("## Step 1/1").count(),
            1,
            "dispatch retry must not append duplicate first steps"
        );
    }

    #[test]
    fn pipeline_status_completed_updates_node_and_returns_pipeline_view() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600500";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600500-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600500-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let root_uri = workflow_session_uri("workflow-1735689600500-root");
        let (root_fm, root_content, _) =
            read_markdown(&root_uri, &context.config).expect("read root session");
        let mut root_fm = root_fm.expect("root frontmatter");
        root_fm.insert(
            "last".to_string(),
            YamlValue::String("Step 1 Response: Root [[context]] summary".to_string()),
        );
        write_markdown(&root_uri, Some(&root_fm), &root_content, &context.config)
            .expect("update root session");

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "root",
                "status": "completed",
                "conclusion": "Root [[context]] conclusion"
            }),
        )
        .expect("complete pipeline node");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("pipelineId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("child".to_string())])
        );
        assert_eq!(
            data.get("completed").and_then(Value::as_array),
            Some(&vec![Value::String("root".to_string())])
        );
        assert_eq!(
            data.get("active").and_then(Value::as_array),
            Some(&Vec::<Value>::new())
        );

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline session");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Completed);
        assert_eq!(updated_states["child"].status, NodeStatus::Pending);

        let (root_fm, _, _) = read_markdown(&root_uri, &context.config).expect("read root session");
        let root_fm = root_fm.expect("root frontmatter");
        assert_eq!(
            root_fm.get("status").and_then(YamlValue::as_str),
            Some("completed")
        );
        assert_eq!(
            root_fm.get("conclusion").and_then(YamlValue::as_str),
            Some("Root [[context]] conclusion")
        );
    }

    #[test]
    fn pipeline_status_abandoned_enables_always_edge_ready_set() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600600";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "cleanup".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "cleanup".to_string(),
                edge_type: EdgeType::Always,
            }],
        };
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600600-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 0,
                },
            ),
            (
                "cleanup".to_string(),
                NodeState {
                    session_id: "workflow-1735689600600-cleanup".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "root",
                "status": "abandoned"
            }),
        )
        .expect("abandon pipeline node");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("cleanup".to_string())])
        );
        assert_eq!(
            data.get("completed").and_then(Value::as_array),
            Some(&Vec::<Value>::new())
        );

        let cleanup_nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        assert_eq!(cleanup_nodes[0]["status"], "abandoned");
        assert_eq!(cleanup_nodes[1]["status"], "pending");

        let (root_fm, _, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600600-root"),
            &context.config,
        )
        .expect("read root session");
        let root_fm = root_fm.expect("root frontmatter");
        assert_eq!(
            root_fm.get("status").and_then(YamlValue::as_str),
            Some("abandoned")
        );
    }

    #[test]
    fn pipeline_status_rejects_non_active_transitions() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600700";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600700-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600700-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let error = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "root",
                "status": "completed",
                "conclusion": "Should fail [[context]]"
            }),
        )
        .expect_err("non-active transition should fail");

        assert!(
            error
                .to_string()
                .contains("must be active to transition to completed"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn predecessor_context_replaces_existing_section_with_all_completed_upstreams() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600800";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root-a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "root-b".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "child".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![
                crate::workflow::pipeline::PipelineEdge {
                    source: "root-a".to_string(),
                    target: "child".to_string(),
                    edge_type: EdgeType::Success,
                },
                crate::workflow::pipeline::PipelineEdge {
                    source: "root-b".to_string(),
                    target: "child".to_string(),
                    edge_type: EdgeType::Always,
                },
            ],
        };
        let node_states = HashMap::from([
            (
                "root-a".to_string(),
                NodeState {
                    session_id: "workflow-1735689600800-root-a".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "root-b".to_string(),
                NodeState {
                    session_id: "workflow-1735689600800-root-b".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600800-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

        for (session_id, last_value) in [
            (
                "workflow-1735689600800-root-a",
                "Step 1 Response: Alpha [[context]]",
            ),
            (
                "workflow-1735689600800-root-b",
                "Step 1 Response: Beta [[context]]",
            ),
        ] {
            let uri = workflow_session_uri(session_id);
            let (frontmatter, content, _) =
                read_markdown(&uri, &context.config).expect("read upstream session");
            let mut frontmatter = frontmatter.expect("upstream frontmatter");
            frontmatter.insert(
                "last".to_string(),
                YamlValue::String(last_value.to_string()),
            );
            write_markdown(&uri, Some(&frontmatter), &content, &context.config)
                .expect("write upstream last");
        }

        let child_uri = workflow_session_uri("workflow-1735689600800-child");
        append_h2_section(
            &child_uri,
            "Predecessor Context",
            "### From root-a\n\nOld [[context]] only",
            &context.config,
        )
        .expect("seed predecessor context");
        append_h2_section(
            &child_uri,
            "Step 1/1",
            "Downstream [[context]] step",
            &context.config,
        )
        .expect("seed downstream step");

        pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "root-b",
                "status": "completed",
                "conclusion": "Beta [[context]] conclusion"
            }),
        )
        .expect("complete second upstream");

        let (_, child_content, _) =
            read_markdown(&child_uri, &context.config).expect("read child session");
        assert_eq!(child_content.matches("## Predecessor Context").count(), 1);
        assert!(child_content.contains("### From root-a"));
        assert!(child_content.contains("Step 1 Response: Alpha [[context]]"));
        assert!(child_content.contains("### From root-b"));
        assert!(child_content.contains("Step 1 Response: Beta [[context]]"));
        assert!(
            !child_content.contains("Old [[context]] only"),
            "stale content should be gone"
        );
        assert!(child_content.contains("## Step 1/1"));
    }

    #[test]
    fn pipeline_restart_recreates_abandoned_node_session() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600900";
        let original_child_session = "workflow-1735689600900-child";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600900-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: original_child_session.to_string(),
                    status: NodeStatus::Abandoned,
                    attempts: 1,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let root_uri = workflow_session_uri("workflow-1735689600900-root");
        let (root_frontmatter, root_content, _) =
            read_markdown(&root_uri, &context.config).expect("read root workflow");
        let mut root_frontmatter = root_frontmatter.expect("root frontmatter");
        root_frontmatter.insert(
            "status".to_string(),
            YamlValue::String("completed".to_string()),
        );
        root_frontmatter.insert(
            "last".to_string(),
            YamlValue::String("Root finished [[handoff]]".to_string()),
        );
        write_markdown(
            &root_uri,
            Some(&root_frontmatter),
            &root_content,
            &context.config,
        )
        .expect("write root workflow");

        let child_uri = workflow_session_uri(original_child_session);
        let (child_frontmatter, child_content, _) =
            read_markdown(&child_uri, &context.config).expect("read child workflow");
        let mut child_frontmatter = child_frontmatter.expect("child frontmatter");
        child_frontmatter.insert(
            "status".to_string(),
            YamlValue::String("abandoned".to_string()),
        );
        write_markdown(
            &child_uri,
            Some(&child_frontmatter),
            &child_content,
            &context.config,
        )
        .expect("write child workflow");

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "child",
                "restart": true
            }),
        )
        .expect("restart node");
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
        let restarted_session = child
            .get("sessionId")
            .and_then(Value::as_str)
            .expect("new session id");
        assert_ne!(restarted_session, original_child_session);
        assert_eq!(child.get("status").and_then(Value::as_str), Some("pending"));
        assert_eq!(child.get("attempts").and_then(Value::as_u64), Some(2));
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("child".to_string())])
        );

        let (pipeline_frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");
        assert_eq!(updated_states["child"].session_id, restarted_session);
        assert_eq!(updated_states["child"].status, NodeStatus::Pending);
        assert_eq!(updated_states["child"].attempts, 2);

        assert!(
            file_exists(&child_uri, &context.config).expect("old session exists"),
            "old session file must remain as audit trail"
        );
        let (new_child_frontmatter, new_child_content, _) =
            read_markdown(&workflow_session_uri(restarted_session), &context.config)
                .expect("read restarted workflow");
        let new_child_frontmatter = new_child_frontmatter.expect("new child frontmatter");
        assert_eq!(
            new_child_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("pending")
        );
        assert!(
            new_child_content.contains("## Predecessor Context"),
            "restarted node should retain predecessor context"
        );
        assert!(new_child_content.contains("Root finished [[handoff]]"));
    }

    #[test]
    fn pipeline_restart_rejects_when_max_attempts_reached() {
        let (_tmp, context, _reset, _test_lock) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689601000";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689601000-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689601000-child".to_string(),
                    status: NodeStatus::Abandoned,
                    attempts: 3,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
        update_workflow_node_frontmatter(
            &context,
            "workflow-1735689601000-child",
            "abandoned",
            None,
        )
        .expect("mark child abandoned");

        let error = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "child",
                "restart": true
            }),
        )
        .expect_err("restart must fail at max attempts");

        assert_eq!(error.to_string(), "Node child has reached max attempts (3)");

        let (pipeline_frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");
        assert_eq!(updated_states["child"].status, NodeStatus::Stuck);
        assert_eq!(
            updated_states["child"].session_id,
            "workflow-1735689601000-child"
        );
        assert_eq!(updated_states["child"].attempts, 3);
    }
}
