use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use serde_json::Value as JsonValue;
use serde_yml::Value as YamlValue;

use super::NodeStatus;
use super::PipelineNode;
use super::compute_blocked_cascade;
use super::engine_ready::compute_unreachable_conditional_failures;
use super::mapping_resolve::{conditional_output_has_json, conditional_parse_failure_reason};
use super::session_io::{
    derive_pipeline_node_states, frontmatter_value, persist_pipeline_state,
    pipeline_definition_from_frontmatter, read_pipeline_session, update_workflow_node_frontmatter,
};
use super::types::ToolProvenance;
use super::update_context::inject_predecessor_context;
use super::view::build_pipeline_view_result;

const ACP_OUTPUT_SCHEMA_FAILURE: &str =
    "ACP node did not produce valid structured output matching output_schema after 1 repair turn";

fn validate_structured_output(conclusion: &str, schema: &JsonValue) -> Result<JsonValue> {
    let value = serde_json::from_str::<JsonValue>(conclusion).map_err(|error| {
        MaenifoldError::InvalidArgument(format!(
            "complete_node validation failed: invalid JSON arguments: {error}"
        ))
    })?;
    let validator = jsonschema::validator_for(schema).map_err(|error| {
        MaenifoldError::InvalidArgument(format!("complete_node validation schema error: {error}"))
    })?;
    if let Err(error) = validator.validate(&value) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "complete_node validation failed: {error}"
        )));
    }
    Ok(value)
}

fn validate_tool_structured_output(conclusion: &str, schema: &JsonValue) -> Result<JsonValue> {
    if schema_declares_data_string(schema) {
        let value = serde_json::json!({ "data": conclusion });
        validate_json_value_against_schema(value, schema)
    } else {
        validate_structured_output(conclusion, schema)
    }
}

fn validate_json_value_against_schema(value: JsonValue, schema: &JsonValue) -> Result<JsonValue> {
    let validator = jsonschema::validator_for(schema).map_err(|error| {
        MaenifoldError::InvalidArgument(format!("complete_node validation schema error: {error}"))
    })?;
    if let Err(error) = validator.validate(&value) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "complete_node validation failed: {error}"
        )));
    }
    Ok(value)
}

fn schema_declares_data_string(schema: &JsonValue) -> bool {
    schema
        .get("properties")
        .and_then(JsonValue::as_object)
        .and_then(|properties| properties.get("data"))
        .and_then(|data| data.get("type"))
        .and_then(JsonValue::as_str)
        == Some("string")
}

