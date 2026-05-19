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
use super::super::{NodeState, PipelineDefinition, PipelineNode, SessionRef};
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
        input_schema: None,
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
        join_policy: None,
        output_schema: None,
        session_ref: None,
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
        input_schema: None,
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
        join_policy: None,
        output_schema: None,
        session_ref: None,
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
        input_schema: None,
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
        join_policy: None,
        output_schema: None,
        session_ref: None,
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
fn build_node_execution_request_sets_resume_session_id_from_prompt_session_ref() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let node_states = HashMap::new();
    let node = PipelineNode::Prompt {
        id: "prompt".to_string(),
        display_label: None,
        prompt: "Prompt".to_string(),
        agent: "test-agent".to_string(),
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
        output_schema: None,
        session_ref: Some(SessionRef {
            agent: "test-agent".to_string(),
            session_id: "sess-123".to_string(),
        }),
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
            session_config.resume_session_id.as_deref(),
            Some("sess-123")
        ),
        NodeExecutionRequest::Workflow { .. }
        | NodeExecutionRequest::SequentialThinking { .. }
        | NodeExecutionRequest::Tool { .. } => {
            panic!("expected prompt node execution request")
        }
    }
}
