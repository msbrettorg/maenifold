use std::collections::HashMap;

use serde_json::json;

use super::pipeline_params::{PipelineEdgeSchema, PipelineEdgeType, PipelineParams};
use super::support::schema_value_no_null_options;
use crate::workflow::pipeline::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, compute_ready_set,
};

#[test]
fn pipeline_edge_schema_accepts_conditional_edge() {
    let edge: PipelineEdgeSchema = serde_json::from_value(json!({
        "source": "upstream",
        "target": "downstream",
        "type": "conditional",
        "path": "verdict",
        "equals": "FAIL"
    }))
    .expect("conditional edge should deserialize");

    match edge.edge_type {
        PipelineEdgeType::Conditional { path, equals } => {
            assert_eq!(path, "verdict");
            assert_eq!(equals, json!("FAIL"));
        }
        other => panic!("expected conditional edge, got {other:?}"),
    }
}

#[test]
fn pipeline_edge_schema_rejects_conditional_without_path() {
    let err = serde_json::from_value::<PipelineEdgeSchema>(json!({
        "source": "upstream",
        "target": "downstream",
        "type": "conditional",
        "equals": "FAIL"
    }))
    .expect_err("missing path must fail");

    assert!(err.to_string().contains("path"), "unexpected error: {err}");
}

#[test]
fn pipeline_edge_schema_rejects_conditional_without_equals() {
    let err = serde_json::from_value::<PipelineEdgeSchema>(json!({
        "source": "upstream",
        "target": "downstream",
        "type": "conditional",
        "path": "verdict"
    }))
    .expect_err("missing equals must fail");

    assert!(
        err.to_string().contains("equals"),
        "unexpected error: {err}"
    );
}

#[test]
fn pipeline_edge_schema_ignores_conditional_fields_on_success_edge() {
    let edge = serde_json::from_value::<PipelineEdgeSchema>(json!({
        "source": "upstream",
        "target": "downstream",
        "type": "success",
        "path": "verdict",
        "equals": "FAIL"
    }))
    .expect("success edge should deserialize when conditional-only fields are present");

    assert!(matches!(edge.edge_type, PipelineEdgeType::Success));
}

#[test]
fn pipeline_schema_includes_conditional_edge_fields() {
    let schema = schema_value_no_null_options::<PipelineParams>();
    let schema_text = serde_json::to_string(&schema).expect("serialize schema");

    assert!(schema_text.contains("\"conditional\""));
    assert!(schema_text.contains("\"path\""));
    assert!(schema_text.contains("\"equals\""));
}

#[test]
fn conditional_edge_deserializes_to_domain_and_drives_ready_set() {
    let definition: PipelineDefinition = serde_json::from_value(json!({
        "nodes": [
            {
                "id": "upstream",
                "type": "tool",
                "toolName": "GetConfig"
            },
            {
                "id": "downstream",
                "type": "tool",
                "toolName": "GetConfig"
            }
        ],
        "edges": [
            {
                "source": "upstream",
                "target": "downstream",
                "type": "conditional",
                "path": "verdict",
                "equals": "FAIL"
            }
        ]
    }))
    .expect("pipeline definition should deserialize");

    match &definition.edges[0].edge_type {
        EdgeType::Conditional { path, equals } => {
            assert_eq!(path, "verdict");
            assert_eq!(equals, &json!("FAIL"));
        }
        other => panic!("expected conditional domain edge, got {other:?}"),
    }

    let mut node_states = HashMap::new();
    node_states.insert(
        "upstream".to_string(),
        NodeState {
            session_id: "upstream-session".to_string(),
            status: NodeStatus::Completed,
            attempts: 1,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            output: Some(r#"{"verdict":"FAIL"}"#.to_string()),
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    );
    node_states.insert(
        "downstream".to_string(),
        NodeState {
            session_id: "downstream-session".to_string(),
            status: NodeStatus::Pending,
            attempts: 0,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            output: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    );

    let ready = compute_ready_set(&definition, &node_states);
    assert_eq!(ready, vec!["downstream".to_string()]);
}
