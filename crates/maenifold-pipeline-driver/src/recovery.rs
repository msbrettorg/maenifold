use chrono::{Datelike, SecondsFormat, TimeZone, Utc};
use maenifold_core::markdown::{read_markdown, write_markdown};
use maenifold_core::tools::ToolContext;
use maenifold_core::workflow::pipeline::{
    NodeState, NodeStatus, PipelineDefinition, compute_pipeline_status,
    compute_pipeline_status_with_definition, recover_crashed_nodes,
};
use maenifold_core::{MaenifoldError, Result};
use serde_yml::Value as YamlValue;
use std::collections::HashMap;
use tracing::warn;

#[derive(Debug, Default)]
pub(crate) struct DriverRecovery {
    pub(crate) reset_to_pending: usize,
    pub(crate) recovered: usize,
}

enum DriverStartedAtState {
    Missing,
    Present,
    Malformed,
}

pub(crate) fn reconcile_driver_start(
    context: &ToolContext,
    session_id: &str,
) -> Result<DriverRecovery> {
    let state = read_driver_started_at_state(context, session_id)?;
    let mut recovery = DriverRecovery::default();

    match state {
        DriverStartedAtState::Missing => {
            recovery.reset_to_pending = reset_active_nodes_to_pending(context, session_id)?;
        }
        DriverStartedAtState::Present => {
            recovery.recovered = recover_crashed_nodes(context, session_id)?;
        }
        DriverStartedAtState::Malformed => {
            warn!(
                session_id = %session_id,
                "[pipeline-driver] driverStartedAt frontmatter is malformed; failing closed"
            );
            recovery.recovered = recover_crashed_nodes(context, session_id)?;
        }
    }

    write_driver_started_at(context, session_id)?;
    Ok(recovery)
}

fn read_driver_started_at_state(
    context: &ToolContext,
    session_id: &str,
) -> Result<DriverStartedAtState> {
    let pipeline_uri = pipeline_session_uri(session_id);
    let (frontmatter, _, _) = read_markdown(&pipeline_uri, &context.config)?;
    let frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Pipeline session {session_id} has invalid frontmatter"
        ))
    })?;

    Ok(match frontmatter.get("driverStartedAt") {
        None => DriverStartedAtState::Missing,
        Some(YamlValue::String(_)) => DriverStartedAtState::Present,
        Some(_) => DriverStartedAtState::Malformed,
    })
}

fn reset_active_nodes_to_pending(context: &ToolContext, session_id: &str) -> Result<usize> {
    let pipeline_uri = pipeline_session_uri(session_id);
    let (frontmatter, pipeline_content, _) = read_markdown(&pipeline_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Pipeline session {session_id} has invalid frontmatter"
        ))
    })?;
    let mut node_states = pipeline_node_states(&frontmatter)?;
    let mut reset_to_pending = 0usize;

    for (node_id, state) in &mut node_states {
        if !matches!(state.status, NodeStatus::Active) {
            continue;
        }
        tracing::info!(
            session_id = %session_id,
            node_id = %node_id,
            "resetting dispatched active node to pending for first driver start"
        );
        state.status = NodeStatus::Pending;
        state.failure_reason = None;
        update_workflow_status(context, &state.session_id, "pending")?;
        reset_to_pending += 1;
    }

    persist_pipeline_frontmatter(
        context,
        &pipeline_uri,
        &mut frontmatter,
        &pipeline_content,
        &node_states,
    )?;
    Ok(reset_to_pending)
}

fn write_driver_started_at(context: &ToolContext, session_id: &str) -> Result<()> {
    let pipeline_uri = pipeline_session_uri(session_id);
    let (frontmatter, pipeline_content, _) = read_markdown(&pipeline_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Pipeline session {session_id} has invalid frontmatter"
        ))
    })?;
    frontmatter.insert(
        "driverStartedAt".to_string(),
        YamlValue::String(Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)),
    );
    write_markdown(
        &pipeline_uri,
        Some(&frontmatter),
        &pipeline_content,
        &context.config,
    )
}

pub(crate) fn persist_pipeline_frontmatter(
    context: &ToolContext,
    pipeline_uri: &str,
    frontmatter: &mut maenifold_core::markdown::Frontmatter,
    pipeline_content: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<()> {
    frontmatter.insert("nodeStates".to_string(), serde_yml::to_value(node_states)?);
    let status = if let Some(definition) = frontmatter
        .get("definition")
        .cloned()
        .and_then(|value| serde_yml::from_value::<PipelineDefinition>(value).ok())
    {
        compute_pipeline_status_with_definition(&definition, node_states).to_string()
    } else {
        compute_pipeline_status(node_states).to_string()
    };
    frontmatter.insert("status".to_string(), YamlValue::String(status));
    write_markdown(
        pipeline_uri,
        Some(frontmatter),
        pipeline_content,
        &context.config,
    )
}

pub(crate) fn pipeline_node_states(
    frontmatter: &maenifold_core::markdown::Frontmatter,
) -> Result<HashMap<String, NodeState>> {
    let node_states = frontmatter.get("nodeStates").cloned().ok_or_else(|| {
        MaenifoldError::Message("ERROR: Pipeline session missing frontmatter 'nodeStates'".into())
    })?;
    serde_yml::from_value(node_states).map_err(Into::into)
}

pub(crate) fn update_workflow_status(
    context: &ToolContext,
    workflow_session_id: &str,
    status: &str,
) -> Result<()> {
    let workflow_uri = workflow_session_uri(workflow_session_id);
    let (frontmatter, workflow_content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Workflow session {workflow_session_id} has invalid frontmatter"
        ))
    })?;
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    write_markdown(
        &workflow_uri,
        Some(&frontmatter),
        &workflow_content,
        &context.config,
    )
}

pub(crate) fn pipeline_session_uri(session_id: &str) -> String {
    let (year, month, day) = session_date_parts(session_id);
    format!("memory://pipelines/{year:04}/{month:02}/{day:02}/{session_id}")
}

pub(crate) fn workflow_session_uri(session_id: &str) -> String {
    let (year, month, day) = session_date_parts(session_id);
    format!("memory://thinking/workflow/{year:04}/{month:02}/{day:02}/{session_id}")
}

fn session_date_parts(session_id: &str) -> (i32, u32, u32) {
    let timestamp = session_timestamp_millis(session_id)
        .and_then(|millis| Utc.timestamp_millis_opt(millis).single())
        .unwrap_or_else(Utc::now);
    (timestamp.year(), timestamp.month(), timestamp.day())
}

fn session_timestamp_millis(session_id: &str) -> Option<i64> {
    if session_id
        .split('-')
        .any(|segment| segment.trim().is_empty())
    {
        return None;
    }

    for segment in session_id.rsplit('-') {
        if segment.len() == 13
            && segment.bytes().all(|byte| byte.is_ascii_digit())
            && let Ok(millis) = segment.parse::<i64>()
            && Utc.timestamp_millis_opt(millis).single().is_some()
        {
            return Some(millis);
        }
    }

    None
}
