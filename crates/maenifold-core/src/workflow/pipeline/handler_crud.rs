use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

use super::handler_validation::{validate_workflow_node_definitions, workflow_node_file};
use super::session_io::{
    delete_pipeline_session_files, frontmatter_value, generate_pipeline_node_session_id,
    read_pipeline_session, write_pipeline_session, write_workflow_node_session,
};
use super::view::view_pipeline_session;
use super::{NodeState, NodeStatus, PipelineDefinition, compute_ready_set, validate_pipeline};

pub(super) fn create_pipeline(context: &ToolContext, payload: &Value) -> Result<String> {
    let Some(definition_value) = payload.get("definition") else {
        return Err(MaenifoldError::Message(
            "Pipeline definition is required".to_string(),
        ));
    };

    AssetManager::initialize_assets(&context.config)?;

    let definition: PipelineDefinition = serde_json::from_value(definition_value.clone())?;
    validate_pipeline(&definition)?;
    validate_workflow_node_definitions(context, &definition.nodes)?;

    let max_concurrency = parse_max_concurrency(payload)?;
    let timestamp_millis = Utc::now().timestamp_millis();
    let pipeline_id = format!("pipeline-{timestamp_millis}");
    let mut node_states = HashMap::with_capacity(definition.nodes.len());
    let mut node_states_frontmatter = BTreeMap::new();
    let mut node_files = Vec::with_capacity(definition.nodes.len());
    let mut response_nodes = Vec::with_capacity(definition.nodes.len());

    for node in &definition.nodes {
        let session_id = generate_pipeline_node_session_id(timestamp_millis, node.id());
        let state = NodeState {
            session_id: session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        };
        node_states.insert(node.id().to_string(), state.clone());
        node_states_frontmatter.insert(node.id().to_string(), serde_yaml::to_value(&state)?);
        node_files.push(workflow_node_file(node, session_id.clone()));
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

#[allow(clippy::too_many_lines)]
pub(super) fn list_pipelines(context: &ToolContext) -> Result<String> {
    let pipelines_dir = context.config.memory_path.join("pipelines");
    let mut pipelines = Vec::new();

    if pipelines_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&pipelines_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }

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
                .and_then(|value| serde_yaml::from_value::<PipelineDefinition>(value.clone()).ok())
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

pub(super) fn delete_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let node_states: HashMap<String, NodeState> = frontmatter_value(&frontmatter, "nodeStates")?;
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

pub(super) fn view_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    view_pipeline_session(context, session_id)
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
