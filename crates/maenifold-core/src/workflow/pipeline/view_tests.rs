use super::handler::pipeline;
use super::test_support::*;
use super::view::build_pipeline_view_result;
use super::*;
use crate::markdown::append_h2_section;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;

fn workflow_node(id: &str, workflow_id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        workflow_id: workflow_id.to_string(),
        acp_client: None,
        prompt: None,
    }
}

fn prompt_node(id: &str, prompt: &str, agent: &str) -> PipelineNode {
    PipelineNode::Prompt {
        id: id.to_string(),
        prompt: prompt.to_string(),
        agent: agent.to_string(),
        tool_mode: "default".to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    }
}

fn thinking_node(
    id: &str,
    topic: &str,
    total_thoughts: u32,
    analysis_type: Option<&str>,
    agent: &str,
) -> PipelineNode {
    PipelineNode::SequentialThinking {
        id: id.to_string(),
        topic: topic.to_string(),
        total_thoughts,
        analysis_type: analysis_type.map(str::to_string),
        agent: agent.to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    }
}

fn tool_node(
    id: &str,
    tool_name: &str,
    tool_args: Option<Value>,
    agent: Option<&str>,
) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        tool_name: tool_name.to_string(),
        tool_args,
        agent: agent.map(str::to_string),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    }
}

fn edge(source: &str, target: &str, edge_type: EdgeType) -> PipelineEdge {
    PipelineEdge {
        source: source.to_string(),
        target: target.to_string(),
        edge_type,
        mapping: None,
    }
}

fn node_state(
    session_id: &str,
    status: NodeStatus,
    attempts: u32,
    output: Option<&str>,
) -> NodeState {
    node_state_with_thinking_session(session_id, status, attempts, None, output)
}

fn node_state_with_thinking_session(
    session_id: &str,
    status: NodeStatus,
    attempts: u32,
    thinking_session_id: Option<&str>,
    output: Option<&str>,
) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status,
        attempts,
        thinking_session_id: thinking_session_id.map(str::to_string),
        output: output.map(str::to_string),
    }
}

#[path = "view_tests_prompt.rs"]
mod prompt;
#[path = "view_tests_render.rs"]
mod render;
#[path = "view_tests_thinking.rs"]
mod thinking;
#[path = "view_tests_workflow.rs"]
mod workflow;
