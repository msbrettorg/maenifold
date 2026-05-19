#![allow(unused_imports)]

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
            join_policy: None,
        }],
        edges: vec![super::super::types::PipelineEdge {
            source: "source".to_string(),
            target: "target".to_string(),
            edge_type: super::super::types::EdgeType::Success,
            mode: Default::default(),
            mapping: Some(HashMap::from([(
                "payload".to_string(),
                "result.value".to_string(),
            )])),
        }],
        inputs: None,
        input_schema: None,
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
            input_artifact: None,
            output_artifact: None,
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
        input_schema: None,
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
        join_policy: None,
        input_schema: None,
        output_schema: None,
        session_ref: None,
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
        join_policy: None,
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
        input_schema: None,
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
        join_policy: None,
        input_schema: None,
        output_schema: None,
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
        join_policy: None,
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
