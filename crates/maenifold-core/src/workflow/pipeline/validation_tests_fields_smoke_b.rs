use super::handler::pipeline;
use super::test_support::*;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
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

// Used by #[ignore]d test nf27 — will become active when BUG: maxTurns=0 is fixed
#[allow(dead_code)]
fn create_error(node: Value) -> String {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    pipeline(
        &context,
        &json!({ "definition": { "nodes": [node], "edges": [] } }),
    )
    .expect_err("pipeline creation should fail")
    .to_string()
}

#[test]
#[serial(config)]
fn test_smoke_nf19_prompt_model_present() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-model",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "model": "gpt-5.4-mini"
        }))["model"],
        "gpt-5.4-mini"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf20_prompt_model_absent_defaults_null() {
    assert_eq!(
        view_node(
            json!({ "id": "prompt-model-null", "type": "prompt", "prompt": "Hello", "agent": "prompt-agent" })
        )["model"],
        Value::Null
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf21_prompt_thought_level_present() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-thought",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "thoughtLevel": "high"
        }))["thoughtLevel"],
        "high"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf22_prompt_thought_level_absent_defaults_null() {
    assert_eq!(
        view_node(
            json!({ "id": "prompt-thought-null", "type": "prompt", "prompt": "Hello", "agent": "prompt-agent" })
        )["thoughtLevel"],
        Value::Null
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf23_prompt_allowed_tools_array_of_strings() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-tools",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "allowedTools": ["ReadMemory", "SearchMemories"]
        }))["allowedTools"],
        json!(["ReadMemory", "SearchMemories"])
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf24_prompt_allowed_tools_empty_array() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-tools-empty",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "allowedTools": []
        }))["allowedTools"],
        json!([])
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf25_prompt_allowed_tools_absent_defaults_null() {
    assert_eq!(
        view_node(
            json!({ "id": "prompt-tools-null", "type": "prompt", "prompt": "Hello", "agent": "prompt-agent" })
        )["allowedTools"],
        Value::Null
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf26_prompt_max_turns_positive_integer() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-max-turns",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "maxTurns": 4
        }))["maxTurns"],
        4
    );
}

#[test]
#[ignore]
// BUG: Pipeline accepts prompt maxTurns=0 instead of rejecting non-positive max turn counts.
#[serial(config)]
fn test_smoke_nf27_prompt_max_turns_zero() {
    let message = create_error(json!({
        "id": "prompt-max-turns-zero",
        "type": "prompt",
        "prompt": "Hello",
        "agent": "prompt-agent",
        "maxTurns": 0
    }));
    assert!(
        message.contains("maxTurns")
            && (message.contains("positive") || message.contains("greater than 0")),
        "unexpected error: {message}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf28_prompt_max_turns_absent_defaults_null() {
    assert_eq!(
        view_node(
            json!({ "id": "prompt-max-turns-null", "type": "prompt", "prompt": "Hello", "agent": "prompt-agent" })
        )["maxTurns"],
        Value::Null
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf29_prompt_agent_config_full_object() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-agent-config-full",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "agentConfig": {
                "model": "gpt-5.4",
                "mode": "interactive",
                "thoughtLevel": "medium",
                "configOptions": { "sandbox": "workspace-write" }
            }
        }))["agentConfig"],
        json!({
            "model": "gpt-5.4",
            "mode": "interactive",
            "thoughtLevel": "medium",
            "configOptions": { "sandbox": "workspace-write" }
        })
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf30_prompt_agent_config_only_model() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-agent-config-model",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "agentConfig": { "model": "gpt-5.4-mini" }
        }))["agentConfig"],
        json!({ "model": "gpt-5.4-mini" })
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf31_prompt_agent_config_only_mode() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-agent-config-mode",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "agentConfig": { "mode": "interactive" }
        }))["agentConfig"],
        json!({ "mode": "interactive" })
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf32_prompt_agent_config_only_config_options() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-agent-config-options",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "agentConfig": { "configOptions": { "sandbox": "workspace-write" } }
        }))["agentConfig"],
        json!({ "configOptions": { "sandbox": "workspace-write" } })
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf33_prompt_agent_config_empty_object() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-agent-config-empty",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "agentConfig": {}
        }))["agentConfig"],
        json!({})
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf34_prompt_agent_config_absent_defaults_null() {
    assert_eq!(
        view_node(
            json!({ "id": "prompt-agent-config-null", "type": "prompt", "prompt": "Hello", "agent": "prompt-agent" })
        )["agentConfig"],
        Value::Null
    );
}

#[test]
#[serial(config)]
fn test_smoke_nf35_prompt_config_options_multiple_entries() {
    assert_eq!(
        view_node(json!({
            "id": "prompt-agent-config-many",
            "type": "prompt",
            "prompt": "Hello",
            "agent": "prompt-agent",
            "agentConfig": {
                "configOptions": {
                    "sandbox": "workspace-write",
                    "approval-policy": "never"
                }
            }
        }))["agentConfig"],
        json!({
            "configOptions": {
                "approval-policy": "never",
                "sandbox": "workspace-write"
            }
        })
    );
}
