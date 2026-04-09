use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_yaml::Value as YamlValue;

use super::session_io::{
    derive_pipeline_node_states, frontmatter_value, persist_pipeline_state, read_pipeline_session,
    update_workflow_node_frontmatter,
};
use super::update_context::inject_predecessor_context;
use super::view::build_pipeline_view_result;
use super::{NodeStatus, PipelineDefinition, compute_blocked_cascade};

#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
pub(super) fn update_pipeline_node_status(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    status: &str,
    conclusion: Option<&str>,
    thinking_session_id: Option<&str>,
    failure_reason: Option<&str>,
) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let raw_node_states = frontmatter_value(&frontmatter, "nodeStates")?;
    let mut node_states = derive_pipeline_node_states(context, &raw_node_states)?;
    definition
        .nodes
        .iter()
        .find(|node| node.id() == node_id)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline definition missing node '{node_id}'"
            ))
        })?;
    let workflow_session_id = {
        let state = raw_node_states.get(node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{node_id}'"
            ))
        })?;
        if state.status != NodeStatus::Active {
            return Err(MaenifoldError::Message(format!(
                "ERROR: Node '{node_id}' must be active to transition to {status}; current status is {:?}",
                state.status
            )));
        }
        state.session_id.clone()
    };

    match status {
        "completed" => {
            let conclusion = conclusion
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    MaenifoldError::Message(
                        "ERROR: Conclusion required when marking pipeline node completed"
                            .to_string(),
                    )
                })?;
            update_workflow_node_frontmatter(
                context,
                &workflow_session_id,
                "completed",
                Some(conclusion),
            )?;
            {
                let state = node_states.get_mut(node_id).ok_or_else(|| {
                    MaenifoldError::Message(format!(
                        "ERROR: Pipeline session missing node state for '{node_id}'"
                    ))
                })?;
                state.status = NodeStatus::Completed;
                state.thinking_session_id = thinking_session_id.map(str::to_string);
                state.output = Some(conclusion.to_string());
                state.failure_reason = None;
            }
            inject_predecessor_context(context, &definition, &node_states, node_id)?;
        }
        "abandoned" | "stuck" => {
            let transitioned_status = if status == "stuck" {
                NodeStatus::Stuck
            } else {
                NodeStatus::Abandoned
            };
            let state = node_states.get_mut(node_id).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{node_id}'"
                ))
            })?;
            state.status = transitioned_status;
            state.thinking_session_id = None;
            state.output = None;
            state.failure_reason = failure_reason
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);

            let cascaded_node_ids = compute_blocked_cascade(&definition, &node_states);
            for cascaded_node_id in &cascaded_node_ids {
                let cascaded_state = node_states.get_mut(cascaded_node_id).ok_or_else(|| {
                    MaenifoldError::Message(format!(
                        "ERROR: Pipeline session missing node state for '{cascaded_node_id}'"
                    ))
                })?;
                cascaded_state.status = NodeStatus::Abandoned;
                cascaded_state.thinking_session_id = None;
                cascaded_state.output = None;
                cascaded_state.failure_reason = Some("predecessor abandoned".to_string());
            }

            persist_pipeline_state(
                &mut frontmatter,
                &node_states,
                &uri,
                &pipeline_content,
                context,
            )?;
            update_workflow_node_frontmatter(context, &workflow_session_id, status, None)?;
            for cascaded_node_id in &cascaded_node_ids {
                let cascaded_session_id = node_states
                    .get(cascaded_node_id)
                    .ok_or_else(|| {
                        MaenifoldError::Message(format!(
                            "ERROR: Pipeline session missing node state for '{cascaded_node_id}'"
                        ))
                    })?
                    .session_id
                    .clone();
                update_workflow_node_frontmatter(
                    context,
                    &cascaded_session_id,
                    "abandoned",
                    None,
                )?;
            }
        }
        _ => {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Unsupported pipeline node status '{status}'"
            )));
        }
    }

    if !matches!(status, "abandoned" | "stuck") {
        persist_pipeline_state(
            &mut frontmatter,
            &node_states,
            &uri,
            &pipeline_content,
            context,
        )?;
    }
    tracing::info!(
        session_id = %session_id,
        node_id = %node_id,
        status = %status,
        "[pipeline] node transitioned"
    );

    let ambient_prompt = frontmatter.get("prompt").and_then(YamlValue::as_str);
    let result = build_pipeline_view_result(
        context,
        session_id,
        &definition,
        &node_states,
        ambient_prompt,
    )?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}
