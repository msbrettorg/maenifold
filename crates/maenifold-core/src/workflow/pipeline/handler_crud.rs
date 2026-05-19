use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use serde_json::{Map as JsonMap, Value, json};
use std::collections::{BTreeMap, HashMap};

use super::handler_validation::{validate_workflow_node_definitions, workflow_node_file};
#[cfg(test)]
use super::lease;
use super::resource_notifications::emit_pipeline_resource_updated;
#[cfg(test)]
use super::session_io::{frontmatter_value, read_pipeline_session};
use super::session_io::{
    generate_pipeline_node_session_id, write_pipeline_session, write_workflow_node_session,
};
use super::view::{view_pipeline_session, view_pipeline_summary_session};
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

    let max_concurrency = parse_max_concurrency(payload)?;
    let prefix = parse_pipeline_id_prefix(payload)?;
    let name = parse_optional_text(payload, "name")?;
    let description = parse_optional_text(payload, "description")?;
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
            output_artifact: None,
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
    {
        // asset-store lock — held across validate+write, NOT across resource notification,
        // NOT while holding a pipeline lease.
        let _guard = crate::workflow::asset_store::lock_asset_store()?;
        validate_workflow_node_definitions(context, &definition.nodes)?;
        write_pipeline_session(
            context,
            &pipeline_id,
            &definition,
            &node_states_frontmatter,
            max_concurrency,
            None,
            name.as_deref(),
            description.as_deref(),
        )?;
    }
    for node_file in &node_files {
        write_workflow_node_session(context, &pipeline_id, node_file)?;
    }
    emit_pipeline_resource_updated(&pipeline_id);

    let result = json!({
        "pipelineId": pipeline_id,
        "ready": ready,
        "nodes": response_nodes,
    });
    let mut result = result;
    if let Some(name) = name {
        result["name"] = json!(name);
    }
    if let Some(description) = description {
        result["description"] = json!(description);
    }
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}
pub(super) fn list_pipelines(context: &ToolContext) -> Result<String> {
    super::handler_list_lineage::list_pipelines(context)
}

#[cfg(test)]
fn list_pipelines_impl(
    context: &ToolContext,
    read_markdown_fn: fn(
        &str,
        &crate::config::Config,
    ) -> Result<(Option<crate::markdown::Frontmatter>, String, String)>,
) -> Result<String> {
    super::handler_list_lineage::list_pipelines_impl(context, read_markdown_fn)
}
#[cfg(test)]
pub(super) use super::handler_delete::delete_unreadable_pipeline;
pub(super) use super::handler_delete::{delete_pipeline, delete_pipeline_with_force};
pub(super) fn view_pipeline(
    context: &ToolContext,
    session_id: &str,
    summary: bool,
) -> Result<String> {
    if summary {
        view_pipeline_summary(context, session_id)
    } else {
        view_pipeline_session(context, session_id)
    }
}

pub(super) fn view_pipeline_summary(context: &ToolContext, session_id: &str) -> Result<String> {
    view_pipeline_summary_session(context, session_id)
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

fn parse_optional_text(payload: &Value, field: &str) -> Result<Option<String>> {
    match payload.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
        }
        Some(_) => Err(MaenifoldError::InvalidArgument(format!(
            "{field} must be a string"
        ))),
    }
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
