use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    Frontmatter, delete_file, file_exists, path_to_uri, read_markdown, uri_to_path, write_markdown,
};
use crate::sessions::{
    date_parts_from_session_id, extract_session_timestamp_millis, pipeline_session_uri,
    workflow_session_uri,
};
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use serde_json::{Map as JsonMap, Value as JsonValue};
use serde_yml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;

use super::resource_notifications::{
    clear_pipeline_resource_state, emit_pipeline_resource_updated,
    emit_pipeline_sessions_resource_updated,
};
use super::session_write::DeletePipelineSessionFilesResult;
use super::{NodeState, PipelineDefinition};

pub(super) fn delete_pipeline_session_files(
    context: &ToolContext,
    pipeline_session_id: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<DeletePipelineSessionFilesResult> {
    let mut deleted_files = Vec::new();
    let mut skipped_files = 0usize;
    let mut workflow_session_ids = HashSet::with_capacity(node_states.len());

    for state in node_states.values() {
        if workflow_session_ids.insert(state.session_id.clone()) {
            let workflow_uri = workflow_session_uri(&state.session_id);
            let exists = file_exists(&workflow_uri, &context.config).unwrap_or(false);
            if !exists {
                skipped_files += 1;
                eprintln!(
                    "Warning: workflow session {} not found, skipping",
                    state.session_id
                );
                continue;
            }
            record_deleted_file(&mut deleted_files, &workflow_uri, &context.config, |err| {
                format!(
                    "Failed to delete workflow session {}: {err}",
                    state.session_id
                )
            })?;
        }
        if let Some(thinking_session_id) = &state.thinking_session_id
            && workflow_session_ids.insert(thinking_session_id.clone())
        {
            let thinking_uri = sequential_session_uri(thinking_session_id);
            let exists = file_exists(&thinking_uri, &context.config).unwrap_or(false);
            if !exists {
                skipped_files += 1;
                eprintln!(
                    "Warning: thinking session {} not found, skipping",
                    thinking_session_id
                );
                continue;
            }
            record_deleted_file(&mut deleted_files, &thinking_uri, &context.config, |err| {
                format!("Failed to delete thinking session {thinking_session_id}: {err}")
            })?;
        }
    }

    if let Some(timestamp_millis) = extract_session_timestamp_millis(pipeline_session_id) {
        let workflow_prefix = format!("workflow-{timestamp_millis}-");
        let (year, month, day) = date_parts_from_session_id(pipeline_session_id);
        let workflow_dir = context
            .config
            .memory_path
            .join("thinking")
            .join("workflow")
            .join(format!("{year:04}"))
            .join(format!("{month:02}"))
            .join(format!("{day:02}"));

        if workflow_dir.exists() {
            for entry in fs::read_dir(&workflow_dir)? {
                let entry = entry?;
                let path = entry.path();
                if !entry.file_type()?.is_file()
                    || path.extension().and_then(|ext| ext.to_str()) != Some("md")
                {
                    continue;
                }

                let session_id = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default();
                if !session_id.starts_with(&workflow_prefix)
                    || workflow_session_ids.contains(session_id)
                {
                    continue;
                }

                let workflow_uri = path_to_uri(&path, &context.config.memory_path)?;
                let (frontmatter, _, _) = read_markdown(&workflow_uri, &context.config)?;
                let belongs_to_pipeline = frontmatter
                    .as_ref()
                    .and_then(|frontmatter| frontmatter.get("pipelineId"))
                    .and_then(YamlValue::as_str)
                    == Some(pipeline_session_id);
                if !belongs_to_pipeline {
                    continue;
                }

                record_deleted_file(&mut deleted_files, &workflow_uri, &context.config, |err| {
                    format!("Failed to delete orphaned workflow session {session_id}: {err}")
                })?;
            }
        }
    }

    let pipeline_uri = pipeline_session_uri(pipeline_session_id);
    deleted_files.push(delete_file_verified(&pipeline_uri, &context.config)?);
    clear_pipeline_resource_state(pipeline_session_id);
    emit_pipeline_sessions_resource_updated();

    Ok(DeletePipelineSessionFilesResult {
        cleaned_up_files: deleted_files.len(),
        skipped_files,
        deleted_files,
    })
}

pub(super) fn delete_file_verified(uri: &str, config: &Config) -> Result<String> {
    delete_file_verified_with(uri, config, file_exists)
}

fn record_deleted_file(
    deleted_files: &mut Vec<String>,
    uri: &str,
    config: &Config,
    failure_message: impl FnOnce(MaenifoldError) -> String,
) -> Result<()> {
    match delete_file_verified(uri, config) {
        Ok(deleted_path) => {
            deleted_files.push(deleted_path);
            Ok(())
        }
        Err(err @ MaenifoldError::DeleteVerificationFailed { .. }) => Err(err),
        Err(err) => Err(MaenifoldError::Message(failure_message(err))),
    }
}

pub(super) fn delete_file_verified_with<F>(
    uri: &str,
    config: &Config,
    mut verifier: F,
) -> Result<String>
where
    F: FnMut(&str, &Config) -> Result<bool>,
{
    let path = uri_to_path(uri, &config.memory_path)?;
    let absolute_path = path.canonicalize().unwrap_or(path);
    delete_file(uri, config)?;
    if verifier(uri, config)? {
        return Err(MaenifoldError::DeleteVerificationFailed {
            path: absolute_path,
        });
    }
    Ok(absolute_path.to_string_lossy().to_string())
}

pub(super) fn write_pipeline_session(
    context: &ToolContext,
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &BTreeMap<String, YamlValue>,
    max_concurrency: u64,
    prompt: Option<&str>,
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
    frontmatter.insert("definition".to_string(), serde_yml::to_value(definition)?);
    if let Some(inputs) = definition
        .inputs
        .as_ref()
        .filter(|inputs| !inputs.is_empty())
    {
        frontmatter.insert("inputs".to_string(), serde_yml::to_value(inputs)?);
    }
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
        serde_yml::to_value(max_concurrency)?,
    );
    frontmatter.insert("first".to_string(), serde_yml::to_value(0u64)?);
    frontmatter.insert("last".to_string(), serde_yml::to_value(0u64)?);
    // T-93.4: cursor tracks completed dispatch cycles for crash-recovery continuity.
    frontmatter.insert("cursor".to_string(), serde_yml::to_value(0u64)?);
    if let Some(prompt) = prompt.map(str::trim).filter(|prompt| !prompt.is_empty()) {
        frontmatter.insert("prompt".to_string(), YamlValue::String(prompt.to_string()));
    }

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )?;
    emit_pipeline_resource_updated(pipeline_id);
    Ok(())
}

