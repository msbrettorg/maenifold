use std::collections::HashMap;

use serde_json::Value as JsonValue;

use crate::error::{MaenifoldError, Result};

use super::mapping_resolve::{resolve_dot_path, resolve_template_value};
use super::{NodeState, NodeStatus, PipelineDefinition};

pub(super) fn mapped_tool_args(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    target_node_id: &str,
    static_args: Option<JsonValue>,
    scope: &JsonValue,
) -> Result<Option<JsonValue>> {
    let static_args = static_args
        .as_ref()
        .map(|value| resolve_template_value(value, scope))
        .transpose()?;
    let mut merged_args = match static_args.clone() {
        Some(JsonValue::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    let mut applied_mapping = false;

    for edge in definition
        .edges
        .iter()
        .filter(|edge| edge.target == target_node_id)
    {
        let Some(mapping) = edge.mapping.as_ref() else {
            continue;
        };
        let Some(source_state) = node_states.get(&edge.source) else {
            continue;
        };
        if source_state.status != NodeStatus::Completed {
            continue;
        }

        let source_output = match source_state.output.as_deref() {
            Some(value) => serde_json::from_str::<JsonValue>(value).map_err(|error| {
                MaenifoldError::InvalidArgument(format!(
                    "Pipeline mapping from '{}' to '{}' could not parse node '{}' output as JSON: {error}",
                    edge.source, edge.target, edge.source
                ))
            })?,
            None => JsonValue::Null,
        };
        for (target_key, source_path) in mapping {
            applied_mapping = true;
            let mapped_value = resolve_dot_path(&source_output, source_path).map_err(|error| {
                MaenifoldError::InvalidArgument(format!(
                    "Pipeline mapping from '{}' to '{}' for target '{}' failed: {error}",
                    edge.source, edge.target, target_key
                ))
            })?;
            merged_args.insert(target_key.clone(), mapped_value);
        }
    }

    if applied_mapping {
        Ok(Some(JsonValue::Object(merged_args)))
    } else {
        Ok(static_args)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;
    use crate::workflow::pipeline::{EdgeType, PipelineEdge, PipelineNode};

    #[test]
    fn mapped_tool_args_returns_error_for_invalid_source_json() {
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![PipelineEdge {
                source: "source".to_string(),
                target: "target".to_string(),
                edge_type: EdgeType::Success,
                mode: Default::default(),
                mapping: Some(HashMap::from([(
                    "payload".to_string(),
                    "result.value".to_string(),
                )])),
            }],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::from([(
            "source".to_string(),
            NodeState {
                session_id: "pipeline-source".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                started_at: None,
                completed_at: None,
                acp_session_id: None,
                thinking_session_id: None,
                tool_provenance: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                output: Some("not-json".to_string()),
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        )]);

        let error = mapped_tool_args(&definition, &node_states, "target", None, &json!({}))
            .expect_err("invalid source output should error");

        assert!(
            error
                .to_string()
                .contains("could not parse node 'source' output as JSON")
        );
    }

    #[test]
    fn mapped_tool_args_returns_error_for_missing_source_path() {
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![PipelineEdge {
                source: "source".to_string(),
                target: "target".to_string(),
                edge_type: EdgeType::Success,
                mode: Default::default(),
                mapping: Some(HashMap::from([(
                    "payload".to_string(),
                    "result.value".to_string(),
                )])),
            }],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::from([(
            "source".to_string(),
            NodeState {
                session_id: "pipeline-source".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                started_at: None,
                completed_at: None,
                acp_session_id: None,
                thinking_session_id: None,
                tool_provenance: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                output: Some(json!({ "result": {} }).to_string()),
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        )]);

        let error = mapped_tool_args(&definition, &node_states, "target", None, &json!({}))
            .expect_err("missing mapping path should error");

        assert!(error.to_string().contains("target 'payload' failed"));
        assert!(error.to_string().contains("missing segment 'value'"));
    }

    #[test]
    fn mapped_tool_args_merges_resolved_values_into_static_args() {
        let definition = PipelineDefinition {
            nodes: vec![PipelineNode::Tool {
                id: "target".to_string(),
                display_label: None,
                tool_name: "noop".to_string(),
                tool_args: None,
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                retry_policy: None,
                join_policy: None,
            }],
            edges: vec![PipelineEdge {
                source: "source".to_string(),
                target: "target".to_string(),
                edge_type: EdgeType::Success,
                mode: Default::default(),
                mapping: Some(HashMap::from([(
                    "payload".to_string(),
                    "result.value".to_string(),
                )])),
            }],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::from([(
            "source".to_string(),
            NodeState {
                session_id: "pipeline-source".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                started_at: None,
                completed_at: None,
                acp_session_id: None,
                thinking_session_id: None,
                tool_provenance: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                output: Some(json!({ "result": { "value": 7 } }).to_string()),
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        )]);

        let mapped = mapped_tool_args(
            &definition,
            &node_states,
            "target",
            Some(json!({ "static": true })),
            &json!({}),
        )
        .expect("mapping should succeed");

        assert_eq!(mapped, Some(json!({ "static": true, "payload": 7 })));
    }
}
