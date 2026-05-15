use crate::config::Config;
use crate::error::{MaenifoldError, Result};
#[cfg(test)]
use crate::markdown::delete_file;
use crate::markdown::{file_exists, path_to_uri, read_markdown, uri_to_path};
use crate::sessions::{
    date_parts_from_session_id, extract_session_timestamp_millis, pipeline_session_uri,
    workflow_session_uri,
};
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use serde_yml::Value as YamlValue;
use std::collections::{HashMap, HashSet};
use std::fs;

use super::NodeState;
#[cfg(test)]
pub use super::quarantine::{restore_pipeline_from_quarantine, run_quarantine_cleanup};
use super::resource_notifications::{
    clear_pipeline_resource_state, emit_pipeline_sessions_resource_updated,
};
use super::session_write::DeletePipelineSessionFilesResult;

pub(super) fn resolve_pipeline_delete_files(
    context: &ToolContext,
    pipeline_session_id: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<Vec<String>> {
    let mut files = Vec::new();
    let mut workflow_session_ids = HashSet::with_capacity(node_states.len());

    for state in node_states.values() {
        if workflow_session_ids.insert(state.session_id.clone()) {
            let workflow_uri = workflow_session_uri(&state.session_id);
            if file_exists(&workflow_uri, &context.config).unwrap_or(false) {
                files.push(resolve_uri_absolute_path(&workflow_uri, &context.config)?);
            }
        }
        if let Some(thinking_session_id) = &state.thinking_session_id
            && workflow_session_ids.insert(thinking_session_id.clone())
        {
            let thinking_uri = sequential_session_uri(thinking_session_id);
            if file_exists(&thinking_uri, &context.config).unwrap_or(false) {
                files.push(resolve_uri_absolute_path(&thinking_uri, &context.config)?);
            }
        }
    }

    collect_orphaned_workflows(
        context,
        pipeline_session_id,
        &workflow_session_ids,
        &mut files,
    )?;
    files.push(resolve_uri_absolute_path(
        &pipeline_session_uri(pipeline_session_id),
        &context.config,
    )?);
    Ok(files)
}

pub(super) fn resolve_unreadable_pipeline_delete_files(
    context: &ToolContext,
    pipeline_session_id: &str,
) -> Result<Vec<String>> {
    Ok(vec![resolve_uri_absolute_path(
        &pipeline_session_uri(pipeline_session_id),
        &context.config,
    )?])
}

pub(super) fn delete_pipeline_session_files(
    context: &ToolContext,
    pipeline_session_id: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<DeletePipelineSessionFilesResult> {
    if context.config.settings.quarantine_retention_days.is_none() {
        return Err(MaenifoldError::Message(
            "quarantine retention not configured".to_string(),
        ));
    }
    let quarantine = super::quarantine::require_quarantine_config(&context.config)?;
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
            record_deleted_file(
                &mut deleted_files,
                &workflow_uri,
                &context.config,
                pipeline_session_id,
                |err| {
                    format!(
                        "Failed to delete workflow session {}: {err}",
                        state.session_id
                    )
                },
            )?;
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
            record_deleted_file(
                &mut deleted_files,
                &thinking_uri,
                &context.config,
                pipeline_session_id,
                |err| format!("Failed to delete thinking session {thinking_session_id}: {err}"),
            )?;
        }
    }

    delete_orphaned_workflows(
        context,
        pipeline_session_id,
        &workflow_session_ids,
        &mut deleted_files,
    )?;

    let pipeline_uri = pipeline_session_uri(pipeline_session_id);
    deleted_files.push(super::quarantine::quarantine_file(
        &pipeline_uri,
        &context.config,
        pipeline_session_id,
    )?);
    clear_pipeline_resource_state(pipeline_session_id);
    emit_pipeline_sessions_resource_updated();

    Ok(DeletePipelineSessionFilesResult {
        cleaned_up_files: deleted_files.len(),
        skipped_files,
        deleted_files,
        retention_days: quarantine.retention_days,
        defaulted_retention: context.config.settings.quarantine_retention_days.is_none(),
    })
}

