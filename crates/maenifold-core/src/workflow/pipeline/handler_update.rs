use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::write_markdown;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use serde_yml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::handler_validation::{validate_workflow_node_definitions, workflow_node_file};
use super::lease;
use super::resource_notifications::emit_pipeline_resource_updated;
use super::session_io::{
    NodeFile, allocate_node_session_id, derive_pipeline_node_states, frontmatter_value,
    pipeline_definition_from_frontmatter, read_pipeline_session, write_workflow_node_session,
};
use super::{NodeState, NodeStatus, PipelineDefinition, compute_ready_set, validate_pipeline};

pub(super) fn update_pipeline(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<String> {
    let definition_value = payload
        .get("definition")
        .ok_or(MaenifoldError::MissingProperty("definition"))?;
    AssetManager::initialize_assets(&context.config)?;
    let definition: PipelineDefinition = serde_json::from_value(definition_value.clone())?;
    validate_pipeline(&definition)?;

    lease::with_exclusive_session_lock(
        session_id,
        &context.config,
        || {
            MaenifoldError::InvalidArgument(
                "pipeline is running; cancel before updating".to_string(),
            )
        },
        || update_pipeline_locked(context, session_id, payload, definition),
    )
}

fn update_pipeline_locked(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
    definition: PipelineDefinition,
) -> Result<String> {
    let (uri, mut frontmatter, content) = read_pipeline_session(context, session_id)?;
    let existing_definition = pipeline_definition_from_frontmatter(&frontmatter)?;
    let existing_states: HashMap<String, NodeState> =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    if existing_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Active | NodeStatus::Retrying))
    {
        return Err(MaenifoldError::InvalidArgument(
            "pipeline is running; cancel before updating".to_string(),
        ));
    }

    let (node_states, new_node_files) =
        merge_node_states(context, &definition, &existing_definition, &existing_states)?;
    let ready = compute_ready_set(&definition, &node_states);
    let name = optional_text(payload, "name")?.or_else(|| existing_text(&frontmatter, "name"));
    let description = optional_text(payload, "description")?
        .or_else(|| existing_text(&frontmatter, "description"));

    {
        let _guard = crate::workflow::asset_store::lock_asset_store()?;
        validate_workflow_node_definitions(context, &definition.nodes)?;
        let node_states_frontmatter = states_to_frontmatter(&node_states)?;
        write_updated_pipeline_session(
            context,
            &uri,
            &mut frontmatter,
            &content,
            &definition,
            &node_states_frontmatter,
            name.as_deref(),
            description.as_deref(),
        )?;
        for node_file in &new_node_files {
            write_workflow_node_session(context, session_id, node_file)?;
        }
    }
    emit_pipeline_resource_updated(session_id);

    let result = json!({
        "pipelineId": session_id,
        "ready": ready,
    });
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn merge_node_states(
    context: &ToolContext,
    definition: &PipelineDefinition,
    existing_definition: &PipelineDefinition,
    existing_states: &HashMap<String, NodeState>,
) -> Result<(HashMap<String, NodeState>, Vec<NodeFile>)> {
    let mut seen = HashSet::with_capacity(definition.nodes.len());
    let mut node_states = HashMap::with_capacity(definition.nodes.len());
    let mut new_node_files = Vec::new();
    let existing_node_types: HashMap<&str, &str> = existing_definition
        .nodes
        .iter()
        .map(|node| (node.id(), node.node_type_name()))
        .collect();

    for node in &definition.nodes {
        if !seen.insert(node.id()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "duplicate node ID '{}'",
                node.id()
            )));
        }
        if let Some(state) = existing_states.get(node.id()) {
            if existing_node_types
                .get(node.id())
                .is_some_and(|old_type| *old_type == node.node_type_name())
            {
                node_states.insert(node.id().to_string(), state.clone());
                continue;
            }
        }
        let session_id = allocate_node_session_id(context, node.id())?;
        node_states.insert(node.id().to_string(), pending_state(session_id.clone()));
        new_node_files.push(workflow_node_file(node, session_id));
    }

    Ok((node_states, new_node_files))
}

fn write_updated_pipeline_session(
    context: &ToolContext,
    uri: &str,
    frontmatter: &mut crate::markdown::Frontmatter,
    content: &str,
    definition: &PipelineDefinition,
    node_states: &BTreeMap<String, YamlValue>,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    upsert_text(frontmatter, "name", name);
    upsert_text(frontmatter, "description", description);
    frontmatter.insert("definition".to_string(), serde_yml::to_value(definition)?);
    frontmatter.insert(
        "nodeStates".to_string(),
        YamlValue::Mapping(
            node_states
                .iter()
                .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                .collect(),
        ),
    );
    write_markdown(uri, Some(frontmatter), content, &context.config)?;
    Ok(())
}

fn upsert_text(frontmatter: &mut crate::markdown::Frontmatter, field: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        frontmatter.insert(field.to_string(), YamlValue::String(value.to_string()));
    } else {
        frontmatter.remove(field);
    }
}

fn pending_state(session_id: String) -> NodeState {
    NodeState {
        session_id,
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
    }
}

fn states_to_frontmatter(
    node_states: &HashMap<String, NodeState>,
) -> Result<BTreeMap<String, YamlValue>> {
    node_states
        .iter()
        .map(|(key, value)| Ok((key.clone(), serde_yml::to_value(value)?)))
        .collect()
}

fn optional_text(payload: &Value, field: &str) -> Result<Option<String>> {
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

fn existing_text(frontmatter: &crate::markdown::Frontmatter, field: &str) -> Option<String> {
    frontmatter
        .get(field)
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
