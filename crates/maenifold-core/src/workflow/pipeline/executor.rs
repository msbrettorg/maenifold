use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::dispatch::dispatch_pipeline;
use super::executor_helpers::{
    DispatchedNode, abandon_active_nodes, derive_execution_status, load_execution_config,
    load_pipeline_node_states, parse_dispatch_result,
};
use super::executor_nodes::{
    NodeOutcome, NodeTaskResult, build_node_execution_request, classify_node_result,
    execute_node_request,
};
use super::supervisor::supervisor_wait_if_paused;
use super::update::update_pipeline_node_status;
use super::{NodeState, PipelineDefinition, PipelineNode};

pub(super) use super::executor_mapping::{assemble_node_prompt, predecessor_context_content};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutionResult {
    pub completed: usize,
    pub abandoned: usize,
    pub status: String,
}

type NodeJoinResult = (String, NodeTaskResult);

#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_lines)]
pub(super) async fn execute_pipeline(
    context: &ToolContext,
    session_id: &str,
    prompt: &str,
    cancel: CancellationToken,
) -> Result<ExecutionResult> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let (definition, max_concurrency) = load_execution_config(&json_context, session_id)?;
    let nodes_by_id = build_node_index(&definition);
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut join_set = JoinSet::new();
    let mut counts = ExecutionCounts::default();
    let prompt = prompt.to_string();

    loop {
        if cancel.is_cancelled() {
            break;
        }

        supervisor_wait_if_paused(session_id, &cancel).await;
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
                &definition,
                &node_states,
                &nodes_by_id,
                &prompt,
                semaphore.clone(),
                &mut join_set,
                dispatch,
                &mut counts.abandoned,
            )
            .await?;
        }

        if dispatched.is_empty() && join_set.is_empty() {
            break;
        }

        let Some((node_id, result)) = join_next_result(&cancel, &mut join_set).await? else {
            break;
        };
        apply_node_outcome(
            &json_context,
            session_id,
            &node_id,
            classify_node_result(result),
            &mut counts,
        )?;
    }

    let cancelled = cancel.is_cancelled();
    if cancelled {
        join_set.abort_all();
        drop(join_set);
        counts.abandoned += abandon_active_nodes(&json_context, session_id)?;
    }

    Ok(ExecutionResult {
        completed: counts.completed,
        abandoned: counts.abandoned,
        status: derive_execution_status(&json_context, session_id, cancelled)?,
    })
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

#[allow(clippy::too_many_arguments)]
async fn schedule_dispatched_node(
    json_context: &ToolContext,
    output_context: &ToolContext,
    session_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    nodes_by_id: &HashMap<String, PipelineNode>,
    ambient_prompt: &str,
    semaphore: Arc<Semaphore>,
    join_set: &mut JoinSet<NodeJoinResult>,
    dispatch: &DispatchedNode,
    abandoned: &mut usize,
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
            update_pipeline_node_status(
                json_context,
                session_id,
                &node_id,
                "abandoned",
                None,
                None,
            )?;
            *abandoned += 1;
            return Ok(());
        }
    };
    let execution = build_node_execution_request(
        json_context,
        definition,
        node_states,
        node,
        dispatch,
        ambient_prompt,
    )?;
    let ctx = ToolContext::new(output_context.config.clone(), output_context.output_format);
    join_set.spawn(async move {
        let _permit = permit;
        execute_node_request(ctx, node_id, execution, agent).await
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
            thinking_session_id,
        } => {
            update_pipeline_node_status(
                context,
                session_id,
                node_id,
                "completed",
                Some(&conclusion),
                thinking_session_id.as_deref(),
            )?;
            counts.completed += 1;
        }
        NodeOutcome::Abandoned => {
            update_pipeline_node_status(context, session_id, node_id, "abandoned", None, None)?;
            counts.abandoned += 1;
        }
    }
    Ok(())
}