pub(super) fn persist_pipeline_runtime_context(
    context: &ToolContext,
    session_id: &str,
    prompt_override: Option<&str>,
    input_overrides: Option<&JsonMap<String, JsonValue>>,
) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) =
        super::session_read::read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition =
        super::session_read::frontmatter_value(&frontmatter, "definition")?;
    let mut inputs = super::session_read::optional_frontmatter_value::<JsonMap<String, JsonValue>>(
        &frontmatter,
        "inputs",
    )?
    .or(definition.inputs.clone())
    .unwrap_or_default();
    if let Some(overrides) = input_overrides {
        inputs.extend(overrides.clone());
    }
    if inputs.is_empty() {
        frontmatter.remove("inputs");
    } else {
        frontmatter.insert("inputs".to_string(), serde_yml::to_value(&inputs)?);
    }

    let prompt = prompt_override
        .map(str::trim)
        .filter(|prompt| !prompt.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            frontmatter
                .get("prompt")
                .and_then(YamlValue::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    if prompt.is_empty() {
        frontmatter.remove("prompt");
    } else {
        frontmatter.insert("prompt".to_string(), YamlValue::String(prompt.clone()));
    }

    write_markdown(&uri, Some(&frontmatter), &pipeline_content, &context.config)?;
    emit_pipeline_resource_updated(session_id);
    Ok(prompt)
}
