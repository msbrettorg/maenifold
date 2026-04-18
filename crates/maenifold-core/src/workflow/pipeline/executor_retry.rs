//! ## Known Limitations
//!
//! - **Retry budget is per-process-lifetime, not per-pipeline-lifetime.**
//!   If the pipeline driver crashes mid-retry and restarts, the retry counter
//!   resets. This means a node can exceed its declared `max_retries` across
//!   restarts. A durability fix is planned but not yet implemented.

use std::time::Duration as StdDuration;

use backon::{ExponentialBuilder, Retryable};
use chrono::{Duration, Utc};
use serde_yaml::Value as YamlValue;
use tracing::info;

use crate::error::{MaenifoldError, Result};
use crate::markdown::{read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;

use super::executor_nodes::NodeOutcome;
use super::session_io::{
    NodeFile, allocate_node_session_id, derive_pipeline_node_states, frontmatter_value,
    persist_pipeline_state, read_pipeline_session, write_workflow_node_session_with_status,
};
use super::update::update_pipeline_node_status;
use super::{NodeState, NodeStatus, PipelineDefinition, RetryPolicy};

pub(super) fn earliest_retry_after(
    node_states: &std::collections::HashMap<String, NodeState>,
) -> Option<std::time::Duration> {
    let now = Utc::now();
    node_states
        .values()
        .filter(|state| matches!(state.status, NodeStatus::Retrying))
        .filter_map(|state| state.retry_after.as_deref())
        .filter_map(|retry_after| chrono::DateTime::parse_from_rfc3339(retry_after).ok())
        .map(|retry_at| retry_at.with_timezone(&Utc))
        .filter_map(|retry_at| {
            let diff = retry_at.signed_duration_since(now);
            if diff > Duration::zero() {
                diff.to_std().ok()
            } else {
                Some(std::time::Duration::ZERO)
            }
        })
        .min()
}

#[derive(Debug)]
pub(super) struct RetryExecutionError {
    pub(super) kind: &'static str,
    pub(super) reason: String,
}

impl std::fmt::Display for RetryExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.reason, self.kind)
    }
}

impl std::error::Error for RetryExecutionError {}

pub(super) async fn execute_with_retry_policy<F, Fut>(
    node_id: &str,
    retry_policy: RetryPolicy,
    initial_retry_attempts: u32,
    mut operation: F,
) -> (NodeOutcome, u32)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<NodeOutcome, RetryExecutionError>>,
{
    let mut retry_attempts = initial_retry_attempts;
    let remaining_attempts = retry_policy.max_attempts.saturating_sub(initial_retry_attempts);
    if remaining_attempts == 0 {
        return match operation().await {
            Ok(outcome) => (outcome, retry_attempts),
            Err(error) => (
                NodeOutcome::Abandoned {
                    reason: format!(
                        "abandoned after {} transient retries: {}",
                        retry_attempts, error.reason
                    ),
                    phase: super::executor_run::FailurePhase::PostPrompt,
                    transient_error_kind: None,
                    acp_session_id: None,
                    thinking_session_id: None,
                    retry_attempts: 0,
                    is_outer_timeout: false,
                },
                retry_attempts,
            ),
        };
    }

    let mut builder = ExponentialBuilder::default()
        .with_min_delay(StdDuration::from_millis(retry_policy.backoff_min_ms))
        .with_max_delay(StdDuration::from_millis(retry_policy.backoff_max_ms))
        .with_max_times(remaining_attempts as usize);
    if retry_policy.jitter {
        builder = builder.with_jitter();
    }

    let result = operation
        .retry(builder)
        .sleep(tokio::time::sleep)
        .when(|_| true)
        .notify(|error: &RetryExecutionError, _| {
            retry_attempts = retry_attempts.saturating_add(1);
            info!(
                node_id = %node_id,
                attempt = retry_attempts,
                error_kind = error.kind,
                error = %error.reason,
                "[pipeline] retrying node execution"
            );
        })
        .await;

    match result {
        Ok(outcome) => (outcome, retry_attempts),
        Err(error) => (
            NodeOutcome::Abandoned {
                reason: format!(
                    "abandoned after {} transient retries: {}",
                    retry_attempts, error.reason
                ),
                phase: super::executor_run::FailurePhase::PostPrompt,
                transient_error_kind: None,
                acp_session_id: None,
                thinking_session_id: None,
                retry_attempts: 0,
                is_outer_timeout: false,
            },
            retry_attempts,
        ),
    }
}

