use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use backon::{ExponentialBuilder, Retryable};
use chrono::{Duration, Utc};
use jockey::EventSender;
use tokio::sync::{Semaphore, broadcast};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::dispatch::dispatch_pipeline;
use super::executor_cleanup::cancel_active_nodes;
use super::executor_helpers::{
    DispatchedNode, derive_execution_status, load_execution_config, load_pipeline_node_states,
    parse_dispatch_result,
};
use super::executor_nodes::{
    NodeOutcome, NodeTaskResult, build_node_execution_request, classify_node_result,
    execute_node_request,
};
use super::session_io::{
    NodeFile, allocate_node_session_id, derive_pipeline_node_states, frontmatter_value,
    persist_pipeline_state, read_pipeline_session, write_workflow_node_session_with_status,
};
use super::update::update_pipeline_node_status;
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode, RetryPolicy};

pub(super) use super::mapping_predecessor::{assemble_node_prompt, predecessor_context_content};

#[cfg(test)]
#[path = "circuit_breaker_tests.rs"]
mod circuit_breaker_tests;
#[cfg(test)]
#[path = "executor_cancellation_tests.rs"]
mod executor_cancellation_tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub completed: usize,
    pub abandoned: usize,
    pub status: String,
}

type NodeJoinResult = (String, NodeOutcome);

#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_lines)]
pub async fn execute_pipeline(
    context: &ToolContext,
    session_id: &str,
    prompt: &str,
    cancel: CancellationToken,
) -> Result<ExecutionResult> {
    let (event_tx, _) = broadcast::channel(256);
    execute_pipeline_inner(context, session_id, prompt, cancel, event_tx).await
}

#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_lines)]
async fn execute_pipeline_inner(
    context: &ToolContext,
    session_id: &str,
    prompt: &str,
    cancel: CancellationToken,
    event_tx: EventSender,
) -> Result<ExecutionResult> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let execution_config = load_execution_config(&json_context, session_id)?;
    let nodes_by_id = build_node_index(&execution_config.definition);
    let semaphore = Arc::new(Semaphore::new(execution_config.max_concurrency));
    let mut join_set = JoinSet::new();
    let mut counts = ExecutionCounts::default();
    let prompt = prompt.to_string();

    loop {
        if cancel.is_cancelled() {
            break;
        }

        let dispatched = parse_dispatch_result(dispatch_pipeline(&json_context, session_id)?)?;
        let node_states = if dispatched.is_empty() {
            HashMap::new()
        } else {
            load_pipeline_node_states(&json_context, session_id)?
        };

        for dispatch in &dispatched {
            schedule_dispatched_node(
                &json_context,
                context,
                session_id,
                &execution_config.definition,
                &node_states,
                &nodes_by_id,
                &prompt,
                &execution_config.inputs,
                event_tx.clone(),
                semaphore.clone(),
                &mut join_set,
                dispatch,
            )
            .await?;
        }

        if dispatched.is_empty() && join_set.is_empty() {
            let node_states_check = load_pipeline_node_states(&json_context, session_id)?;
            match earliest_retry_after(&node_states_check) {
                Some(sleep_duration) => {
                    warn!(
                        "[pipeline] all nodes idle, sleeping {sleep_duration:?} until next retry"
                    );
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => break,
                        _ = tokio::time::sleep(sleep_duration) => continue,
                    }
                }
                None => break,
            }
        }

        let Some((node_id, outcome)) = join_next_result(session_id, &cancel, &mut join_set).await?
        else {
            break;
        };
        apply_node_outcome(&json_context, session_id, &node_id, outcome, &mut counts)?;
    }

    let cancelled = cancel.is_cancelled();
    if cancelled {
        join_set.abort_all();
        drop(join_set);
        if let Ok(node_states) = load_pipeline_node_states(&json_context, session_id) {
            terminate_node_sessions(&execution_config.definition, &node_states).await;
        } else {
            warn!(
                session_id = %session_id,
                "failed to load node states while terminating cancelled ACP sessions"
            );
        }
        counts.abandoned += cancel_active_nodes(&json_context, session_id)?;
    }
    log_incomplete_execution(&json_context, session_id)?;

    Ok(ExecutionResult {
        completed: counts.completed,
        abandoned: counts.abandoned,
        status: derive_execution_status(&json_context, session_id, cancelled)?,
    })
}

