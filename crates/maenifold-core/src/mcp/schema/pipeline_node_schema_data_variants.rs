// T-01.9: Extracted from pipeline_node_schema.rs to keep that file under the
// 300 LOC ceiling. Hosts the sequentialThinking + tool oneOf variants. Each
// function returns one fully inlined `serde_json::Value`. Schemas remain flat
// (no `$ref`) to satisfy GPT-5.4's schema validator. Shared `agent_config_schema()`
// and `node_adopt_schema()` helpers live in `pipeline_node_schema.rs` and are
// exported as `pub(super)` so each variant can compose them inline.

use super::pipeline_node_schema::{agent_config_schema, join_policy_schema, node_adopt_schema};
use serde_json::{Value, json};

pub(super) fn sequential_thinking_variant_schema() -> Value {
    json!(
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
                    "agent": {
                        "type": "string",
                        "description": "ACP agent for sequential thinking nodes"
                    },
                    "adopt": node_adopt_schema(),
                    "joinPolicy": join_policy_schema(),
                    "requiresRole": {
                        "type": "string",
                        "description": "Optional required role identifier for adopt validation"
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
            }
    )
}

pub(super) fn tool_variant_schema() -> Value {
    json!(
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
                    "joinPolicy": join_policy_schema(),
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
    )
}
