use super::support::{integer_property, object_schema, string_array_property, string_property};
use serde_json::{Value, json};

pub(super) fn pipeline_node_schema() -> Value {
    json!({
        "oneOf": [
            workflow_node_schema(),
            prompt_node_schema(),
            sequential_thinking_node_schema(),
            tool_node_schema()
        ]
    })
}

fn workflow_node_schema() -> Value {
    object_schema(
        vec![
            ("id", string_property("New node identifier")),
            (
                "type",
                json!({
                    "type": "string",
                    "description": "Workflow node type",
                    "enum": ["workflow"]
                }),
            ),
            ("workflowId", string_property("Workflow definition to run")),
            ("acpClient", string_property("Optional ACP client")),
            (
                "prompt",
                string_property("Optional prompt for workflow nodes"),
            ),
            ("timeout", integer_property("Optional timeout in seconds")),
        ],
        &["id", "type", "workflowId"],
    )
}

fn prompt_node_schema() -> Value {
    object_schema(
        vec![
            ("id", string_property("New node identifier")),
            ("type", node_type_property("prompt")),
            ("prompt", string_property("Prompt text for prompt nodes")),
            ("agent", string_property("ACP agent for prompt nodes")),
            ("toolMode", string_property("Tool mode for prompt nodes")),
            ("model", string_property("Model override for prompt nodes")),
            (
                "thoughtLevel",
                string_property("Thought level override for prompt nodes"),
            ),
            (
                "allowedTools",
                string_array_property("Allowed tools override for prompt nodes"),
            ),
            (
                "maxTurns",
                integer_property("Maximum turns for prompt nodes"),
            ),
            ("timeout", integer_property("Optional timeout in seconds")),
            ("agentConfig", agent_config_schema()),
        ],
        &["id", "type", "prompt", "agent"],
    )
}

fn sequential_thinking_node_schema() -> Value {
    object_schema(
        vec![
            ("id", string_property("New node identifier")),
            ("type", node_type_property("sequentialThinking")),
            (
                "topic",
                string_property("Topic for sequential thinking nodes"),
            ),
            (
                "totalThoughts",
                integer_property("Thought count for thinking nodes"),
            ),
            (
                "analysisType",
                string_property("Analysis type for thinking nodes"),
            ),
            (
                "agent",
                string_property("ACP agent for sequential thinking nodes"),
            ),
            (
                "model",
                string_property("Model override for sequential thinking nodes"),
            ),
            (
                "thoughtLevel",
                string_property("Thought level override for sequential thinking nodes"),
            ),
            (
                "allowedTools",
                string_array_property("Allowed tools override for sequential thinking nodes"),
            ),
            (
                "maxTurns",
                integer_property("Maximum turns for sequential thinking nodes"),
            ),
            ("timeout", integer_property("Optional timeout in seconds")),
            ("agentConfig", agent_config_schema()),
        ],
        &["id", "type", "topic", "agent", "totalThoughts"],
    )
}

fn tool_node_schema() -> Value {
    object_schema(
        vec![
            ("id", string_property("New node identifier")),
            ("type", node_type_property("tool")),
            ("toolName", string_property("MCP tool name for tool nodes")),
            (
                "toolArgs",
                json!({
                    "type": "object",
                    "description": "Tool arguments for tool nodes"
                }),
            ),
            (
                "agent",
                string_property("Optional ACP agent for tool nodes"),
            ),
            ("model", string_property("Model override for tool nodes")),
            (
                "thoughtLevel",
                string_property("Thought level override for tool nodes"),
            ),
            (
                "allowedTools",
                string_array_property("Allowed tools override for tool nodes"),
            ),
            ("maxTurns", integer_property("Maximum turns for tool nodes")),
            ("agentConfig", agent_config_schema()),
        ],
        &["id", "type", "toolName"],
    )
}

fn node_type_property(node_type: &str) -> Value {
    json!({
        "type": "string",
        "description": format!("Node type '{node_type}'"),
        "enum": [node_type]
    })
}

fn agent_config_schema() -> Value {
    object_schema(
        vec![
            (
                "model",
                string_property("Model override for the agent session"),
            ),
            (
                "mode",
                string_property("Session mode override for the agent session"),
            ),
            (
                "thoughtLevel",
                string_property("Thought level override for the agent session"),
            ),
            (
                "configOptions",
                json!({
                    "type": "object",
                    "description": "Arbitrary config options forwarded to the agent session"
                }),
            ),
        ],
        &[],
    )
}
