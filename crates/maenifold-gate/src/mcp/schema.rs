use serde_json::{Value, json};

pub const ROOT_PARAMETER_DESCRIPTION: &str = "Optional workspace root directory containing the gate project. If omitted, gate reuses the \
     root already selected for this MCP session; otherwise the call fails and you must pass \
     'root' explicitly.";

const STATUS_DESCRIPTION: &str = "Read the project task tracker by listing gate task status from the .gate/gate.db read model \
     derived from authoritative task files. Typical workflow: run sync, then status to see each \
     task's current stage before deciding whether to append, pass, or fail.";

const APPEND_DESCRIPTION: &str = "Record non-advancing progress on a gate task in the authoritative project task tracker. Use \
     append for notes and evidence within the current stage when work should stay in the same \
     stage rather than advance or fail.";

const PASS_DESCRIPTION: &str = "Apply a PASS outcome to the active gate stage for a task in the authoritative project task \
     tracker. PASS moves the task forward to the next stage in the pipeline workflow.";

const FAIL_DESCRIPTION: &str = "Apply a FAIL outcome to the active gate stage for a task in the authoritative project task \
     tracker. FAIL records blocking evidence and sends the task back according to the pipeline's \
     stage transitions.";

const SYNC_DESCRIPTION: &str = "Refresh the .gate/gate.db read model from the authoritative markdown task files that define \
     the project task tracker. Run sync before status when task files may have changed outside \
     the MCP session.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
}

pub fn list() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "status",
            description: STATUS_DESCRIPTION,
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
                    }
                },
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "append",
            description: APPEND_DESCRIPTION,
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
            name: "pass",
            description: PASS_DESCRIPTION,
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
            name: "fail",
            description: FAIL_DESCRIPTION,
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
            name: "sync",
            description: SYNC_DESCRIPTION,
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
    fn mcp_tool_list_exposes_exactly_five_gate_tools() {
        let tools = list();
        let names: Vec<_> = tools.iter().map(|tool| tool.name).collect();
        assert_eq!(names, vec!["status", "append", "pass", "fail", "sync"]);
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
                description.contains("reuses the root already selected for this MCP session"),
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
        assert!(pass.description.contains("moves the task forward"));

        let fail = tools
            .iter()
            .find(|tool| tool.name == "fail")
            .expect("fail tool should exist");
        assert!(fail.description.contains("sends the task back"));

        let sync = tools
            .iter()
            .find(|tool| tool.name == "sync")
            .expect("sync tool should exist");
        assert!(
            sync.description
                .contains("authoritative markdown task files")
        );
        assert!(sync.description.contains("project task tracker"));
    }
}
