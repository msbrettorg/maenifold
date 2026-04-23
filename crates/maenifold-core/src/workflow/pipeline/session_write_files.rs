use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    Frontmatter, delete_file, file_exists, path_to_uri, read_markdown, write_markdown,
};
use crate::sessions::{
    date_parts_from_session_id, extract_session_timestamp_millis, pipeline_session_uri,
    workflow_session_uri,
};
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use serde_json::{Map as JsonMap, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;

use super::session_write::DeletePipelineSessionFilesResult;
use super::{NodeState, PipelineDefinition};

pub(super) fn delete_pipeline_session_files(
    context: &ToolContext,
    pipeline_session_id: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<DeletePipelineSessionFilesResult> {
    let mut cleaned_up_files = 0usize;
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
            match delete_file(&workflow_uri, &context.config) {
                Ok(()) => cleaned_up_files += 1,
                Err(err) => {
                    return Err(MaenifoldError::Message(format!(
                        "Failed to delete workflow session {}: {}",
                        state.session_id, err
                    )));
                }
            }
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
            match delete_file(&thinking_uri, &context.config) {
                Ok(()) => cleaned_up_files += 1,
                Err(err) => {
                    return Err(MaenifoldError::Message(format!(
                        "Failed to delete thinking session {}: {}",
                        thinking_session_id, err
                    )));
                }
            }
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

                match delete_file(&workflow_uri, &context.config) {
                    Ok(()) => cleaned_up_files += 1,
                    Err(err) => {
                        return Err(MaenifoldError::Message(format!(
                            "Failed to delete orphaned workflow session {}: {}",
                            session_id, err
                        )));
                    }
                }
            }
        }
    }

    delete_file(&pipeline_session_uri(pipeline_session_id), &context.config)?;
    cleaned_up_files += 1;

    Ok(DeletePipelineSessionFilesResult {
        cleaned_up_files,
        skipped_files,
    })
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
    frontmatter.insert("definition".to_string(), serde_yaml::to_value(definition)?);
    if let Some(inputs) = definition
        .inputs
        .as_ref()
        .filter(|inputs| !inputs.is_empty())
    {
        frontmatter.insert("inputs".to_string(), serde_yaml::to_value(inputs)?);
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
        serde_yaml::to_value(max_concurrency)?,
    );
    frontmatter.insert("first".to_string(), serde_yaml::to_value(0u64)?);
    frontmatter.insert("last".to_string(), serde_yaml::to_value(0u64)?);
    // T-93.4: cursor tracks completed dispatch cycles for crash-recovery continuity.
    frontmatter.insert("cursor".to_string(), serde_yaml::to_value(0u64)?);
    if let Some(prompt) = prompt.map(str::trim).filter(|prompt| !prompt.is_empty()) {
        frontmatter.insert("prompt".to_string(), YamlValue::String(prompt.to_string()));
    }

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
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
        frontmatter.insert("inputs".to_string(), serde_yaml::to_value(&inputs)?);
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
    Ok(prompt)
}
