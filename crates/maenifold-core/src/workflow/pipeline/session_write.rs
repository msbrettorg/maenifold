use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, delete_file, file_exists, read_markdown, write_markdown};
use crate::sessions::{date_parts_from_session_id, pipeline_session_uri, workflow_session_uri};
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use crate::workflow::initial_workflow_frontmatter;
use chrono::Utc;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;

use super::{NodeState, NodeStatus, PipelineDefinition, compute_pipeline_status};

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

#[allow(clippy::too_many_arguments)]
pub(super) fn persist_pipeline_state_with_status(
    frontmatter: &mut Frontmatter,
    node_states: &HashMap<String, NodeState>,
    status_override: Option<&str>,
    uri: &str,
    pipeline_content: &str,
    context: &ToolContext,
) -> Result<()> {
    frontmatter.insert("nodeStates".to_string(), serde_yaml::to_value(node_states)?);
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String(
            status_override
                .unwrap_or_else(|| compute_pipeline_status(node_states))
                .to_string(),
        ),
    );
    write_markdown(uri, Some(frontmatter), pipeline_content, &context.config)
}

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
        if let Some(thinking_session_id) = &state.thinking_session_id {
            if workflow_session_ids.insert(thinking_session_id.clone()) {
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
    }

    delete_file(&pipeline_session_uri(pipeline_session_id), &context.config)?;
    cleaned_up_files += 1;

    Ok(DeletePipelineSessionFilesResult {
        cleaned_up_files,
        skipped_files,
    })
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
    }
}

pub(super) fn write_pipeline_session(
    context: &ToolContext,
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &BTreeMap<String, YamlValue>,
    max_concurrency: u64,
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

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
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
