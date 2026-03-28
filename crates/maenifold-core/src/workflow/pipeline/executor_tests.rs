use std::collections::HashMap;

use serde_json::{Value, json};

use crate::tools::ToolContext;
use crate::workflow::pipeline::{EdgeType, NodeState, PipelineDefinition, PipelineEdge};

use super::executor_helpers::DispatchedNode;
use super::executor_mapping::resolve_dot_path;
use super::executor_nodes::{NodeExecutionRequest, build_node_execution_request};
use super::executor_tests_support::{
    completed_node_state, pending_node_state, pipeline_test_context, source_tool_node,
    target_tool_node,
};

#[test]
fn resolve_dot_path_handles_nested_object_and_arrays() {
    let value = json!({
        "results": [{"title": "First"}],
        "data": {"content": "Body"}
    });

    assert_eq!(resolve_dot_path(&value, "results.0.title"), json!("First"));
    assert_eq!(resolve_dot_path(&value, "results[0].title"), json!("First"));
    assert_eq!(resolve_dot_path(&value, "data.content"), json!("Body"));
}

#[test]
fn build_tool_request_applies_mapped_args_from_completed_source() {
    let (_tmp, context, _reset) = pipeline_test_context(crate::util::json::OutputFormat::Json);
    let tool_args = build_tool_request_args(
        &context,
        PipelineDefinition {
            nodes: vec![
                source_tool_node("source"),
                target_tool_node(Some(json!({"static": "keep"}))),
            ],
            edges: vec![PipelineEdge {
                source: "source".to_string(),
                target: "tool".to_string(),
                edge_type: EdgeType::Success,
                mapping: Some(HashMap::from([
                    ("title".to_string(), "results.0.title".to_string()),
                    ("count".to_string(), "meta.count".to_string()),
                ])),
            }],
        },
        HashMap::from([
            completed_node_state(
                "source",
                "pipeline-executor-mapped-source",
                json!({"results": [{"title": "Resolved"}], "meta": {"count": 2}}),
            ),
            pending_node_state("tool", "pipeline-executor-mapped-tool"),
        ]),
    );

    assert_eq!(
        tool_args,
        Some(json!({"static": "keep", "title": "Resolved", "count": 2}))
    );
}

#[test]
fn build_tool_request_keeps_static_args_when_edge_has_no_mapping() {
    let (_tmp, context, _reset) = pipeline_test_context(crate::util::json::OutputFormat::Json);
    let tool_args = build_tool_request_args(
        &context,
        PipelineDefinition {
            nodes: vec![
                source_tool_node("source"),
                target_tool_node(Some(json!({"query": "static"}))),
            ],
            edges: vec![PipelineEdge {
                source: "source".to_string(),
                target: "tool".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            }],
        },
        HashMap::from([
            completed_node_state(
                "source",
                "pipeline-executor-static-source",
                json!({"query": "mapped"}),
            ),
            pending_node_state("tool", "pipeline-executor-static-tool"),
        ]),
    );

    assert_eq!(tool_args, Some(json!({"query": "static"})));
}

#[test]
fn build_tool_request_sets_null_for_unresolved_paths() {
    let (_tmp, context, _reset) = pipeline_test_context(crate::util::json::OutputFormat::Json);
    let tool_args = build_tool_request_args(
        &context,
        PipelineDefinition {
            nodes: vec![source_tool_node("source"), target_tool_node(None)],
            edges: vec![PipelineEdge {
                source: "source".to_string(),
                target: "tool".to_string(),
                edge_type: EdgeType::Success,
                mapping: Some(HashMap::from([(
                    "missing".to_string(),
                    "results[9].title".to_string(),
                )])),
            }],
        },
        HashMap::from([
            completed_node_state(
                "source",
                "pipeline-executor-null-source",
                json!({"results": [{"title": "Resolved"}]}),
            ),
            pending_node_state("tool", "pipeline-executor-null-tool"),
        ]),
    );

    assert_eq!(tool_args, Some(json!({"missing": Value::Null})));
}

#[test]
fn build_tool_request_mapping_overrides_static_values() {
    let (_tmp, context, _reset) = pipeline_test_context(crate::util::json::OutputFormat::Json);
    let tool_args = build_tool_request_args(
        &context,
        PipelineDefinition {
            nodes: vec![
                source_tool_node("source"),
                target_tool_node(Some(json!({"title": "Static", "mode": "keep"}))),
            ],
            edges: vec![PipelineEdge {
                source: "source".to_string(),
                target: "tool".to_string(),
                edge_type: EdgeType::Success,
                mapping: Some(HashMap::from([(
                    "title".to_string(),
                    "results[0].title".to_string(),
                )])),
            }],
        },
        HashMap::from([
            completed_node_state(
                "source",
                "pipeline-executor-override-source",
                json!({"results": [{"title": "Resolved"}]}),
            ),
            pending_node_state("tool", "pipeline-executor-override-tool"),
        ]),
    );

    assert_eq!(
        tool_args,
        Some(json!({"title": "Resolved", "mode": "keep"}))
    );
}

fn build_tool_request_args(
    context: &ToolContext,
    definition: PipelineDefinition,
    node_states: HashMap<String, NodeState>,
) -> Option<serde_json::Value> {
    let tool_node = definition
        .nodes
        .iter()
        .find(|node| node.id() == "tool")
        .expect("tool node");

    match build_node_execution_request(
        context,
        &definition,
        &node_states,
        tool_node,
        &DispatchedNode {
            node_id: "tool".to_string(),
            session_id: None,
        },
        "Ambient context",
    )
    .expect("build node execution request")
    {
        NodeExecutionRequest::Tool { tool_args, .. } => tool_args,
        other => panic!("expected tool execution request, got {other:?}"),
    }
}
