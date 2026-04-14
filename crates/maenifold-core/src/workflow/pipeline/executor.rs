use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use jockey::EventSender;
use tokio::sync::{Semaphore, broadcast};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::dispatch::dispatch_pipeline;
use super::executor_cleanup::abandon_active_nodes;
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
    persist_pipeline_state, read_pipeline_session, update_workflow_node_frontmatter,
    write_workflow_node_session_with_status,
};
use super::update::update_pipeline_node_status;
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

pub(super) use super::executor_mapping::{assemble_node_prompt, predecessor_context_content};

#[cfg(test)]
#[path = "circuit_breaker_tests.rs"]
mod circuit_breaker_tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub completed: usize,
    pub abandoned: usize,
    pub status: String,
}

type NodeJoinResult = (String, NodeTaskResult);

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
                    eprintln!(
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

        let Some((node_id, result)) = join_next_result(&cancel, &mut join_set).await? else {
            break;
        };
        apply_node_outcome(
            &json_context,
            session_id,
            &node_id,
            classify_node_result(&node_id, result),
            &mut counts,
        )?;
    }

    let cancelled = cancel.is_cancelled();
    if cancelled {
        join_set.abort_all();
        drop(join_set);
        counts.abandoned += abandon_active_nodes(&json_context, session_id)?;
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
        tracing::error!(
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

fn extract_timeout(
    request: &super::executor_nodes::NodeExecutionRequest,
) -> Option<std::time::Duration> {
    const DEFAULT_TOOL_NODE_TIMEOUT_SECS: u64 = 60;

    let seconds = match request {
        super::executor_nodes::NodeExecutionRequest::Workflow { session_config, .. }
        | super::executor_nodes::NodeExecutionRequest::Prompt { session_config, .. }
        | super::executor_nodes::NodeExecutionRequest::SequentialThinking {
            session_config, ..
        } => session_config.timeout_seconds,
        super::executor_nodes::NodeExecutionRequest::Tool {
            timeout_seconds, ..
        } => timeout_seconds.or(Some(DEFAULT_TOOL_NODE_TIMEOUT_SECS)),
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
        None if matches!(node, PipelineNode::Tool { .. }) => None,
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
    let ctx = ToolContext::new(output_context.config.clone(), output_context.output_format);
    let timeout = extract_timeout(&execution);
    // Strip timeout_seconds from the session config so the outer wrapper owns node TTL.
    // Inner agent-session setup keeps its default connection behavior.
    strip_node_timeout(&mut execution);
    join_set.spawn(async move {
        let _permit = permit;
        match timeout {
            Some(duration) => {
                match tokio::time::timeout(
                    duration,
                    execute_node_request(ctx, node_id.clone(), execution, agent, event_tx.clone()),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_elapsed) => {
                        // NOTE: T-107.8 — The dropped future may leave stale in-memory session
                        // tracking behind the agent session wrapper. This is bounded (max
                        // concurrent nodes) and has no wire-protocol impact.
                        let reason = format!("node timed out after {}s", duration.as_secs());
                        eprintln!("[pipeline] node {node_id} {reason}");
                        (
                            node_id,
                            NodeTaskResult::Prompt(Err(MaenifoldError::Message(reason))),
                        )
                    }
                }
            }
            None => execute_node_request(ctx, node_id, execution, agent, event_tx).await,
        }
    });
    Ok(())
}

async fn join_next_result(
    cancel: &CancellationToken,
    join_set: &mut JoinSet<NodeJoinResult>,
) -> Result<Option<NodeJoinResult>> {
    let Some(joined) = (tokio::select! {
        biased;
        _ = cancel.cancelled() => None,
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
        } => {
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
            acp_session_id,
            thinking_session_id,
        } => {
            if phase.is_retryable() {
                schedule_node_retry(context, session_id, node_id, &reason)?;
            } else {
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
    }
    Ok(())
}

fn schedule_node_retry(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    reason: &str,
) -> Result<()> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let max_attempts = frontmatter
        .get("maxAttempts")
        .and_then(serde_yaml::Value::as_u64)
        .unwrap_or(3) as u32;
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
    let previous_session_id = state.session_id.clone();
    let current_attempts = state.attempts;

    if current_attempts >= max_attempts {
        state.status = NodeStatus::Stuck;
        state.output = None;
        state.tool_provenance = None;
        state.failure_reason = Some(reason.to_string());
        state.retry_after = None;
        persist_pipeline_state(
            &mut frontmatter,
            &node_states,
            &uri,
            &pipeline_content,
            context,
        )?;
        update_workflow_node_frontmatter(context, &previous_session_id, "stuck", None)?;
        return Ok(());
    }

    let retry_count = state.retry_count.unwrap_or(0) + 1;
    let next_session_id = allocate_node_session_id(context, node_id)?;
    state.session_id = next_session_id.clone();
    state.status = NodeStatus::Retrying;
    state.attempts += 1;
    state.acp_session_id = None;
    state.output = None;
    state.thinking_session_id = None;
    state.tool_provenance = None;
    state.failure_reason = None;
    state.retry_after = Some(compute_retry_after(state.attempts));
    state.retry_count = Some(retry_count);

    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;
    update_workflow_node_frontmatter(context, &previous_session_id, "abandoned", None)?;
    write_workflow_node_session_with_status(
        context,
        session_id,
        &NodeFile {
            session_id: next_session_id,
            node_id: node_id.to_string(),
            workflow_id: node.effective_workflow_id(),
        },
        "retrying",
    )?;
    Ok(())
}

fn compute_retry_after(attempts: u32) -> String {
    let backoff_secs = 2u64.pow(attempts.min(6));
    (Utc::now() + Duration::seconds(backoff_secs as i64)).to_rfc3339()
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
        }
    }

    #[test]
    fn tool_nodes_default_to_sixty_second_timeout() {
        let request = NodeExecutionRequest::Tool {
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            allowed_tools: None,
            timeout_seconds: None,
        };

        assert_eq!(
            extract_timeout(&request),
            Some(std::time::Duration::from_secs(60))
        );
    }

    #[test]
    fn tool_nodes_honor_explicit_timeout_override() {
        let request = NodeExecutionRequest::Tool {
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            allowed_tools: None,
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
}

#[cfg(test)]
#[path = "apply_node_outcome_failure_reason_tests.rs"]
mod apply_node_outcome_failure_reason_tests;

#[cfg(test)]
#[path = "retry_tests.rs"]
mod retry_tests;
