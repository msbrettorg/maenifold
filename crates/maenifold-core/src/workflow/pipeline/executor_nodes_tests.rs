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

#[allow(dead_code)]
#[allow(dead_code)]
#[test]
#[serial]
fn timeout_seconds_is_extracted_from_prompt_node() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let node = PipelineNode::Prompt {
        id: "prompt".to_string(),
        display_label: None,
        prompt: "Prompt".to_string(),
        agent: "agent-prompt".to_string(),
        tool_mode: "auto".to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: Some(30),
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
    };
    let dispatch = DispatchedNode {
        node_id: node.id().to_string(),
        session_id: Some("workflow-1700000000000-prompt".to_string()),
    };

    let request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &node,
        &dispatch,
        "",
        &serde_json::Map::new(),
    )
    .expect("build request");

    match request {
        NodeExecutionRequest::Prompt { session_config, .. } => {
            assert_eq!(session_config.timeout_seconds, Some(30));
        }
        NodeExecutionRequest::Workflow { .. }
        | NodeExecutionRequest::SequentialThinking { .. }
        | NodeExecutionRequest::Tool { .. } => {
            panic!("expected prompt node execution request")
        }
    }
}

#[test]
#[serial]
fn timeout_seconds_is_none_when_not_configured() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let node = PipelineNode::Prompt {
        id: "prompt".to_string(),
        display_label: None,
        prompt: "Prompt".to_string(),
        agent: "agent-prompt".to_string(),
        tool_mode: "auto".to_string(),
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
    };
    let dispatch = DispatchedNode {
        node_id: node.id().to_string(),
        session_id: Some("workflow-1700000000000-prompt".to_string()),
    };

    let request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &node,
        &dispatch,
        "",
        &serde_json::Map::new(),
    )
    .expect("build request");

    match request {
        NodeExecutionRequest::Prompt { session_config, .. } => {
            assert_eq!(session_config.timeout_seconds, None);
        }
        NodeExecutionRequest::Workflow { .. }
        | NodeExecutionRequest::SequentialThinking { .. }
        | NodeExecutionRequest::Tool { .. } => {
            panic!("expected prompt node execution request")
        }
    }
}

#[test]
#[serial]
fn build_node_execution_request_propagates_node_session_id_into_session_config() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let node = PipelineNode::Prompt {
        id: "prompt".to_string(),
        display_label: None,
        prompt: "Prompt".to_string(),
        agent: "agent-prompt".to_string(),
        tool_mode: "auto".to_string(),
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
    };
    let dispatch = DispatchedNode {
        node_id: node.id().to_string(),
        session_id: Some("workflow-1700000000000-prompt".to_string()),
    };

    let request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &node,
        &dispatch,
        "",
        &serde_json::Map::new(),
    )
    .expect("build request");

    match request {
        NodeExecutionRequest::Prompt { session_config, .. } => assert_eq!(
            session_config.session_id.as_deref(),
            Some("workflow-1700000000000-prompt")
        ),
        NodeExecutionRequest::Workflow { .. }
        | NodeExecutionRequest::SequentialThinking { .. }
        | NodeExecutionRequest::Tool { .. } => {
            panic!("expected prompt node execution request")
        }
    }
}

#[test]
#[serial]
fn timeout_seconds_is_extracted_from_tool_node() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let node = PipelineNode::Tool {
        id: "tool".to_string(),
        display_label: None,
        tool_name: "GetConfig".to_string(),
        tool_args: None,
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: Some(15),
        agent_config: None,
        retry_policy: None,
    };
    let dispatch = DispatchedNode {
        node_id: node.id().to_string(),
        session_id: Some("pipeline-1700000000000-tool".to_string()),
    };

    let request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &node,
        &dispatch,
        "",
        &serde_json::Map::new(),
    )
    .expect("build request");

    match request {
        NodeExecutionRequest::Tool {
            timeout_seconds, ..
        } => {
            assert_eq!(timeout_seconds, Some(15));
        }
        NodeExecutionRequest::Workflow { .. }
        | NodeExecutionRequest::Prompt { .. }
        | NodeExecutionRequest::SequentialThinking { .. } => {
            panic!("expected tool node execution request")
        }
    }
}

#[test]
fn node_timeout_produces_abandoned_outcome() {
    let outcome = classify_node_result(
        "prompt-timeout",
        NodeTaskResult::Prompt(Err(MaenifoldError::Message(
            "node timed out after 1s".to_string(),
        ))),
    );

    match outcome {
        NodeOutcome::Abandoned { reason, .. } => {
            let reason = reason.to_ascii_lowercase();
            assert!(
                reason.contains("timeout") || reason.contains("timed out"),
                "unexpected timeout reason: {reason}"
            );
        }
        NodeOutcome::Completed { .. } => {
            panic!("timeout errors should abandon the node")
        }
    }
}

#[test]
#[serial]
fn build_node_execution_request_errors_when_tool_mapping_fails() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
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
        }],
        edges: vec![super::super::types::PipelineEdge {
            source: "source".to_string(),
            target: "target".to_string(),
            edge_type: super::super::types::EdgeType::Success,
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
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            output: Some("not-json".to_string()),
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    )]);
    let node = definition.nodes.first().expect("target node");
    let dispatch = DispatchedNode {
        node_id: "target".to_string(),
        session_id: Some("pipeline-target".to_string()),
    };

    let error = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        node,
        &dispatch,
        "",
        &serde_json::Map::new(),
    )
    .expect_err("invalid mapped tool args should abort dispatch");

    assert!(
        error
            .to_string()
            .contains("could not parse node 'source' output as JSON")
    );
}

