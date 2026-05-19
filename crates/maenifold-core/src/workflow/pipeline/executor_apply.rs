use super::PipelineNode;
use super::executor_nodes::NodeOutcome;
use super::executor_orchestration::ExecutionCounts;
use super::executor_retry::{increment_node_attempts, persist_retryable_failure};
use super::update::update_pipeline_node_status;
use crate::error::Result;
use crate::tools::ToolContext;

pub(super) fn node_adoption_labels(node: &PipelineNode) -> (&str, &str, &str) {
    let role = node
        .adopt()
        .and_then(|adopt| adopt.role.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("none");
    let color = node
        .adopt()
        .and_then(|adopt| adopt.color.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("none");
    let perspective = node
        .adopt()
        .and_then(|adopt| adopt.perspective.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("none");
    (role, color, perspective)
}

pub(super) fn apply_node_outcome(
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
            input_tokens,
            output_tokens,
            model,
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
                input_tokens,
                output_tokens,
                model.as_deref(),
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
            input_tokens,
            output_tokens,
            model,
            is_outer_timeout,
        } => {
            if reason
                == "ACP node did not produce valid structured output matching output_schema after 1 repair turn"
            {
                update_pipeline_node_status(
                    context,
                    session_id,
                    node_id,
                    "failed",
                    None,
                    acp_session_id.as_deref(),
                    thinking_session_id.as_deref(),
                    None,
                    Some(&reason),
                    input_tokens,
                    output_tokens,
                    model.as_deref(),
                )?;
                counts.abandoned += 1;
                return Ok(());
            }
            if phase.is_retryable() && !is_outer_timeout {
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
                input_tokens,
                output_tokens,
                model.as_deref(),
            )?;
            counts.abandoned += 1;
        }
    }
    Ok(())
}
