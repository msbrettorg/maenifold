use maenifold_core::McpToolDefinition;
use serde_json::json;

#[path = "schema_descriptions.rs"]
mod schema_descriptions;

use schema_descriptions::{
    APPEND_DESCRIPTION, CANCEL_DESCRIPTION, CREATE_DESCRIPTION, FAIL_DESCRIPTION,
    HISTORY_DESCRIPTION, PASS_DESCRIPTION, ROOT_PARAMETER_DESCRIPTION, STATUS_DESCRIPTION,
    SYNC_DESCRIPTION, TASK_BODY_DESCRIPTION,
};

pub fn list() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "status".to_string(),
            description: STATUS_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    },
                    "pipeline": {
                        "type": "string",
                        "description": "Optional pipeline name to filter status results. When omitted, all pipelines are included."
                    },
                    "state": {
                        "type": "string",
                        "description": "Optional task state to filter status results, for example 'pending' or 'red-team'."
                    },
                    "ready": {
                        "type": "boolean",
                        "description": "Optional flag to show only ready-to-work tasks, currently defined as tasks in todo/ whose effective state is 'pending'. Defaults to false."
                    },
                    "blocked": {
                        "type": "boolean",
                        "description": "Optional flag to show only blocked tasks, including tasks blocked by unfinished, missing, or cyclic dependencies. Defaults to false."
                    },
                    "includeDone": {
                        "type": "boolean",
                        "description": "Optional flag to include tasks whose effective state is 'done'. Defaults to false so status focuses on actionable work."
                    }
                },
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "append".to_string(),
            description: APPEND_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to append work notes to, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    },
                    "pipeline": {
                        "type": "string",
                        "description": "Optional pipeline name used to resolve the task within a specific pipeline. When omitted, normal task lookup rules apply."
                    },
                    "agent": {
                        "type": "string",
                        "description": "Required agent or actor name that is recording the append entry."
                    },
                    "notes": {
                        "type": "string",
                        "description": "Required human-readable progress notes to append to the task without changing its stage outcome."
                    },
                    "model": {
                        "type": "string",
                        "description": "Optional model name associated with the work record. When omitted, no model metadata is stored."
                    }
                },
                "required": ["taskId", "agent", "notes"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "create".to_string(),
            description: CREATE_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier for the new task, for example 'T-90.20'. Must match ^[A-Za-z0-9._-]+$ and must not contain '..', '/', or '\\'."
                    },
                    "title": {
                        "type": "string",
                        "description": "Required human-readable task title used in task frontmatter, heading, and generated filename slug."
                    },
                    "pipeline": {
                        "type": "string",
                        "description": "Required gate pipeline name to bind the task to, for example 'sdd', 'tdd', 'gsd', or 'lightweight'."
                    },
                    "priority": {
                        "type": "string",
                        "description": "Optional task priority written to frontmatter. Defaults to P2."
                    },
                    "body": {
                        "type": "string",
                        "description": "Optional task body used for the Goal section and, when present, initial Acceptance Criteria."
                    },
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    }
                },
                "required": ["taskId", "title", "pipeline"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "pass".to_string(),
            description: PASS_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to mark as passed for the active stage, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    },
                    "pipeline": {
                        "type": "string",
                        "description": "Optional pipeline name used to resolve the task within a specific pipeline. When omitted, normal task lookup rules apply."
                    },
                    "agent": {
                        "type": "string",
                        "description": "Required agent or actor name approving the PASS outcome."
                    },
                    "notes": {
                        "type": "string",
                        "description": "Required human-readable rationale or evidence for the PASS outcome."
                    },
                    "model": {
                        "type": "string",
                        "description": "Optional model name associated with the PASS record. When omitted, no model metadata is stored."
                    }
                },
                "required": ["taskId", "agent", "notes"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "fail".to_string(),
            description: FAIL_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to mark as failed for the active stage, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    },
                    "pipeline": {
                        "type": "string",
                        "description": "Optional pipeline name used to resolve the task within a specific pipeline. When omitted, normal task lookup rules apply."
                    },
                    "agent": {
                        "type": "string",
                        "description": "Required agent or actor name recording the FAIL outcome."
                    },
                    "notes": {
                        "type": "string",
                        "description": "Required human-readable rationale or evidence for the FAIL outcome."
                    },
                    "model": {
                        "type": "string",
                        "description": "Optional model name associated with the FAIL record. When omitted, no model metadata is stored."
                    }
                },
                "required": ["taskId", "agent", "notes"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "cancel".to_string(),
            description: CANCEL_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to cancel and archive, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    },
                    "pipeline": {
                        "type": "string",
                        "description": "Optional pipeline name used to resolve the task within a specific pipeline. When omitted, normal task lookup rules apply."
                    },
                    "agent": {
                        "type": "string",
                        "description": "Required agent or actor name recording the cancellation."
                    },
                    "notes": {
                        "type": "string",
                        "description": "Required human-readable rationale explaining why the task is being cancelled."
                    },
                    "model": {
                        "type": "string",
                        "description": "Optional model name associated with the cancellation record. When omitted, no model metadata is stored."
                    }
                },
                "required": ["taskId", "agent", "notes"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "history".to_string(),
            description: HISTORY_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to inspect stage history for, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    }
                },
                "required": ["taskId"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "task_body".to_string(),
            description: TASK_BODY_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to read markdown for, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    }
                },
                "required": ["taskId"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "sync".to_string(),
            description: SYNC_DESCRIPTION.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": {
                        "type": "string",
                        "description": ROOT_PARAMETER_DESCRIPTION
                    },
                    "pipeline": {
                        "type": "string",
                        "description": "Optional pipeline name to limit synchronization to a specific pipeline. When omitted, all pipelines are synced."
                    },
                    "rebuild": {
                        "type": "boolean",
                        "description": "Optional flag to force a rebuild of the read model instead of an incremental sync. Defaults to false."
                    }
                },
                "additionalProperties": false
            }),
        },
    ]
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