pub(super) fn with_retry_attempts(outcome: NodeOutcome, retry_attempts: u32) -> NodeOutcome {
    match outcome {
        NodeOutcome::Completed {
            conclusion,
            acp_session_id,
            thinking_session_id,
            tool_provenance,
            ..
        } => NodeOutcome::Completed {
            conclusion,
            acp_session_id,
            thinking_session_id,
            tool_provenance,
            retry_attempts,
        },
        NodeOutcome::Abandoned {
            reason,
            phase,
            transient_error_kind,
            acp_session_id,
            thinking_session_id,
            is_outer_timeout,
            ..
        } => NodeOutcome::Abandoned {
            reason,
            phase,
            transient_error_kind,
            acp_session_id,
            thinking_session_id,
            retry_attempts,
            is_outer_timeout,
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn persist_retryable_failure(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    reason: &str,
    transient_error_kind: Option<&'static str>,
    retry_attempts: u32,
    acp_session_id: Option<&str>,
    thinking_session_id: Option<&str>,
) -> Result<()> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let max_attempts = frontmatter
        .get("maxAttempts")
        .and_then(serde_yaml::Value::as_u64)
        .unwrap_or(3) as u32;
    let prior_retry_count = extract_retry_count_from_state(node_states.get(node_id));
    let retry_count = retry_attempts.max(prior_retry_count).max(1);
    let node = definition
        .nodes
        .iter()
        .find(|node| node.id() == node_id)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline definition missing node '{node_id}'"
            ))
        })?;
    let state = node_states.get_mut(node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        ))
    })?;
    if state.status != NodeStatus::Active {
        return Err(MaenifoldError::Message(format!(
            "ERROR: Node '{node_id}' must be active to transition to retrying/stuck; current status is {:?}",
            state.status
        )));
    }

    state.retry_count = Some(retry_count);
    state.retry_after = None;
    if retry_attempts > 0 {
        state.attempts = state.attempts.saturating_add(retry_attempts);
    }

    if state.attempts >= max_attempts {
        persist_workflow_retry_count(context, &state.session_id, retry_count)?;
        persist_pipeline_state(
            &mut frontmatter,
            &node_states,
            &uri,
            &pipeline_content,
            context,
        )?;
        let failure_reason = transient_error_kind
            .map(|kind| format!("{reason} ({kind})"))
            .unwrap_or_else(|| reason.to_string());
        update_pipeline_node_status(
            context,
            session_id,
            node_id,
            "stuck",
            None,
            acp_session_id,
            thinking_session_id,
            None,
            Some(&failure_reason),
        )?;
        return Ok(());
    }

    let next_session_id = allocate_node_session_id(context, node_id)?;
    state.session_id = next_session_id.clone();
    state.status = NodeStatus::Retrying;
    state.attempts = state.attempts.saturating_add(1);
    state.acp_session_id = None;
    state.thinking_session_id = None;
    state.tool_provenance = None;
    state.output = None;
    state.failure_reason = None;
    state.retry_after = Some((Utc::now() + Duration::seconds(1)).to_rfc3339());

    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;

    let node_file = NodeFile {
        session_id: next_session_id,
        node_id: node_id.to_string(),
        workflow_id: node.effective_workflow_id(),
    };
    write_workflow_node_session_with_status(context, session_id, &node_file, "retrying")?;
    let persisted_retry_count = node_states
        .get(node_id)
        .and_then(|state| state.retry_count)
        .unwrap_or(retry_count);
    persist_workflow_retry_count(context, &node_file.session_id, persisted_retry_count)
}

pub(super) fn increment_node_attempts(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    retry_attempts: u32,
) -> Result<()> {
    if retry_attempts == 0 {
        return Ok(());
    }

    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let state = node_states.get_mut(node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        ))
    })?;
    state.attempts = state.attempts.saturating_add(retry_attempts);
    state.retry_count = Some(retry_attempts);
    state.retry_after = None;
    persist_workflow_retry_count(context, &state.session_id, retry_attempts)?;
    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )
}

pub(super) fn extract_retry_count_from_state(state: Option<&NodeState>) -> u32 {
    state.and_then(|state| state.retry_count).unwrap_or(0)
}

fn persist_workflow_retry_count(
    context: &ToolContext,
    workflow_session_id: &str,
    retry_count: u32,
) -> Result<()> {
    let workflow_uri = workflow_session_uri(workflow_session_id);
    let (frontmatter, content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Workflow session {workflow_session_id} has invalid format"
        ))
    })?;
    frontmatter.insert(
        "retryCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(retry_count)),
    );
    write_markdown(&workflow_uri, Some(&frontmatter), &content, &context.config)
}
