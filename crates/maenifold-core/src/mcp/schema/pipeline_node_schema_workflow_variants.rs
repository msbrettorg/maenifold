// T-01.9: Extracted from pipeline_node_schema.rs to keep that file under the
// 300 LOC ceiling. Hosts the workflow + prompt oneOf variants. Each function
// returns one fully inlined `serde_json::Value`. Schemas remain flat (no `$ref`)
// to satisfy GPT-5.4's schema validator. Shared `agent_config_schema()` and
// `node_adopt_schema()` helpers live in `pipeline_node_schema.rs` and are
// exported as `pub(super)` so each variant can compose them inline.

use super::pipeline_node_schema::{agent_config_schema, node_adopt_schema};
use serde_json::{Value, json};

pub(super) fn workflow_variant_schema() -> Value {
    json!(
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
                    "adopt": node_adopt_schema(),
                    "requiresRole": {
                        "type": "string",
                        "description": "Optional required role identifier for adopt validation"
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
            }
    )
}

pub(super) fn prompt_variant_schema() -> Value {
    json!(
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
                    "adopt": node_adopt_schema(),
                    "requiresRole": {
                        "type": "string",
                        "description": "Optional required role identifier for adopt validation"
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
            }
    )
}