#[allow(clippy::too_many_arguments)]
fn reject_invalid_structured_output(
    context: &ToolContext,
    frontmatter: &mut crate::markdown::Frontmatter,
    uri: &str,
    pipeline_content: &str,
    node_states: &mut std::collections::HashMap<String, super::NodeState>,
    node_id: &str,
    workflow_session_id: &str,
    reason: &str,
) -> Result<String> {
    let state = node_states.get_mut(node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        ))
    })?;
    let repair_count = state.retry_count.unwrap_or(0);
    state.output_artifact = None;
    state.failure_reason = Some(reason.to_string());
    if repair_count == 0 {
        state.retry_count = Some(1);
        persist_pipeline_state(frontmatter, node_states, uri, pipeline_content, context)?;
        return Err(MaenifoldError::InvalidArgument(reason.to_string()));
    }

    state.status = NodeStatus::Failed;
    state.completed_at = Some(Utc::now());
    state.output = None;
    state.failure_reason = Some(ACP_OUTPUT_SCHEMA_FAILURE.to_string());
    persist_pipeline_state(frontmatter, node_states, uri, pipeline_content, context)?;
    update_workflow_node_frontmatter(context, workflow_session_id, "failed", None)?;
    Err(MaenifoldError::InvalidArgument(
        ACP_OUTPUT_SCHEMA_FAILURE.to_string(),
    ))
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
pub(super) fn update_pipeline_node_status(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    status: &str,
    conclusion: Option<&str>,
    acp_session_id: Option<&str>,
    thinking_session_id: Option<&str>,
    tool_provenance: Option<&ToolProvenance>,
    failure_reason: Option<&str>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    model: Option<&str>,
) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition = pipeline_definition_from_frontmatter(&frontmatter)?;
    let raw_node_states = frontmatter_value(&frontmatter, "nodeStates")?;
    let mut node_states = derive_pipeline_node_states(context, &raw_node_states)?;
    let current_node = definition
        .nodes
        .iter()
        .find(|node| node.id() == node_id)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline definition missing node '{node_id}'"
            ))
        })?;
    let output_schema = current_node.output_schema().cloned();
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
            let output_artifact = if let Some(schema) = output_schema.as_ref() {
                let validation = if matches!(current_node, PipelineNode::Tool { .. }) {
                    validate_tool_structured_output(conclusion, schema)
                } else {
                    validate_structured_output(conclusion, schema)
                };
                match validation {
                    Ok(value) => Some(value),
                    Err(reason) => {
                        return reject_invalid_structured_output(
                            context,
                            &mut frontmatter,
                            &uri,
                            &pipeline_content,
                            &mut node_states,
                            node_id,
                            &workflow_session_id,
                            &reason.to_string(),
                        );
                    }
                }
            } else {
                None
            };
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
                state.completed_at = Some(Utc::now());
                state.acp_session_id = acp_session_id.map(str::to_string);
                state.thinking_session_id = thinking_session_id.map(str::to_string);
                state.tool_provenance = tool_provenance.cloned();
                state.input_tokens = input_tokens;
                state.output_tokens = output_tokens;
                state.model = model.map(str::to_string);
                state.output = Some(conclusion.to_string());
                state.output_artifact = output_artifact;
                state.failure_reason = None;
                state.retry_count = None;
            }
            if definition.edges.iter().any(|edge| {
                edge.source == node_id
                    && matches!(edge.edge_type, super::EdgeType::Conditional { .. })
            }) && !conditional_output_has_json(
                node_states
                    .get(node_id)
                    .and_then(|state| state.output_artifact.as_ref()),
                Some(conclusion),
            ) {
                let state = node_states.get_mut(node_id).ok_or_else(|| {
                    MaenifoldError::Message(format!(
                        "ERROR: Pipeline session missing node state for '{node_id}'"
                    ))
                })?;
                state.failure_reason = Some(conditional_parse_failure_reason(node_id));
            }
            inject_predecessor_context(context, &definition, &node_states, node_id)?;

            let unreachable_failures =
                compute_unreachable_conditional_failures(&definition, &node_states);
            for (unreachable_node_id, failure_reason) in &unreachable_failures {
                let unreachable_state =
                    node_states.get_mut(unreachable_node_id).ok_or_else(|| {
                        MaenifoldError::Message(format!(
                            "ERROR: Pipeline session missing node state for '{unreachable_node_id}'"
                        ))
                    })?;
                unreachable_state.status = NodeStatus::Abandoned;
                unreachable_state.completed_at = Some(Utc::now());
                unreachable_state.acp_session_id = None;
                unreachable_state.thinking_session_id = None;
                unreachable_state.tool_provenance = None;
                unreachable_state.input_tokens = None;
                unreachable_state.output_tokens = None;
                unreachable_state.model = None;
                unreachable_state.output = None;
                unreachable_state.output_artifact = None;
                unreachable_state.failure_reason = Some(failure_reason.clone());
            }

            let cascaded_node_ids = compute_blocked_cascade(&definition, &node_states);
            for cascaded_node_id in &cascaded_node_ids {
                let cascaded_state = node_states.get_mut(cascaded_node_id).ok_or_else(|| {
                    MaenifoldError::Message(format!(
                        "ERROR: Pipeline session missing node state for '{cascaded_node_id}'"
                    ))
                })?;
                cascaded_state.status = NodeStatus::Abandoned;
                cascaded_state.completed_at = Some(Utc::now());
                cascaded_state.acp_session_id = None;
                cascaded_state.thinking_session_id = None;
                cascaded_state.tool_provenance = None;
                cascaded_state.input_tokens = None;
                cascaded_state.output_tokens = None;
                cascaded_state.model = None;
                cascaded_state.output = None;
                cascaded_state.output_artifact = None;
                cascaded_state.failure_reason = Some("predecessor abandoned".to_string());
            }

            let mut inactive_node_ids = unreachable_failures
                .iter()
                .map(|(node_id, _)| node_id.clone())
                .collect::<Vec<_>>();
            inactive_node_ids.extend(cascaded_node_ids);
            for inactive_node_id in &inactive_node_ids {
                let inactive_session_id = node_states
                    .get(inactive_node_id)
                    .ok_or_else(|| {
                        MaenifoldError::Message(format!(
                            "ERROR: Pipeline session missing node state for '{inactive_node_id}'"
                        ))
                    })?
                    .session_id
                    .clone();
                update_workflow_node_frontmatter(context, &inactive_session_id, "abandoned", None)?;
            }
        }
        "failed" => {
            let state = node_states.get_mut(node_id).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{node_id}'"
                ))
            })?;
            state.status = NodeStatus::Failed;
            state.completed_at = Some(Utc::now());
            state.acp_session_id = acp_session_id.map(str::to_string);
            state.thinking_session_id = thinking_session_id.map(str::to_string);
            state.tool_provenance = None;
            state.input_tokens = input_tokens;
            state.output_tokens = output_tokens;
            state.model = model.map(str::to_string);
            state.output = None;
            state.output_artifact = None;
            state.failure_reason = failure_reason
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            update_workflow_node_frontmatter(context, &workflow_session_id, "failed", None)?;
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
            state.completed_at = Some(Utc::now());
            if let Some(acp_session_id) = acp_session_id {
                state.acp_session_id = Some(acp_session_id.to_string());
            }
            if let Some(thinking_session_id) = thinking_session_id {
                state.thinking_session_id = Some(thinking_session_id.to_string());
            }
            state.tool_provenance = None;
            state.input_tokens = input_tokens;
            state.output_tokens = output_tokens;
            state.model = model.map(str::to_string);
            state.output = None;
            state.output_artifact = None;
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
                cascaded_state.completed_at = Some(Utc::now());
                cascaded_state.acp_session_id = None;
                cascaded_state.thinking_session_id = None;
                cascaded_state.tool_provenance = None;
                cascaded_state.input_tokens = None;
                cascaded_state.output_tokens = None;
                cascaded_state.model = None;
                cascaded_state.output = None;
                cascaded_state.output_artifact = None;
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
                update_workflow_node_frontmatter(context, &cascaded_session_id, "abandoned", None)?;
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
        None,
    )?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}
