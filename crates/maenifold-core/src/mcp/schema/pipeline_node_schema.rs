use super::params::PipelineNodeSchema;
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use serde_json::json;

/// Custom `JsonSchema` for `PipelineNodeSchema` that produces a **flat `oneOf`** with every
/// variant schema fully inlined — no `$ref` pointers into a `definitions` block.
///
/// This is required for compatibility with GPT-5.4 and other OpenAI models whose schema
/// validator rejects externally-tagged enum wrappers like `{"workflow": {...}}` and rejects
/// schemas where `$ref` appears alongside sibling keywords inside a `oneOf` variant.
///
/// Each variant is a plain JSON Schema object with:
/// - `type: "object"`
/// - `additionalProperties: false`
/// - `required: [...]` listing all mandatory fields
/// - `properties: {...}` with all fields inlined (including `agentConfig`)
///
/// The `type` property inside every variant acts as a discriminator
/// (`"enum": ["workflow"]`, `"enum": ["prompt"]`, etc.) so agents can unambiguously
/// identify which variant they are targeting.
impl JsonSchema for PipelineNodeSchema {
    /// Returning `false` forces schemars to inline this schema wherever it appears
    /// (e.g. as array `items`) instead of emitting a `$ref` to `definitions`.
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        "PipelineNodeSchema".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        serde_json::from_value(node_schema_value())
            .expect("PipelineNodeSchema value is valid JSON Schema")
    }
}

/// Shared inline schema for the `agentConfig` property that appears in prompt,
/// sequentialThinking, and tool node variants.
fn agent_config_schema() -> serde_json::Value {
    json!({
        "description": "Agent configuration for pipeline node execution",
        "type": "object",
        "additionalProperties": false,
            "properties": {
                "model": {
                    "type": "string",
                    "description": "Model override for the agent session"
                },
            "mode": {
                "type": "string",
                "description": "Session mode override for the agent session"
            },
            "thoughtLevel": {
                "type": "string",
                "description": "Thought level override for the agent session"
            },
            "configOptions": {
                "type": "object",
                "description": "Arbitrary config options forwarded to the agent session",
                "additionalProperties": true
            }
        }
    })
}

