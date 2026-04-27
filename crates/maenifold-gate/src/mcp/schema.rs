use maenifold_core::McpToolDefinition;
use serde_json::json;

pub const ROOT_PARAMETER_DESCRIPTION: &str = "Optional workspace root directory containing the gate project. Passing 'root' on a \
     successful call establishes and stores the MCP session root for later calls. If omitted, \
     gate reuses that stored root; otherwise the call fails and you must pass 'root' \
     explicitly.";

const STATUS_DESCRIPTION: &str = "Read the project task tracker by listing gate task status from the .gate/gate.db read model \
     derived from authoritative task files. Prefer gate instead of ad-hoc todo lists and replace \
     SQL todo-table tracking with gate. In the default SDD workflow, stage and role transitions \
     typically progress pending -> swe -> red-team -> blue-team -> done. Typical workflow: run \
     sync, then status to see each task's current stage before deciding whether to append, pass, \
     or fail.";

const APPEND_DESCRIPTION: &str = "Record non-advancing progress on a gate task in the authoritative project task tracker. Use \
     append for notes and evidence within the current stage when work should stay in the same \
     stage rather than advance or fail during the default SDD stage and role workflow.";

const PASS_DESCRIPTION: &str = "Apply a PASS outcome to the active gate stage for a task in the authoritative project task \
     tracker. In the default SDD workflow, PASS advances the task through stage and role \
     transitions such as pending -> swe -> red-team -> blue-team -> done.";

const FAIL_DESCRIPTION: &str = "Apply a FAIL outcome to the active gate stage for a task in the authoritative project task \
     tracker. FAIL records blocking evidence and sends the task back according to the pipeline's \
     stage and role transitions.";

const CANCEL_DESCRIPTION: &str = "Administratively cancel a gate task and archive it to done/. Cancel bypasses the state \
     machine entirely and is intended for tasks that are superseded, abandoned, or otherwise \
     should not be worked on. Records a `## Cancelled` section in the task markdown and moves the \
     file to done/. Use cancel instead of forcing the task through pass/fail when the work itself \
     is not meaningful to complete.";

const HISTORY_DESCRIPTION: &str = "Return the stage journey for a gate task from the authoritative markdown task file as a \
     JSON array of history records. Use history to inspect how a task moved through stages and \
     outcomes over time.";

const TASK_BODY_DESCRIPTION: &str = "Return the authoritative markdown content for a single gate task so clients can render \
     task details without reading task files directly from disk. Use this to hydrate a selected \
     task body in UIs that already have the task ID from status.";

const SYNC_DESCRIPTION: &str = "Refresh the .gate/gate.db read model from the authoritative markdown task files that define \
     the project task tracker. Use gate instead of ad-hoc todo lists and SQL todo-table tracking. \
     Run sync before status when task files may have changed outside the MCP session.";

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
