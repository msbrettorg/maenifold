use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, file_exists, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use crate::workflow::initial_workflow_frontmatter;
use chrono::Utc;
use serde_yml::Value as YamlValue;
use std::collections::HashMap;

use super::{
    NodeState, NodeStatus, PipelineDefinition, compute_pipeline_status,
    compute_pipeline_status_with_definition,
    resource_notifications::emit_pipeline_resource_updated,
};

pub(super) struct NodeFile {
    pub(super) session_id: String,
    pub(super) node_id: String,
    pub(super) workflow_id: String,
}

pub(super) struct DeletePipelineSessionFilesResult {
    pub(super) cleaned_up_files: usize,
    pub(super) skipped_files: usize,
}

pub(super) fn generate_pipeline_node_session_id(timestamp_millis: i64, node_id: &str) -> String {
    format!("workflow-{timestamp_millis}-{node_id}")
}

pub(super) fn allocate_node_session_id(context: &ToolContext, node_id: &str) -> Result<String> {
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

pub(super) fn persist_pipeline_state(
    frontmatter: &mut Frontmatter,
    node_states: &HashMap<String, NodeState>,
    uri: &str,
    pipeline_content: &str,
    context: &ToolContext,
) -> Result<()> {
    persist_pipeline_state_with_status(
        frontmatter,
        node_states,
        None,
        uri,
        pipeline_content,
        context,
    )
}

pub fn persist_final_pipeline_status(
    context: &ToolContext,
    session_id: &str,
    final_status: &str,
) -> Result<()> {
    let (uri, mut frontmatter, pipeline_content) =
        super::session_read::read_pipeline_session(context, session_id)?;
    let node_states = super::session_read::frontmatter_value::<HashMap<String, NodeState>>(
        &frontmatter,
        "nodeStates",
    )?;
    persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some(final_status),
        &uri,
        &pipeline_content,
        context,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn persist_pipeline_state_with_status(
    frontmatter: &mut Frontmatter,
    node_states: &HashMap<String, NodeState>,
    status_override: Option<&str>,
    uri: &str,
    pipeline_content: &str,
    context: &ToolContext,
) -> Result<()> {
    frontmatter.insert("nodeStates".to_string(), serde_yml::to_value(node_states)?);

    // Compute the status to persist: explicit override wins; otherwise use
    // terminal-node-aware derivation when a definition is present in frontmatter.
    let status: String = if let Some(s) = status_override {
        s.to_string()
    } else if let Some(def) = frontmatter
        .get("definition")
        .and_then(|v| serde_yml::from_value::<PipelineDefinition>(v.clone()).ok())
    {
        compute_pipeline_status_with_definition(&def, node_states).to_string()
    } else {
        compute_pipeline_status(node_states).to_string()
    };

    frontmatter.insert("status".to_string(), YamlValue::String(status));
    let session_id = frontmatter
        .get("permalink")
        .and_then(YamlValue::as_str)
        .map(str::to_owned);
    write_markdown(uri, Some(frontmatter), pipeline_content, &context.config)?;
    if let Some(session_id) = session_id {
        emit_pipeline_resource_updated(&session_id);
    }
    Ok(())
}

pub(super) fn update_workflow_node_frontmatter(
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

pub(super) fn node_status_name(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "pending",
        NodeStatus::Ready => "ready",
        NodeStatus::Active => "active",
        NodeStatus::Completed => "completed",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Stuck => "stuck",
        NodeStatus::Retrying => "retrying",
    }
}

pub(super) fn write_workflow_node_session(
    context: &ToolContext,
    pipeline_id: &str,
    node_file: &NodeFile,
) -> Result<()> {
    write_workflow_node_session_with_status(context, pipeline_id, node_file, "pending")
}

pub(super) fn write_workflow_node_session_with_status(
    context: &ToolContext,
    pipeline_id: &str,
    node_file: &NodeFile,
    status: &str,
) -> Result<()> {
    let uri = workflow_session_uri(&node_file.session_id);
    let mut frontmatter = initial_workflow_frontmatter(
        &node_file.session_id,
        std::slice::from_ref(&node_file.workflow_id),
        status,
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
