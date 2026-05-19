#![allow(unused_imports)]

use std::path::Path;
use std::{collections::HashMap, fs};

use serde_json::json;
use serial_test::serial;

use crate::error::MaenifoldError;
use crate::util::json::OutputFormat;

use super::super::executor_helpers::DispatchedNode;
use super::super::executor_nodes_cwd::resolve_node_cwd;
use super::super::executor_run::run_tool_node;
use super::super::test_support::pipeline_test_context;
use super::super::types::NodeStatus;
use super::super::{NodeState, PipelineDefinition, PipelineNode};
use super::*;

#[test]
#[serial]
fn build_node_execution_request_interpolates_sequential_thinking_topic() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let inputs = serde_json::Map::from_iter([("name".to_string(), json!("world"))]);
    let node = PipelineNode::SequentialThinking {
        id: "think".to_string(),
        display_label: None,
        topic: "Plan for {{inputs.name}}".to_string(),
        total_thoughts: 3,
        agent: "agent-think".to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
        join_policy: None,
    };
    let dispatch = DispatchedNode {
        node_id: node.id().to_string(),
        session_id: Some("workflow-1700000000000-think".to_string()),
    };

    let request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &node,
        &dispatch,
        "",
        &inputs,
    )
    .expect("build sequential thinking request");

    match request {
        NodeExecutionRequest::SequentialThinking {
            topic,
            prompt_context,
            ..
        } => {
            assert_eq!(topic, "Plan for world");
            assert_eq!(prompt_context, "Plan for world");
        }
        other => panic!("expected sequential thinking request, got {other:?}"),
    }
}

#[tokio::test]
#[serial]
async fn run_tool_node_preserves_local_tool_registry_path_for_non_namespaced_tools() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

    let output = run_tool_node(
        &context,
        "GetConfig",
        None,
        None,
        None,
        std::path::Path::new("."),
    )
    .await
    .expect("local tool should execute through ToolRegistry");

    assert!(output.response.contains("memoryPath"));
}

#[tokio::test]
#[serial]
async fn run_tool_node_preserves_local_tool_registry_path_for_maenifold_namespace() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

    let output = run_tool_node(
        &context,
        "maenifold.GetConfig",
        None,
        None,
        None,
        std::path::Path::new("."),
    )
    .await
    .expect("maenifold namespaced tool should execute through ToolRegistry");

    assert!(output.response.contains("memoryPath"));
}

#[tokio::test]
#[serial]
async fn run_tool_node_rejects_external_tool_not_in_explicit_allowlist() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

    let error = run_tool_node(
        &context,
        "server.echo",
        Some(&json!({ "value": 1 })),
        Some("test-agent"),
        Some(&["server.other".to_string()]),
        std::path::Path::new("."),
    )
    .await
    .expect_err("external MCP tool not in explicit list should be rejected");

    assert!(matches!(error, MaenifoldError::AgentSetup(_)));
    assert!(
        error
            .to_string()
            .contains("not in the explicit allowed_tools")
    );
}

#[tokio::test]
#[serial]
async fn run_tool_node_allows_external_tools_by_default_when_no_allowlist() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

    let error = run_tool_node(
        &context,
        "server.echo",
        Some(&json!({ "value": 1 })),
        None,
        None,
        std::path::Path::new("."),
    )
    .await
    .expect_err("should fail at server resolution, not at tool permission");

    assert!(
        error.to_string().contains("not found"),
        "with no allowlist restriction, the error should be about missing server config, not permissions: {error}"
    );
}

// --- resolve_node_cwd unit tests ---

#[test]
#[serial]
fn resolve_node_cwd_none_returns_maenifold_root() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let result = resolve_node_cwd(&context, None).expect("should resolve");
    assert_eq!(
        result,
        context.config.maenifold_root.canonicalize().unwrap()
    );
}

#[test]
#[serial]
fn resolve_node_cwd_absolute_valid_dir() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let sub = context.config.maenifold_root.join("abs_dir");
    fs::create_dir_all(&sub).expect("create sub dir");
    let result = resolve_node_cwd(&context, Some(sub.as_path())).expect("should resolve");
    assert_eq!(result, sub.canonicalize().unwrap());
}

#[test]
#[serial]
fn resolve_node_cwd_relative_valid_dir() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let sub = context.config.maenifold_root.join("rel_child");
    fs::create_dir_all(&sub).expect("create sub dir");
    let result = resolve_node_cwd(&context, Some(Path::new("rel_child"))).expect("should resolve");
    assert_eq!(result, sub.canonicalize().unwrap());
}

#[test]
#[serial]
fn resolve_node_cwd_nonexistent_path_errors() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let err = resolve_node_cwd(&context, Some(Path::new("no_such_dir")))
        .expect_err("should fail for nonexistent path");
    assert!(
        err.to_string().contains("could not be resolved"),
        "unexpected error: {err}"
    );
}

#[test]
#[serial]
fn resolve_node_cwd_file_not_dir_errors() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let file_path = context.config.maenifold_root.join("a_file.txt");
    fs::write(&file_path, "not a directory").expect("write file");
    let err = resolve_node_cwd(&context, Some(file_path.as_path()))
        .expect_err("should fail for file path");
    assert!(
        err.to_string().contains("is not a directory"),
        "unexpected error: {err}"
    );
}

#[test]
#[serial]
fn resolve_node_cwd_relative_traversal_escape_errors() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let sibling = context
        .config
        .maenifold_root
        .parent()
        .expect("root parent")
        .join("sibling-project");
    fs::create_dir_all(&sibling).expect("create sibling dir");
    let err = resolve_node_cwd(&context, Some(Path::new("../sibling-project")))
        .expect_err("relative traversal outside root should fail");
    assert!(
        err.to_string().contains("escapes maenifold root"),
        "unexpected error: {err}"
    );
}

#[test]
#[serial]
fn resolve_node_cwd_absolute_escape_errors() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let escaped = context
        .config
        .maenifold_root
        .parent()
        .expect("root parent")
        .join("absolute-escape");
    fs::create_dir_all(&escaped).expect("create escaped dir");

    let err = resolve_node_cwd(&context, Some(escaped.as_path()))
        .expect_err("absolute cwd outside root should fail");

    assert!(
        err.to_string().contains("escapes maenifold root"),
        "unexpected error: {err}"
    );
}
