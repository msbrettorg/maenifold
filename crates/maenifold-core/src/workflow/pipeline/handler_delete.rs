use crate::error::{MaenifoldError, Result};
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use serde_json::json;
use serde_yml::Value as YamlValue;
use std::collections::HashMap;

use super::audit::{AuditWriter, JsonlAuditWriter, PipelineDeleteAuditRecord};
use super::lease;
use super::quarantine::quarantine_file;
use super::resource_notifications::{
    clear_pipeline_resource_state, emit_pipeline_sessions_resource_updated,
};
use super::session_delete_files::{
    resolve_pipeline_delete_files, resolve_unreadable_pipeline_delete_files,
};
use super::session_io::{
    delete_pipeline_session_files_with_default_retention, derive_pipeline_node_states,
    frontmatter_value, read_pipeline_session,
};
use super::{NodeState, NodeStatus};

pub(super) fn delete_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    delete_pipeline_with_force(context, session_id, false)
}

pub(super) fn delete_pipeline_with_force(
    context: &ToolContext,
    session_id: &str,
    force: bool,
) -> Result<String> {
    lease::with_exclusive_session_lock(
        session_id,
        &context.config,
        || {
            MaenifoldError::InvalidArgument(format!(
                "Pipeline {session_id} is running; cancel it before deleting"
            ))
        },
        || {
            let (_, frontmatter, _) = match read_pipeline_session(context, session_id) {
                Ok(session) => session,
                Err(error @ MaenifoldError::FileDataless { .. }) => {
                    return delete_unreadable_pipeline(context, session_id, force, error);
                }
                Err(error) => return Err(error),
            };
            if !force && frontmatter.get("status").and_then(YamlValue::as_str) == Some("completed")
            {
                return Err(MaenifoldError::InvalidArgument(format!(
                    "Pipeline {session_id} is completed; pass force:true to delete completed pipelines"
                )));
            }
            let node_states = delete_node_states(context, session_id, &frontmatter);
            if !force
                && node_states
                    .values()
                    .any(|state| matches!(state.status, NodeStatus::Active | NodeStatus::Retrying))
            {
                return Err(MaenifoldError::InvalidArgument(format!(
                    "Pipeline {session_id} has active work; cancel it before deleting"
                )));
            }
            let files = resolve_pipeline_delete_files(context, session_id, &node_states)?;
            audit_pipeline_delete(context, session_id, force, files)?;
            let deleted_files = delete_pipeline_session_files_with_default_retention(
                context,
                session_id,
                &node_states,
            )?;
            render_delete_result(context, session_id, deleted_files, None)
        },
    )
}

pub(super) fn delete_unreadable_pipeline(
    context: &ToolContext,
    session_id: &str,
    force: bool,
    read_error: MaenifoldError,
) -> Result<String> {
    if !force {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is unreadable ({read_error}); pass force:true to delete unreadable pipelines"
        )));
    }
    let files = resolve_unreadable_pipeline_delete_files(context, session_id)?;
    audit_pipeline_delete(context, session_id, force, files)?;
    let pipeline_uri = pipeline_session_uri(session_id);
    if context.config.settings.quarantine_retention_days.is_none() {
        return Err(MaenifoldError::Message(
            "quarantine retention not configured".to_string(),
        ));
    }
    let quarantine = context.config.quarantine_config().ok_or_else(|| {
        MaenifoldError::Message("quarantine retention not configured".to_string())
    })?;
    let deleted_file = quarantine_file(&pipeline_uri, &context.config, session_id)?;
    clear_pipeline_resource_state(session_id);
    emit_pipeline_sessions_resource_updated();
    let degraded = json!({
        "reason": "dataless",
        "nodeCleanup": "skipped_unreadable_parent",
        "readError": read_error.to_string(),
    });
    render_delete_result(
        context,
        session_id,
        super::session_write::DeletePipelineSessionFilesResult {
            cleaned_up_files: 1,
            skipped_files: 0,
            deleted_files: vec![deleted_file],
            retention_days: quarantine.retention_days,
            defaulted_retention: context.config.settings.quarantine_retention_days.is_none(),
        },
        Some(degraded),
    )
}

fn delete_node_states(
    context: &ToolContext,
    session_id: &str,
    frontmatter: &crate::markdown::Frontmatter,
) -> HashMap<String, NodeState> {
    match frontmatter_value(frontmatter, "nodeStates") {
        Ok(raw_node_states) => match derive_pipeline_node_states(context, &raw_node_states) {
            Ok(derived_node_states) => derived_node_states,
            Err(err) => {
                eprintln!(
                    "Warning: failed to derive node states for pipeline {session_id}; \
                     falling back to raw frontmatter nodeStates: {err}"
                );
                raw_node_states
            }
        },
        Err(err) => {
            eprintln!(
                "Warning: failed to read raw nodeStates for corrupted pipeline \
                 {session_id}; deleting pipeline without node cleanup metadata: {err}"
            );
            HashMap::new()
        }
    }
}

fn audit_pipeline_delete(
    context: &ToolContext,
    session_id: &str,
    force: bool,
    files: Vec<String>,
) -> Result<()> {
    JsonlAuditWriter::new(&context.config).write_pipeline_delete(&PipelineDeleteAuditRecord {
        op: "pipeline_delete",
        session_id: session_id.to_string(),
        force,
        files,
        requester_agent: "unknown".to_string(),
        requester_pid: std::process::id(),
        ts: Utc::now().to_rfc3339(),
    })
}

fn render_delete_result(
    context: &ToolContext,
    session_id: &str,
    deleted_files: super::session_write::DeletePipelineSessionFilesResult,
    degraded: Option<serde_json::Value>,
) -> Result<String> {
    let mut result = json!({
        "deletedSessionId": session_id,
        "cleanedUpFiles": deleted_files.cleaned_up_files,
        "skippedFiles": deleted_files.skipped_files,
        "deletedFiles": deleted_files.deleted_files,
        "retentionDays": deleted_files.retention_days,
        "defaultedRetention": deleted_files.defaulted_retention,
        "message": format!(
            "Pipeline moved to quarantine for {} days.",
            deleted_files.retention_days
        ),
    });
    if let Some(degraded) = degraded {
        result["degraded"] = degraded;
    }
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}
