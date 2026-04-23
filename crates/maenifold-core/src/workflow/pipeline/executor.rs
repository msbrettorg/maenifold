use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
pub(super) use super::NodeStatus;
use super::dispatch::dispatch_pipeline;
use super::executor_apply::{apply_node_outcome, node_adoption_labels};
#[cfg(test)]
pub(super) use super::executor_cleanup::cancellable_node_session_targets;
use super::executor_cleanup::{cancel_active_nodes, terminate_node_sessions};
use super::executor_helpers::{
    DispatchedNode, derive_execution_status, load_execution_config, load_pipeline_node_states,
    parse_dispatch_result,
};
use super::executor_nodes::build_node_execution_request;
use super::executor_orchestration::{
    ExecutionCounts, NodeJoinResult, build_node_index, join_next_result, log_incomplete_execution,
    spawn_dispatched_node_task,
};
use super::executor_retry::{earliest_retry_after, extract_retry_count_from_state};
use super::executor_timeout::{extract_timeout, strip_node_timeout, timeout_result_kind};
pub(super) use super::mapping_predecessor::{assemble_node_prompt, predecessor_context_content};
use super::{NodeState, PipelineDefinition, PipelineNode};
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use jockey::EventSender;
use tokio::sync::{Semaphore, broadcast};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
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
#[allow(clippy::too_many_arguments)]
async fn schedule_dispatched_node(
    json_context: &ToolContext,
    output_context: &ToolContext,
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
    if agent.is_some() {
        let (adopt_role, adopt_color, adopt_perspective) = node_adoption_labels(node);
        info!(
            node_id = %node_id,
            node_type = %node.node_type_name(),
            agent = ?agent,
            adopt_role = %adopt_role,
            adopt_color = %adopt_color,
            adopt_perspective = %adopt_perspective,
            "dispatching pipeline agent invocation"
        );
    }
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
    let initial_retry_attempts = extract_retry_count_from_state(node_states.get(&node_id));
    let output_config = output_context.config.clone();
    let output_format = output_context.output_format;
    let timeout = extract_timeout(&execution);
    let timeout_kind = timeout_result_kind(&execution);
    strip_node_timeout(&mut execution);
    spawn_dispatched_node_task(
        join_set,
        permit,
        node_id,
        execution,
        agent,
        event_tx,
        output_config,
        output_format,
        timeout,
        timeout_kind,
        node_retry_policy,
        initial_retry_attempts,
    );
    Ok(())
}

#[cfg(test)]
#[path = "apply_node_outcome_failure_reason_tests.rs"]
mod apply_node_outcome_failure_reason_tests;

#[cfg(test)]
#[path = "retry_tests.rs"]
mod retry_tests;

#[cfg(test)]
#[path = "retry_tests_exec.rs"]
mod retry_tests_exec;