fn node_schema_value() -> serde_json::Value {
    json!({
        "oneOf": [
            {
                "description": "A workflow pipeline node",
                "type": "object",
                "additionalProperties": false,
                "required": ["id", "type", "workflowId"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "New node identifier"
                    },
                    "displayLabel": {
                        "type": "string",
                        "description": "Optional presentation-only label for UI surfaces"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["workflow"],
                        "description": "Workflow node type"
                    },
                    "workflowId": {
                        "type": "string",
                        "description": "Workflow definition to run"
                    },
                    "acpClient": {
                        "type": "string",
                        "description": "Optional ACP client"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Optional prompt for workflow nodes"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Optional working directory for this node"
                    },
                    "timeout": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Optional timeout in seconds"
                    },
                    "retryMaxAttempts": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Optional maximum transient retry attempts; 0 means no retry"
                    },
                    "retryBackoffMinMs": {
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0,
                        "description": "Optional minimum retry backoff in milliseconds"
                    },
                    "retryBackoffMaxMs": {
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0,
                        "description": "Optional maximum retry backoff in milliseconds"
                    },
                    "retryJitter": {
                        "type": "boolean",
                        "description": "Optional jitter toggle for retry backoff"
                    }
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["id", "type", "prompt", "agent"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "New node identifier"
                    },
                    "displayLabel": {
                        "type": "string",
                        "description": "Optional presentation-only label for UI surfaces"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["prompt"],
                        "description": "Node type 'prompt'"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Prompt text for prompt nodes"
                    },
                    "agent": {
                        "type": "string",
                        "description": "ACP agent for prompt nodes"
                    },
                    "toolMode": {
                        "type": "string",
                        "description": "Tool mode for prompt nodes"
                    },
                    "model": {
                        "type": "string",
                        "description": "Model override for prompt nodes"
                    },
                    "thoughtLevel": {
                        "type": "string",
                        "description": "Thought level override for prompt nodes"
                    },
                    "allowedTools": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Allowed tools override for prompt nodes"
                    },
                    "maxTurns": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Maximum turns for prompt nodes"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Optional working directory for this node"
                    },
                    "timeout": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Optional timeout in seconds"
                    },
                    "retryMaxAttempts": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Optional maximum transient retry attempts; 0 means no retry"
                    },
                    "retryBackoffMinMs": {
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0,
                        "description": "Optional minimum retry backoff in milliseconds"
                    },
                    "retryBackoffMaxMs": {
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0,
                        "description": "Optional maximum retry backoff in milliseconds"
                    },
                    "retryJitter": {
                        "type": "boolean",
                        "description": "Optional jitter toggle for retry backoff"
                    },
                    "agentConfig": agent_config_schema()
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["id", "type", "topic", "totalThoughts", "agent"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "New node identifier"
                    },
                    "displayLabel": {
                        "type": "string",
                        "description": "Optional presentation-only label for UI surfaces"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["sequentialThinking"],
                        "description": "Node type 'sequentialThinking'"
                    },
                    "topic": {
                        "type": "string",
                        "description": "Topic for sequential thinking nodes"
                    },
                    "totalThoughts": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Thought count for thinking nodes"
                    },
                    "analysisType": {
                        "type": "string",
                        "description": "Analysis type for thinking nodes"
                    },
                    "agent": {
                        "type": "string",
                        "description": "ACP agent for sequential thinking nodes"
                    },
                    "model": {
                        "type": "string",
                        "description": "Model override for sequential thinking nodes"
                    },
                    "thoughtLevel": {
                        "type": "string",
                        "description": "Thought level override for sequential thinking nodes"
                    },
                    "allowedTools": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Allowed tools override for sequential thinking nodes"
                    },
                    "maxTurns": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Maximum turns for sequential thinking nodes"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Optional working directory for this node"
                    },
                    "timeout": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Optional timeout in seconds"
                    },
                    "retryMaxAttempts": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Optional maximum transient retry attempts; 0 means no retry"
                    },
                    "retryBackoffMinMs": {
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0,
                        "description": "Optional minimum retry backoff in milliseconds"
                    },
                    "retryBackoffMaxMs": {
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0,
                        "description": "Optional maximum retry backoff in milliseconds"
                    },
                    "retryJitter": {
                        "type": "boolean",
                        "description": "Optional jitter toggle for retry backoff"
                    },
                    "agentConfig": agent_config_schema()
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["id", "type", "toolName"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "New node identifier"
                    },
                    "displayLabel": {
                        "type": "string",
                        "description": "Optional presentation-only label for UI surfaces"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["tool"],
                        "description": "Node type 'tool'"
                    },
                    "toolName": {
                        "type": "string",
                        "description": "MCP tool name for tool nodes"
                    },
                    "toolArgs": {
                        "type": "object",
                        "description": "Tool arguments for tool nodes",
                        "additionalProperties": true
                    },
                    "agent": {
                        "type": "string",
                        "description": "Optional ACP agent for tool nodes"
                    },
                    "model": {
                        "type": "string",
                        "description": "Model override for tool nodes"
                    },
                    "thoughtLevel": {
                        "type": "string",
                        "description": "Thought level override for tool nodes"
                    },
                    "allowedTools": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Allowed tools override for tool nodes"
                    },
                    "maxTurns": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Maximum turns for tool nodes"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Optional working directory for this node"
                    },
                    "timeout": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Optional timeout in seconds"
                    },
                    "retryMaxAttempts": {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0,
                        "description": "Optional maximum transient retry attempts; 0 means no retry"
                    },
                    "retryBackoffMinMs": {
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0,
                        "description": "Optional minimum retry backoff in milliseconds"
                    },
                    "retryBackoffMaxMs": {
                        "type": "integer",
                        "format": "uint64",
                        "minimum": 0,
                        "description": "Optional maximum retry backoff in milliseconds"
                    },
                    "retryJitter": {
                        "type": "boolean",
                        "description": "Optional jitter toggle for retry backoff"
                    },
                    "agentConfig": agent_config_schema()
                }
            }
        ]
    })
}
