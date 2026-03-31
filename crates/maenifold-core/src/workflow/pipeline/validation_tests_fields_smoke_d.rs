use super::handler::pipeline;
use super::test_support::*;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;

fn view_node(node: Value) -> Value {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let create_output = pipeline(
        &context,
        &json!({ "definition": { "nodes": [node], "edges": [] } }),
    )
    .expect("create pipeline");
    let create_response: Value =
        serde_json::from_str(&create_output).expect("parse create response");
    let pipeline_id = create_response["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id");
    let view_output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
        .expect("view pipeline");
    let view_response: Value = serde_json::from_str(&view_output).expect("parse view response");
    view_response["data"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.first())
        .cloned()
        .expect("viewed node")
}

fn assert_view_field(node: Value, field: &str, expected: Value) {
    assert_eq!(view_node(node)[field], expected);
}

#[test]
#[serial(config)]
fn test_smoke_nf55_model_on_tool_node() {
    assert_view_field(
        json!({ "id": "tool-model", "type": "tool", "toolName": "ReadMemory", "model": "gpt-5.2" }),
        "model",
        json!("gpt-5.2"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf56_thought_level_on_tool_node() {
    assert_view_field(
        json!({ "id": "tool-thought", "type": "tool", "toolName": "ReadMemory", "thoughtLevel": "low" }),
        "thoughtLevel",
        json!("low"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf57_allowed_tools_on_thinking_node() {
    assert_view_field(
        json!({ "id": "think-tools", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent", "allowedTools": ["BuildContext"] }),
        "allowedTools",
        json!(["BuildContext"]),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf58_max_turns_on_thinking_node() {
    assert_view_field(
        json!({ "id": "think-turns", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent", "maxTurns": 4 }),
        "maxTurns",
        json!(4),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf59_agent_config_on_thinking_node() {
    assert_view_field(
        json!({ "id": "think-config", "type": "sequentialThinking", "topic": "Review execution path", "agent": "think-agent", "agentConfig": { "model": "claude-sonnet-4.6", "mode": "batch", "thoughtLevel": "high", "configOptions": { "temperature": 0 } } }),
        "agentConfig",
        json!({ "model": "claude-sonnet-4.6", "mode": "batch", "thoughtLevel": "high", "configOptions": { "temperature": 0 } }),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf60_agent_config_on_tool_node() {
    assert_view_field(
        json!({ "id": "tool-config", "type": "tool", "toolName": "ReadMemory", "agentConfig": { "model": "gpt-5.1", "mode": "tool", "thoughtLevel": "medium", "configOptions": { "verbosity": "low" } } }),
        "agentConfig",
        json!({ "model": "gpt-5.1", "mode": "tool", "thoughtLevel": "medium", "configOptions": { "verbosity": "low" } }),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nc01_prompt_agent_config_fields_preserved() {
    assert_view_field(
        json!({ "id": "prompt-agent-config", "type": "prompt", "prompt": "Draft summary", "agent": "prompt-agent", "agentConfig": { "model": "gpt-5.4", "mode": "interactive", "thoughtLevel": "medium", "configOptions": { "sandbox": "workspace-write" } } }),
        "agentConfig",
        json!({ "model": "gpt-5.4", "mode": "interactive", "thoughtLevel": "medium", "configOptions": { "sandbox": "workspace-write" } }),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nc02_thinking_config_fields_preserved() {
    let node = view_node(json!({
        "id": "thinking-config",
        "type": "sequentialThinking",
        "topic": "Trace the pipeline execution path",
        "agent": "thinking-agent",
        "model": "claude-sonnet-4.6",
        "thoughtLevel": "deep",
        "allowedTools": ["BuildContext"],
        "maxTurns": 5,
        "timeout": 45,
        "agentConfig": {
            "model": "claude-sonnet-4.5",
            "mode": "batch",
            "thoughtLevel": "high",
            "configOptions": { "temperature": 0 }
        }
    }));
    assert_eq!(node["model"], "claude-sonnet-4.6");
    assert_eq!(node["thoughtLevel"], "deep");
    assert_eq!(node["allowedTools"], json!(["BuildContext"]));
    assert_eq!(node["maxTurns"], 5);
    assert_eq!(node["timeout"], 45);
    assert_eq!(
        node["agentConfig"],
        json!({ "model": "claude-sonnet-4.5", "mode": "batch", "thoughtLevel": "high", "configOptions": { "temperature": 0 } })
    );
}

#[test]
#[serial(config)]
fn test_smoke_nc03_tool_args_complex_nested_object_preserved() {
    let node = json!({
        "id": "complex-args", "type": "tool", "toolName": "ReadMemory",
        "toolArgs": {
            "identifier": "memory://test",
            "options": { "depth": 3, "includeContent": true },
            "tags": ["a", "b"]
        }
    });
    assert_view_field(
        node,
        "toolArgs",
        json!({
            "identifier": "memory://test",
            "options": { "depth": 3, "includeContent": true },
            "tags": ["a", "b"]
        }),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nc04_workflow_timeout_field_preserved() {
    assert_view_field(
        json!({ "id": "workflow-timeout", "workflowId": "test-workflow-a", "timeout": 120 }),
        "timeout",
        json!(120),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nc05_prompt_tool_mode_field_preserved() {
    assert_view_field(
        json!({ "id": "prompt-mode", "type": "prompt", "prompt": "Draft summary", "agent": "prompt-agent", "toolMode": "required" }),
        "toolMode",
        json!("required"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nc06_prompt_allowed_tools_field_preserved() {
    assert_view_field(
        json!({ "id": "prompt-allowed-tools", "type": "prompt", "prompt": "Draft summary", "agent": "prompt-agent", "allowedTools": ["ReadMemory", "SearchMemories"] }),
        "allowedTools",
        json!(["ReadMemory", "SearchMemories"]),
    );
}

#[test]
#[serial(config)]
fn test_smoke_nc07_prompt_max_turns_field_preserved() {
    assert_view_field(
        json!({ "id": "prompt-max-turns", "type": "prompt", "prompt": "Draft summary", "agent": "prompt-agent", "maxTurns": 7 }),
        "maxTurns",
        json!(7),
    );
}