fn log_incomplete_execution(context: &ToolContext, session_id: &str) -> Result<()> {
    let non_terminal = load_pipeline_node_states(context, session_id)?
        .into_iter()
        .filter_map(|(node_id, state)| {
            is_non_terminal(&state.status)
                .then(|| format!("{node_id} ({})", node_status_name(&state.status)))
        })
        .collect::<Vec<_>>();
    if !non_terminal.is_empty() {
        error!(
            session_id = %session_id,
            non_terminal = ?non_terminal,
            "[pipeline] executor exited before all nodes reached terminal states"
        );
    }
    Ok(())
}

fn is_non_terminal(status: &NodeStatus) -> bool {
    matches!(
        status,
        NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active | NodeStatus::Retrying
    )
}

fn node_status_name(status: &NodeStatus) -> &'static str {
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

#[derive(Default)]
struct ExecutionCounts {
    completed: usize,
    abandoned: usize,
}

fn build_node_index(definition: &PipelineDefinition) -> HashMap<String, PipelineNode> {
    definition
        .nodes
        .iter()
        .cloned()
        .map(|node| (node.id().to_string(), node))
        .collect()
}

async fn terminate_node_sessions(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) {
    for (node_id, agent, session_id) in cancellable_node_session_targets(definition, node_states) {
        match jockey::acp::reset_session(&agent, &agent, Some(&session_id)).await {
            Ok(()) => {
                debug!(
                    node_id = %node_id,
                    session_id = %session_id,
                    agent = %agent,
                    "terminated ACP session after pipeline cancellation"
                );
            }
            Err(error) => {
                debug!(
                    node_id = %node_id,
                    session_id = %session_id,
                    agent = %agent,
                    error = %error,
                    "ACP session already exited or could not be terminated"
                );
            }
        }
    }
}

