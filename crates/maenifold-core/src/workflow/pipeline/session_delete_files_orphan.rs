use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::markdown::{path_to_uri, read_markdown, uri_to_path};
use crate::sessions::{date_parts_from_session_id, extract_session_timestamp_millis};
use crate::tools::ToolContext;
use serde_yml::Value as YamlValue;
use std::{collections::HashSet, fs};

pub(super) fn collect_orphaned_workflows(
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

pub(super) fn delete_orphaned_workflows(
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

pub(super) fn record_deleted_file(
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

pub(super) fn resolve_uri_absolute_path(uri: &str, config: &Config) -> Result<String> {
    let path = uri_to_path(uri, &config.memory_path)?;
    let absolute_path = match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => path,
    };
    Ok(absolute_path.to_string_lossy().to_string())
}
