use std::collections::HashMap;

use tokio::task::JoinSet;
use tokio::sync::OwnedSemaphorePermit;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::executor_helpers::load_pipeline_node_states;
use super::executor_nodes::{NodeExecutionRequest, NodeOutcome, classify_node_result, execute_node_request};
use super::executor_retry::{
    RetryExecutionError, execute_with_retry_policy, with_retry_attempts,
};
use super::executor_timeout::{TimeoutResultKind, timeout_result_for_kind};
use super::{NodeStatus, PipelineDefinition, PipelineNode, RetryPolicy};
use jockey::EventSender;

pub(super) type NodeJoinResult = (String, NodeOutcome);

#[derive(Default)]
pub(super) struct ExecutionCounts {
    pub(super) completed: usize,
    pub(super) abandoned: usize,
}

pub(super) fn build_node_index(definition: &PipelineDefinition) -> HashMap<String, PipelineNode> {
    definition
        .nodes
        .iter()
        .cloned()
        .map(|node| (node.id().to_string(), node))
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_dispatched_node_task(
    join_set: &mut JoinSet<NodeJoinResult>,
    permit: OwnedSemaphorePermit,
    node_id: String,
    execution: NodeExecutionRequest,
    agent: Option<String>,
    event_tx: EventSender,
    output_config: crate::config::Config,
    output_format: OutputFormat,
    timeout: Option<std::time::Duration>,
    timeout_kind: TimeoutResultKind,
    node_retry_policy: Option<RetryPolicy>,
    initial_retry_attempts: u32,
) {
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
                        is_outer_timeout: false,
                        ..
                    } if !timed_out && phase.is_retryable() => Err(RetryExecutionError {
                        kind: transient_error_kind.unwrap_or("transient"),
                        reason,
                    }),
                    NodeOutcome::Abandoned {
                        reason,
                        phase,
                        transient_error_kind,
                        acp_session_id,
                        thinking_session_id,
                        retry_attempts,
                        ..
                    } if timed_out => Ok(NodeOutcome::Abandoned {
                        reason,
                        phase,
                        transient_error_kind,
                        acp_session_id,
                        thinking_session_id,
                        retry_attempts,
                        is_outer_timeout: true,
                    }),
                    other => Ok(other),
                }
            }
        };

        let (outcome, retry_attempts) = match node_retry_policy {
            Some(retry_policy) if retry_policy.max_attempts > 0 => {
                execute_with_retry_policy(&node_id, retry_policy, initial_retry_attempts, run_once)
                    .await
            }
            _ => match run_once().await {
                Ok(outcome) => (outcome, initial_retry_attempts),
                Err(error) => (
                    NodeOutcome::Abandoned {
                        reason: error.reason,
                        phase: super::executor_run::FailurePhase::PostPrompt,
                        transient_error_kind: None,
                        acp_session_id: None,
                        thinking_session_id: None,
                        retry_attempts: 0,
                        is_outer_timeout: false,
                    },
                    initial_retry_attempts,
                ),
            },
        };

        let outcome = with_retry_attempts(outcome, retry_attempts);
        (node_id, outcome)
    });
}

pub(super) async fn join_next_result(
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

pub(super) fn log_incomplete_execution(context: &ToolContext, session_id: &str) -> Result<()> {
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