fn cancellable_node_session_targets(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<(String, String, String)> {
    let nodes_by_id = build_node_index(definition);
    node_states
        .iter()
        .filter_map(|(node_id, state)| {
            let node = nodes_by_id.get(node_id)?;
            let agent = node.acp_client()?.trim();
            if agent.is_empty() {
                return None;
            }

            let session_id = match state.status {
                NodeStatus::Active => state.session_id.as_str(),
                _ => state.acp_session_id.as_deref()?,
            }
            .trim();
            if session_id.is_empty() {
                return None;
            }

            Some((node_id.to_string(), agent.to_string(), session_id.to_string()))
        })
        .collect()
}

fn earliest_retry_after(node_states: &HashMap<String, NodeState>) -> Option<std::time::Duration> {
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

/// Resolve the per-node timeout strictly from caller-supplied configuration.
///
/// T-174.4: Returns `None` when the caller did not supply `timeout_seconds`. There
/// is **no silent default fallback** — unbounded execution is the deliberate,
/// caller-acknowledged choice when `timeout_seconds` is absent. Reintroducing
/// silent fallback timeout constants here is a P0 constitutional violation
/// ("Mechanism, Not Policy").
///
/// A caller-supplied `Some(0)` is treated as "no timeout" rather than an instant
/// elapse, since wrapping in `tokio::time::timeout(Duration::ZERO, _)` would
/// always immediately abandon the node and serves no caller intent.
fn extract_timeout(
    request: &super::executor_nodes::NodeExecutionRequest,
) -> Option<std::time::Duration> {
    let seconds = match request {
        super::executor_nodes::NodeExecutionRequest::Workflow { session_config, .. }
        | super::executor_nodes::NodeExecutionRequest::Prompt { session_config, .. }
        | super::executor_nodes::NodeExecutionRequest::SequentialThinking {
            session_config, ..
        } => session_config.timeout_seconds,
        super::executor_nodes::NodeExecutionRequest::Tool {
            timeout_seconds, ..
        } => *timeout_seconds,
    };

    seconds
        .filter(|&seconds| seconds > 0)
        .map(std::time::Duration::from_secs)
}

fn strip_node_timeout(request: &mut super::executor_nodes::NodeExecutionRequest) {
    match request {
        super::executor_nodes::NodeExecutionRequest::Workflow { session_config, .. }
        | super::executor_nodes::NodeExecutionRequest::Prompt { session_config, .. }
        | super::executor_nodes::NodeExecutionRequest::SequentialThinking {
            session_config, ..
        } => {
            session_config.timeout_seconds = None;
        }
        super::executor_nodes::NodeExecutionRequest::Tool { .. } => {}
    }
}

#[derive(Clone, Copy)]
enum TimeoutResultKind {
    Workflow,
    Prompt,
    SequentialThinking,
    Tool,
}

fn timeout_result_kind(request: &super::executor_nodes::NodeExecutionRequest) -> TimeoutResultKind {
    match request {
        super::executor_nodes::NodeExecutionRequest::Workflow { .. } => TimeoutResultKind::Workflow,
        super::executor_nodes::NodeExecutionRequest::Prompt { .. } => TimeoutResultKind::Prompt,
        super::executor_nodes::NodeExecutionRequest::SequentialThinking { .. } => {
            TimeoutResultKind::SequentialThinking
        }
        super::executor_nodes::NodeExecutionRequest::Tool { .. } => TimeoutResultKind::Tool,
    }
}

fn timeout_result_for_kind(kind: TimeoutResultKind, reason: String) -> NodeTaskResult {
    let error = MaenifoldError::Message(reason);
    match kind {
        TimeoutResultKind::Workflow => NodeTaskResult::Workflow(Err(error)),
        TimeoutResultKind::Prompt => NodeTaskResult::Prompt(Err(error)),
        TimeoutResultKind::SequentialThinking => NodeTaskResult::SequentialThinking(Err(error)),
        TimeoutResultKind::Tool => NodeTaskResult::Tool(Err(error)),
    }
}

#[allow(clippy::too_many_arguments)]
async fn schedule_dispatched_node(
    json_context: &ToolContext,
    output_context: &ToolContext,
    _session_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    nodes_by_id: &HashMap<String, PipelineNode>,
    ambient_prompt: &str,
    inputs: &serde_json::Map<String, serde_json::Value>,
    event_tx: EventSender,
    semaphore: Arc<Semaphore>,
    join_set: &mut JoinSet<NodeJoinResult>,
    dispatch: &DispatchedNode,
) -> Result<()> {
    let permit = semaphore.clone().acquire_owned().await.map_err(|_| {
        MaenifoldError::Message(
            "ERROR: Pipeline executor semaphore unexpectedly closed".to_string(),
        )
    })?;
    let node = nodes_by_id.get(&dispatch.node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline definition missing node '{}'",
            dispatch.node_id
        ))
    })?;
    let node_id = dispatch.node_id.clone();
    let agent = match node.acp_client() {
        Some(client) => Some(client.to_string()),
        None if matches!(node, PipelineNode::Tool { .. }
) => None,
        None => {
            drop(permit);
            let field = node.required_execution_target_field().unwrap_or("agent");
            return Err(MaenifoldError::Message(format!(
                "ERROR: Pipeline node '{}' cannot execute: missing '{field}' configuration",
                node_id
            )));
        }
    };
    let mut execution = build_node_execution_request(
        json_context,
        definition,
        node_states,
        node,
        dispatch,
        ambient_prompt,
        inputs,
    )?;
    let node_retry_policy = node.retry_policy().cloned();
    let output_config = output_context.config.clone();
    let output_format = output_context.output_format;
    let timeout = extract_timeout(&execution);
    let timeout_kind = timeout_result_kind(&execution);
    // Strip timeout_seconds from the session config so the outer wrapper owns node TTL.
    // Inner agent-session setup keeps its default connection behavior.
    strip_node_timeout(&mut execution);
    join_set.spawn(async move {
        let _permit = permit;
        let run_once = || {
            let node_id = node_id.clone();
            let execution = execution.clone();
            let agent = agent.clone();
            let event_tx = event_tx.clone();
            let output_config = output_config.clone();
            async move {
                let context = ToolContext::new(output_config, output_format);
                let (timed_out, (_, result)) = match timeout {
                    Some(duration) => {
                        match tokio::time::timeout(
                            duration,
                            execute_node_request(
                                context,
                                node_id.clone(),
                                execution,
                                agent,
                                event_tx.clone(),
                            ),
                        )
                        .await
                        {
                            Ok(result) => (false, result),
                            Err(_elapsed) => {
                                // NOTE: T-107.8 — The dropped future may leave stale in-memory session
                                // tracking behind the agent session wrapper. This is bounded (max
                                // concurrent nodes) and has no wire-protocol impact.
                                let reason = format!("timeout after {}s", duration.as_secs());
                                warn!(node_id = %node_id, reason = %reason, "pipeline node timed out");
                                (
                                    true,
                                    (node_id.clone(), timeout_result_for_kind(timeout_kind, reason)),
                                )
                            }
                        }
                    }
                    None => (
                        false,
                        execute_node_request(context, node_id.clone(), execution, agent, event_tx)
                            .await,
                    ),
                };

                let outcome = classify_node_result(&node_id, result);
                match outcome {
                    NodeOutcome::Abandoned {
                        reason,
                        phase,
                        transient_error_kind,
                        ..
                    } if !timed_out && phase.is_retryable() => Err(RetryExecutionError {
                        kind: transient_error_kind.unwrap_or("transient"),
                        reason,
                    }),
                    other => Ok(other),
                }
            }
        };

        let (outcome, retry_attempts) = match node_retry_policy {
            Some(retry_policy) if retry_policy.max_attempts > 0 => {
                execute_with_retry_policy(&node_id, retry_policy, run_once).await
            }
            _ => match run_once().await {
                Ok(outcome) => (outcome, 0),
                Err(error) => (
                    NodeOutcome::Abandoned {
                        reason: error.reason,
                        phase: super::executor_run::FailurePhase::PostPrompt,
                        transient_error_kind: None,
                        acp_session_id: None,
                        thinking_session_id: None,
                        retry_attempts: 0,
                    },
                    0,
                ),
            },
        };

        let outcome = with_retry_attempts(outcome, retry_attempts);
        (node_id, outcome)
    });
    Ok(())
}