#[test]
#[serial]
fn build_node_execution_request_interpolates_inputs_in_prompt_and_tool_args() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let inputs = serde_json::Map::from_iter([("name".to_string(), json!("world"))]);

    let prompt_node = PipelineNode::Prompt {
        id: "prompt".to_string(),
        display_label: None,
        prompt: "Hello {{inputs.name}}".to_string(),
        agent: "agent-prompt".to_string(),
        tool_mode: "auto".to_string(),
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
    };
    let dispatch = DispatchedNode {
        node_id: prompt_node.id().to_string(),
        session_id: Some("workflow-1700000000000-prompt".to_string()),
    };
    let prompt_request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &prompt_node,
        &dispatch,
        "",
        &inputs,
    )
    .expect("build prompt request");
    match prompt_request {
        NodeExecutionRequest::Prompt { prompt, .. } => assert_eq!(prompt, "Hello world"),
        other => panic!("expected prompt request, got {other:?}"),
    }

    let tool_node = PipelineNode::Tool {
        id: "tool".to_string(),
        display_label: None,
        tool_name: "GetConfig".to_string(),
        tool_args: Some(json!({
            "message": "Hello {{inputs.name}}",
            "typed": { "$ref": "inputs.name" }
        })),
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        retry_policy: None,
    };
    let tool_dispatch = DispatchedNode {
        node_id: tool_node.id().to_string(),
        session_id: Some("workflow-1700000000000-tool".to_string()),
    };
    let tool_request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &tool_node,
        &tool_dispatch,
        "",
        &inputs,
    )
    .expect("build tool request");
    match tool_request {
        NodeExecutionRequest::Tool { tool_args, .. } => assert_eq!(
            tool_args,
            Some(json!({ "message": "Hello world", "typed": "world" }))
        ),
        other => panic!("expected tool request, got {other:?}"),
    }
}

#[test]
#[serial]
fn build_node_execution_request_does_not_interpolate_workflow_id_or_tool_name() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let inputs = serde_json::Map::from_iter([
        ("workflow".to_string(), json!("resolved.workflow")),
        ("tool".to_string(), json!("GetConfig")),
        ("name".to_string(), json!("world")),
    ]);

    let workflow_node = PipelineNode::Workflow {
        id: "workflow".to_string(),
        display_label: None,
        workflow_id: "{{inputs.workflow}}".to_string(),
        prompt: Some("Hello {{inputs.name}}".to_string()),
        cwd: None,
        timeout: None,
        acp_client: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
    };
    let workflow_dispatch = DispatchedNode {
        node_id: workflow_node.id().to_string(),
        session_id: Some("workflow-1700000000000-workflow".to_string()),
    };
    let workflow_request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &workflow_node,
        &workflow_dispatch,
        "",
        &inputs,
    )
    .expect("build workflow request");
    match workflow_request {
        NodeExecutionRequest::Workflow { workflow_id, .. } => {
            assert_eq!(workflow_id, "{{inputs.workflow}}");
        }
        other => panic!("expected workflow request, got {other:?}"),
    }

    let tool_node = PipelineNode::Tool {
        id: "tool".to_string(),
        display_label: None,
        tool_name: "{{inputs.tool}}".to_string(),
        tool_args: Some(json!({ "message": "Hello {{inputs.name}}" })),
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        retry_policy: None,
    };
    let tool_dispatch = DispatchedNode {
        node_id: tool_node.id().to_string(),
        session_id: Some("workflow-1700000000000-tool".to_string()),
    };
    let tool_request = build_node_execution_request(
        &context,
        &definition,
        &node_states,
        &tool_node,
        &tool_dispatch,
        "",
        &inputs,
    )
    .expect("build tool request");
    match tool_request {
        NodeExecutionRequest::Tool {
            tool_name,
            tool_args,
            ..
        } => {
            assert_eq!(tool_name, "{{inputs.tool}}");
            assert_eq!(tool_args, Some(json!({ "message": "Hello world" })));
        }
        other => panic!("expected tool request, got {other:?}"),
    }
}

#[test]
#[serial]
fn build_node_execution_request_interpolates_sequential_thinking_topic() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let inputs = serde_json::Map::from_iter([("name".to_string(), json!("world"))]);
    let node = PipelineNode::SequentialThinking {
        id: "think".to_string(),
        display_label: None,
        topic: "Plan for {{inputs.name}}".to_string(),
        total_thoughts: 3,
        analysis_type: None,
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
fn resolve_node_cwd_relative_traversal_resolves() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let sibling = context
        .config
        .maenifold_root
        .parent()
        .expect("root parent")
        .join("sibling-project");
    fs::create_dir_all(&sibling).expect("create sibling dir");
    let result =
        resolve_node_cwd(&context, Some(Path::new("../sibling-project"))).expect("should resolve");
    let expected = sibling.canonicalize().unwrap();
    assert_eq!(result, expected);
}
