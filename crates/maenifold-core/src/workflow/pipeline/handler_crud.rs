use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use serde_json::{Map as JsonMap, Value, json};
use serde_yml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

use super::engine_runs::compute_pipeline_runs;
use super::engine_status::{
    completed_node_count, definition_node_count, derive_pipeline_list_status, pipeline_node_states,
};
use super::handler_validation::{validate_workflow_node_definitions, workflow_node_file};
#[cfg(test)]
use super::lease;
#[cfg(test)]
use super::session_io::{frontmatter_value, read_pipeline_session};
use super::session_io::{
    generate_pipeline_node_session_id, write_pipeline_session, write_workflow_node_session,
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
    let mut definition: PipelineDefinition = serde_json::from_value(definition_value.clone())?;
    let payload_inputs = parse_inputs_payload(payload.get("inputs"))?;
    definition.inputs = merge_pipeline_inputs(definition.inputs.take(), payload_inputs);
    validate_pipeline(&definition)?;
    validate_workflow_node_definitions(context, &definition.nodes)?;

    let max_concurrency = parse_max_concurrency(payload)?;
    let prefix = parse_pipeline_id_prefix(payload)?;
    let timestamp_millis = Utc::now().timestamp_millis();
    let pipeline_id = format!("{prefix}-{timestamp_millis}");
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
        None,
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
pub(super) fn list_pipelines(context: &ToolContext) -> Result<String> {
    list_pipelines_impl(context, read_markdown)
}
fn list_pipelines_impl(
    context: &ToolContext,
    read_markdown_fn: fn(
        &str,
        &crate::config::Config,
    ) -> Result<(Option<crate::markdown::Frontmatter>, String, String)>,
) -> Result<String> {
    let pipelines_dir = context.config.memory_path.join("pipelines");
    let mut pipelines = Vec::new();
    crate::db::GraphDatabase::initialize_database(&context.config)?;
    let connection = crate::db::open_connection(&context.config.database_path)?;
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
            match read_markdown_fn(&uri, &context.config) {
                Ok((Some(frontmatter), _, _)) => {
                    if frontmatter.get("type").and_then(YamlValue::as_str) != Some("pipeline") {
                        continue;
                    }
                    let node_states = pipeline_node_states(&frontmatter);
                    let status = derive_pipeline_list_status(
                        session_id,
                        frontmatter.get("status").and_then(YamlValue::as_str),
                        &node_states,
                        &context.config,
                    );
                    pipelines.push(json!({
                        "pipelineId": frontmatter.get("permalink").and_then(YamlValue::as_str).unwrap_or(session_id),
                        "status": status,
                        "nodeCount": definition_node_count(frontmatter.get("definition")),
                        "completedCount": completed_node_count(&node_states),
                        "runs": compute_pipeline_runs(&node_states),
                    }));
                }
                Ok((None, _, _)) => {
                    if let Some(entry) = crate::db::lookup_pipeline_degraded_json(
                        &connection,
                        &uri,
                        "markdown_unreadable",
                        "pipeline markdown had no frontmatter",
                    )? {
                        pipelines.push(entry);
                    }
                }
                Err(error) => {
                    let reason = if matches!(error, MaenifoldError::FileDataless { .. }) {
                        "dataless"
                    } else {
                        "markdown_unreadable"
                    };
                    if let Some(entry) = crate::db::lookup_pipeline_degraded_json(
                        &connection,
                        &uri,
                        reason,
                        &error.to_string(),
                    )? {
                        pipelines.push(entry);
                    }
                }
            }
        }
    }
    pipelines.sort_by(|a, b| {
        a.get("pipelineId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("pipelineId").and_then(Value::as_str).unwrap_or(""))
    });
    let result = json!({ "pipelines": pipelines });
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}
#[cfg(test)]
pub(super) use super::handler_delete::delete_unreadable_pipeline;
pub(super) use super::handler_delete::{delete_pipeline, delete_pipeline_with_force};
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

fn parse_pipeline_id_prefix(payload: &Value) -> Result<&str> {
    let prefix = match payload.get("name") {
        None | Some(Value::Null) => return Ok("pipeline"),
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok("pipeline");
            }
            trimmed
        }
        Some(_) => {
            return Err(MaenifoldError::InvalidArgument(
                "name must be a string".to_string(),
            ));
        }
    };

    let mut chars = prefix.chars();
    let Some(first) = chars.next() else {
        return Ok("pipeline");
    };
    if !first.is_ascii_alphabetic() || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        return Err(MaenifoldError::InvalidArgument(
            "name must start with a letter and contain only alphanumeric characters or hyphens"
                .to_string(),
        ));
    }
    if prefix.len() > 64 {
        return Err(MaenifoldError::InvalidArgument(
            "name must be 64 characters or fewer".to_string(),
        ));
    }

    Ok(prefix)
}

fn parse_inputs_payload(value: Option<&Value>) -> Result<Option<JsonMap<String, Value>>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(map)) => Ok(Some(map.clone())),
        Some(_) => Err(MaenifoldError::InvalidArgument(
            "inputs must be a JSON object".to_string(),
        )),
    }
}

fn merge_pipeline_inputs(
    existing: Option<JsonMap<String, Value>>,
    overrides: Option<JsonMap<String, Value>>,
) -> Option<JsonMap<String, Value>> {
    let mut merged = existing.unwrap_or_default();
    if let Some(overrides) = overrides {
        merged.extend(overrides);
    }
    (!merged.is_empty()).then_some(merged)
}

#[cfg(test)]
#[path = "handler_crud_tests.rs"]
mod tests;
