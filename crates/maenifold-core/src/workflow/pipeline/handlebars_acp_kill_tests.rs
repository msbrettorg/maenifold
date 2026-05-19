use std::collections::HashMap;

use serde_json::{Value as JsonValue, json};

use crate::util::json::OutputFormat;

use super::executor_helpers::DispatchedNode;
use super::executor_nodes::{NodeExecutionRequest, build_node_execution_request};
use super::test_support::pipeline_test_context;
use super::{PipelineDefinition, PipelineNode, validate_pipeline};

fn empty_definition() -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    }
}

fn prompt_node(prompt: &str) -> PipelineNode {
    PipelineNode::Prompt {
        id: "prompt".to_string(),
        display_label: None,
        prompt: prompt.to_string(),
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
        session_ref: None,
        join_policy: None,
        input_schema: None,
        output_schema: None,
    }
}

fn build_request(
    node: &PipelineNode,
    inputs: &serde_json::Map<String, JsonValue>,
) -> crate::error::Result<NodeExecutionRequest> {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    build_node_execution_request(
        &context,
        &empty_definition(),
        &HashMap::new(),
        node,
        &DispatchedNode {
            node_id: node.id().to_string(),
            session_id: Some(format!("workflow-1700000000000-{}", node.id())),
        },
        "",
        inputs,
    )
}

#[test]
fn acp_prompt_with_handlebars_tokens_passes_through_verbatim() {
    let prompt_node = prompt_node("use {{inputs.foo}} here");
    let inputs = serde_json::Map::from_iter([("foo".to_string(), json!("bar"))]);

    let request = build_request(&prompt_node, &inputs).expect("prompt request should build");

    let NodeExecutionRequest::Prompt { prompt, .. } = request else {
        panic!("expected prompt request");
    };
    assert!(prompt.contains("{{inputs.foo}}"));
}

#[test]
fn tool_args_with_handlebars_still_interpolated() {
    let tool_node = PipelineNode::Tool {
        id: "tool".to_string(),
        display_label: None,
        tool_name: "GetConfig".to_string(),
        tool_args: Some(json!({ "x": "{{inputs.foo}}" })),
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
        input_schema: None,
        output_schema: None,
    };
    let inputs = serde_json::Map::from_iter([("foo".to_string(), json!("bar"))]);

    let request = build_request(&tool_node, &inputs).expect("tool request should build");

    let NodeExecutionRequest::Tool { tool_args, .. } = request else {
        panic!("expected tool request");
    };
    assert_eq!(tool_args, Some(json!({ "x": "bar" })));
}

#[test]
fn acp_prompt_handlebars_warning_logged_on_validation() {
    let definition = PipelineDefinition {
        nodes: vec![prompt_node("use {{inputs.foo}} here")],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };

    validate_pipeline(&definition).expect("warning must not reject validation");
}