pub(super) fn delete_pipeline_session_files_with_default_retention(
    context: &ToolContext,
    pipeline_session_id: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<DeletePipelineSessionFilesResult> {
    if context.config.settings.quarantine_retention_days.is_some() {
        return delete_pipeline_session_files(context, pipeline_session_id, node_states);
    }

    let mut defaulted_config = context.config.clone();
    defaulted_config.settings.quarantine_retention_days =
        Some(Config::default_quarantine_retention_days());
    let defaulted_context = ToolContext {
        config: defaulted_config,
        output_format: context.output_format,
        stdio_pool: context.stdio_pool.clone(),
        cancellation_token: context.cancellation_token.clone(),
    };
    let mut result =
        delete_pipeline_session_files(&defaulted_context, pipeline_session_id, node_states)?;
    result.defaulted_retention = true;
    Ok(result)
}

#[cfg(test)]
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

fn collect_orphaned_workflows(
    context: &ToolContext,
    pipeline_session_id: &str,
    workflow_session_ids: &HashSet<String>,
    files: &mut Vec<String>,
) -> Result<()> {
    for (session_id, workflow_uri) in
        orphaned_workflows(context, pipeline_session_id, workflow_session_ids)?
    {
        let _ = session_id;
        files.push(resolve_uri_absolute_path(&workflow_uri, &context.config)?);
    }
    Ok(())
}

fn delete_orphaned_workflows(
    context: &ToolContext,
    pipeline_session_id: &str,
    workflow_session_ids: &HashSet<String>,
    deleted_files: &mut Vec<String>,
) -> Result<()> {
    for (session_id, workflow_uri) in
        orphaned_workflows(context, pipeline_session_id, workflow_session_ids)?
    {
        record_deleted_file(
            deleted_files,
            &workflow_uri,
            &context.config,
            pipeline_session_id,
            |err| format!("Failed to delete orphaned workflow session {session_id}: {err}"),
        )?;
    }
    Ok(())
}

fn orphaned_workflows(
    context: &ToolContext,
    pipeline_session_id: &str,
    workflow_session_ids: &HashSet<String>,
) -> Result<Vec<(String, String)>> {
    let Some(timestamp_millis) = extract_session_timestamp_millis(pipeline_session_id) else {
        return Ok(Vec::new());
    };
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

    if !workflow_dir.exists() {
        return Ok(Vec::new());
    }

    let mut workflows = Vec::new();
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
        if !session_id.starts_with(&workflow_prefix) || workflow_session_ids.contains(session_id) {
            continue;
        }
        let workflow_uri = path_to_uri(&path, &context.config.memory_path)?;
        let (frontmatter, _, _) = read_markdown(&workflow_uri, &context.config)?;
        let belongs_to_pipeline = frontmatter
            .as_ref()
            .and_then(|frontmatter| frontmatter.get("pipelineId"))
            .and_then(YamlValue::as_str)
            == Some(pipeline_session_id);
        if belongs_to_pipeline {
            workflows.push((session_id.to_string(), workflow_uri));
        }
    }
    Ok(workflows)
}

fn record_deleted_file(
    deleted_files: &mut Vec<String>,
    uri: &str,
    config: &Config,
    pipeline_session_id: &str,
    failure_message: impl FnOnce(MaenifoldError) -> String,
) -> Result<()> {
    match super::quarantine::quarantine_file(uri, config, pipeline_session_id) {
        Ok(deleted_path) => {
            deleted_files.push(deleted_path);
            Ok(())
        }
        Err(err @ MaenifoldError::DeleteVerificationFailed { .. }) => Err(err),
        Err(err) => Err(MaenifoldError::Message(failure_message(err))),
    }
}

fn resolve_uri_absolute_path(uri: &str, config: &Config) -> Result<String> {
    let path = uri_to_path(uri, &config.memory_path)?;
    let absolute_path = match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => path,
    };
    Ok(absolute_path.to_string_lossy().to_string())
}
