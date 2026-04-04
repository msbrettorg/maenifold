use serde_json::{Value, json};

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
            description: "List gate task status from the .gate/gate.db read model derived from authoritative task files.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": {
                        "type": "string",
                        "description": "Optional workspace root directory containing the gate project. Defaults to the current working directory ('.')."
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
            description: "Append a non-advancing stage work record to a gate task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to append work notes to, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": "Optional workspace root directory containing the gate project. Defaults to the current working directory ('.')."
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
            description: "Apply a PASS outcome to the active gate stage for a task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to mark as passed for the active stage, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": "Optional workspace root directory containing the gate project. Defaults to the current working directory ('.')."
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
            description: "Apply a FAIL outcome to the active gate stage for a task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": {
                        "type": "string",
                        "description": "Required gate task identifier to mark as failed for the active stage, for example 'T-90.20'."
                    },
                    "root": {
                        "type": "string",
                        "description": "Optional workspace root directory containing the gate project. Defaults to the current working directory ('.')."
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
            description: "Refresh the .gate/gate.db read model in place from the authoritative markdown files.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": {
                        "type": "string",
                        "description": "Optional workspace root directory containing the gate project. Defaults to the current working directory ('.')."
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
}
