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
mod tests {
    use super::list;
    use serde_json::Value;

    #[test]
    fn mcp_tool_list_exposes_exactly_eight_gate_tools() {
        let tools = list();
        let names: Vec<_> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "status",
                "append",
                "pass",
                "fail",
                "cancel",
                "history",
                "task_body",
                "sync"
            ]
        );
    }

    #[test]
    fn every_mcp_tool_schema_property_has_a_description() {
        for tool in list() {
            let properties = tool
                .input_schema
                .get("properties")
                .and_then(Value::as_object)
                .expect("tool schema should define properties");

            for (property_name, schema) in properties {
                let description = schema
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                assert!(
                    !description.trim().is_empty(),
                    "tool '{}' property '{}' is missing a description",
                    tool.name,
                    property_name
                );
            }
        }
    }

    #[test]
    fn every_root_parameter_description_matches_runtime_behavior() {
        for tool in list() {
            let Some(root_schema) = tool
                .input_schema
                .get("properties")
                .and_then(Value::as_object)
                .and_then(|properties| properties.get("root"))
            else {
                continue;
            };

            let description = root_schema
                .get("description")
                .and_then(Value::as_str)
                .expect("root description should be present");

            assert!(
                description.contains("establishes and stores the MCP session root"),
                "tool '{}' root description should explain storing the session root",
                tool.name
            );
            assert!(
                description.contains("reuses that stored root"),
                "tool '{}' root description should explain stored-root reuse",
                tool.name
            );
            assert!(
                description.contains("call fails"),
                "tool '{}' root description should explain failure without a root",
                tool.name
            );
            assert!(
                !description.contains("Defaults to the current working directory"),
                "tool '{}' root description must not claim '.' defaulting",
                tool.name
            );
        }
    }

    #[test]
    fn tool_descriptions_explain_gate_workflow() {
        let tools = list();
        let status = tools
            .iter()
            .find(|tool| tool.name == "status")
            .expect("status tool should exist");
        assert!(status.description.contains("project task tracker"));
        assert!(
            status
                .description
                .contains("Prefer gate instead of ad-hoc todo lists")
        );
        assert!(
            status
                .description
                .contains("replace SQL todo-table tracking with gate")
        );
        assert!(status.description.contains("default SDD workflow"));
        assert!(
            status
                .description
                .contains("pending -> swe -> red-team -> blue-team -> done")
        );
        assert!(status.description.contains("run sync, then status"));
        assert!(status.description.contains("append, pass, or fail"));

        let append = tools
            .iter()
            .find(|tool| tool.name == "append")
            .expect("append tool should exist");
        assert!(append.description.contains("non-advancing progress"));
        assert!(append.description.contains("current stage"));

        let pass = tools
            .iter()
            .find(|tool| tool.name == "pass")
            .expect("pass tool should exist");
        assert!(pass.description.contains("stage and role transitions"));
        assert!(
            pass.description
                .contains("pending -> swe -> red-team -> blue-team -> done")
        );

        let fail = tools
            .iter()
            .find(|tool| tool.name == "fail")
            .expect("fail tool should exist");
        assert!(fail.description.contains("sends the task back"));

        let history = tools
            .iter()
            .find(|tool| tool.name == "history")
            .expect("history tool should exist");
        assert!(history.description.contains("stage journey"));
        assert!(
            history
                .description
                .contains("JSON array of history records")
        );

        let task_body = tools
            .iter()
            .find(|tool| tool.name == "task_body")
            .expect("task_body tool should exist");
        assert!(
            task_body
                .description
                .contains("authoritative markdown content")
        );
        assert!(
            task_body
                .description
                .contains("without reading task files directly from disk")
        );

        let sync = tools
            .iter()
            .find(|tool| tool.name == "sync")
            .expect("sync tool should exist");
        assert!(
            sync.description
                .contains("authoritative markdown task files")
        );
        assert!(sync.description.contains("project task tracker"));
        assert!(
            sync.description
                .contains("Use gate instead of ad-hoc todo lists")
        );
        assert!(sync.description.contains("SQL todo-table tracking"));
    }
}