async fn join_next_result(
    session_id: &str,
    cancel: &CancellationToken,
    join_set: &mut JoinSet<NodeJoinResult>,
) -> Result<Option<NodeJoinResult>> {
    let Some(joined) = (tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            warn!(session_id, "pipeline execution cancelled");
            None
        },
        joined = join_set.join_next(), if !join_set.is_empty() => joined,
    }) else {
        return Ok(None);
    };

    joined
        .map(Some)
        .map_err(|error| MaenifoldError::Message(format!("ERROR: Node task failed: {error}")))
}

fn apply_node_outcome(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    outcome: NodeOutcome,
    counts: &mut ExecutionCounts,
) -> Result<()> {
    match outcome {
        NodeOutcome::Completed {
            conclusion,
            acp_session_id,
            thinking_session_id,
            tool_provenance,
            retry_attempts,
        } => {
            increment_node_attempts(context, session_id, node_id, retry_attempts)?;
            update_pipeline_node_status(
                context,
                session_id,
                node_id,
                "completed",
                Some(&conclusion),
                acp_session_id.as_deref(),
                thinking_session_id.as_deref(),
                tool_provenance.as_ref(),
                None,
            )?;
            counts.completed += 1;
        }
        NodeOutcome::Abandoned {
            reason,
            phase,
            transient_error_kind,
            retry_attempts,
            acp_session_id,
            thinking_session_id,
            ..
        } => {
            if phase.is_retryable() {
                return persist_retryable_failure(
                    context,
                    session_id,
                    node_id,
                    &reason,
                    transient_error_kind,
                    retry_attempts,
                    acp_session_id.as_deref(),
                    thinking_session_id.as_deref(),
                );
            }
            increment_node_attempts(context, session_id, node_id, retry_attempts)?;
            update_pipeline_node_status(
                context,
                session_id,
                node_id,
                "abandoned",
                None,
                acp_session_id.as_deref(),
                thinking_session_id.as_deref(),
                None,
                Some(&reason),
            )?;
            counts.abandoned += 1;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn persist_retryable_failure(
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
    let retry_count = retry_attempts.max(1);
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
    write_workflow_node_session_with_status(context, session_id, &node_file, "retrying")
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
    operation: F,
) -> (NodeOutcome, u32)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<NodeOutcome, RetryExecutionError>>,
{
    let mut retry_attempts: u32 = 0;
    let mut builder = ExponentialBuilder::default()
        .with_min_delay(StdDuration::from_millis(retry_policy.backoff_min_ms))
        .with_max_delay(StdDuration::from_millis(retry_policy.backoff_max_ms))
        .with_max_times(retry_policy.max_attempts as usize);
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
            ..
        } => NodeOutcome::Abandoned {
            reason,
            phase,
            transient_error_kind,
            acp_session_id,
            thinking_session_id,
            retry_attempts,
        },
    }
}

fn increment_node_attempts(
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
    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::executor_nodes::NodeExecutionRequest;
    use super::extract_timeout;
    use crate::workflow::runner::AgentSessionConfig;

    fn sample_session_config(timeout_seconds: Option<u64>) -> AgentSessionConfig {
        AgentSessionConfig {
            cwd: PathBuf::from("."),
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds,
            session_id: None,
            auto_approve: true,
        }
    }

    /// T-174.4: Tool nodes WITHOUT a caller-supplied `timeout_seconds` must run
    /// unbounded. There is no silent default fallback.
    #[test]
    fn tool_nodes_without_timeout_run_unbounded() {
        let request = NodeExecutionRequest::Tool {
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            allowed_tools: None,
            cwd: PathBuf::from("."),
            timeout_seconds: None,
        };

        assert_eq!(extract_timeout(&request), None);
    }

    #[test]
    fn tool_nodes_honor_explicit_timeout_override() {
        let request = NodeExecutionRequest::Tool {
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            allowed_tools: None,
            cwd: PathBuf::from("."),
            timeout_seconds: Some(12),
        };

        assert_eq!(
            extract_timeout(&request),
            Some(std::time::Duration::from_secs(12))
        );
    }

    #[test]
    fn non_tool_nodes_continue_using_session_timeout() {
        let request = NodeExecutionRequest::Prompt {
            session_id: "pipeline-node".to_string(),
            prompt: "prompt".to_string(),
            tool_mode: "disabled".to_string(),
            session_config: sample_session_config(Some(7)),
        };

        assert_eq!(
            extract_timeout(&request),
            Some(std::time::Duration::from_secs(7))
        );
    }

    /// T-174.4: Agent nodes (Prompt / SequentialThinking / Workflow) WITHOUT a
    /// caller-supplied `timeout_seconds` must run unbounded. The previous T-170.8
    /// silent 300s default has been deleted.
    #[test]
    fn agent_nodes_without_timeout_run_unbounded() {
        let request = NodeExecutionRequest::Prompt {
            session_id: "pipeline-node".to_string(),
            prompt: "prompt".to_string(),
            tool_mode: "disabled".to_string(),
            session_config: sample_session_config(None),
        };

        assert_eq!(extract_timeout(&request), None);
    }

    /// T-174.4: A caller-supplied `Some(0)` is treated as "no timeout" rather than
    /// instant elapse, since wrapping in `tokio::time::timeout(Duration::ZERO, _)`
    /// would always immediately abandon the node and serves no caller intent.
    #[test]
    fn explicit_zero_timeout_is_treated_as_unbounded() {
        let request = NodeExecutionRequest::Tool {
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            allowed_tools: None,
            cwd: PathBuf::from("."),
            timeout_seconds: Some(0),
        };

        assert_eq!(extract_timeout(&request), None);
    }
}

#[cfg(test)]
#[path = "apply_node_outcome_failure_reason_tests.rs"]
mod apply_node_outcome_failure_reason_tests;

#[cfg(test)]
#[path = "retry_tests.rs"]
mod retry_tests;
