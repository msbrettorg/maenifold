use chrono::{Duration, Utc};
use serde_yaml::Value as YamlValue;

use crate::error::{MaenifoldError, Result};
use crate::markdown::{read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;

use super::session_io::{
    NodeFile, allocate_node_session_id, derive_pipeline_node_states, frontmatter_value,
    persist_pipeline_state, read_pipeline_session, write_workflow_node_session_with_status,
};
use super::update::update_pipeline_node_status;
use super::{NodeStatus, PipelineDefinition, RetryPolicy};

use super::executor_retry::extract_retry_count_from_state;

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
    let fallback_max_attempts = frontmatter
        .get("maxAttempts")
        .and_then(serde_yaml::Value::as_u64)
        .unwrap_or(3) as u32;
    let prior_retry_count = extract_retry_count_from_state(node_states.get(node_id));
    let retry_count = retry_attempts.max(prior_retry_count);
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

    let retry_policy = node.retry_policy();
    let retry_budget_exhausted = retry_policy
        .map(|policy| retry_count >= policy.max_attempts)
        .unwrap_or(state.attempts >= fallback_max_attempts);
    if retry_budget_exhausted {
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
            None,
            None,
            None,
        )?;
        return Ok(());
    }

    let next_retry_count = retry_count.saturating_add(1);
    let next_session_id = allocate_node_session_id(context, node_id)?;
    state.session_id = next_session_id.clone();
    state.status = NodeStatus::Retrying;
    state.attempts = state.attempts.saturating_add(1);
    state.started_at = None;
    state.completed_at = None;
    state.acp_session_id = None;
    state.thinking_session_id = None;
    state.tool_provenance = None;
    state.input_tokens = None;
    state.output_tokens = None;
    state.model = None;
    state.output = None;
    state.failure_reason = None;
    state.retry_count = Some(next_retry_count);
    state.retry_after = Some(next_retry_after(retry_policy, next_retry_count));

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
        .unwrap_or(next_retry_count);
    persist_workflow_retry_count(context, &node_file.session_id, persisted_retry_count)
}

fn next_retry_after(policy: Option<&RetryPolicy>, retry_count: u32) -> String {
    let delay = policy
        .map(|policy| retry_delay(policy, retry_count))
        .unwrap_or_else(|| Duration::seconds(1));
    (Utc::now() + delay).to_rfc3339()
}

fn retry_delay(policy: &RetryPolicy, retry_count: u32) -> Duration {
    let exponent = retry_count.saturating_sub(1).min(31);
    let multiplier = 1u64.checked_shl(exponent).unwrap_or(u64::MAX);
    let delay_ms = policy
        .backoff_min_ms
        .saturating_mul(multiplier)
        .min(policy.backoff_max_ms);
    let delay_ms = if policy.jitter {
        deterministic_jitter_ms(delay_ms, retry_count)
    } else {
        delay_ms
    };
    Duration::milliseconds(i64::try_from(delay_ms).unwrap_or(i64::MAX))
}

fn deterministic_jitter_ms(delay_ms: u64, retry_count: u32) -> u64 {
    if delay_ms <= 1 {
        return delay_ms;
    }
    let spread = delay_ms / 2;
    let offset = (u64::from(retry_count).saturating_mul(1_103_515_245) % spread.max(1)) + 1;
    delay_ms.saturating_sub(offset)
}

pub(super) fn persist_workflow_retry_count(
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
