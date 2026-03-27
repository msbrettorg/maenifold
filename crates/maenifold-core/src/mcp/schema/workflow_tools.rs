use super::support::{
    bool_property, enum_string_property, integer_property, integer_property_with_default,
    object_schema, string_or_string_array_property, string_property, string_property_with_pattern,
    tool_definition,
};
use crate::mcp::McpToolDefinition;
use serde_json::json;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "SequentialThinking" => tool_definition(
            registry_name,
            "Creates structured thinking sessions with persistent branching and conclusions.",
            object_schema(
                vec![
                    (
                        "response",
                        string_property(
                            "Main thought content; must include [[WikiLinks]] unless cancelling",
                        ),
                    ),
                    (
                        "nextThoughtNeeded",
                        bool_property("Whether another thought is needed after this one", false),
                    ),
                    (
                        "thoughtNumber",
                        integer_property_with_default("Current thought number", json!(0)),
                    ),
                    (
                        "totalThoughts",
                        integer_property_with_default(
                            "Current estimate for total thoughts",
                            json!(0),
                        ),
                    ),
                    (
                        "sessionId",
                        string_property("Existing sequential thinking session identifier"),
                    ),
                    (
                        "cancel",
                        bool_property("Cancel the session instead of adding a thought", false),
                    ),
                    (
                        "thoughts",
                        string_property(
                            "Optional meta-thoughts; include [[WikiLinks]] when provided",
                        ),
                    ),
                    (
                        "isRevision",
                        bool_property("Marks this thought as a revision of prior thinking", false),
                    ),
                    (
                        "revisesThought",
                        integer_property("Thought number that this revision updates"),
                    ),
                    (
                        "branchFromThought",
                        integer_property("Thought number where this branch diverges"),
                    ),
                    (
                        "branchId",
                        string_property("Branch identifier for multi-agent coordination"),
                    ),
                    (
                        "needsMoreThoughts",
                        bool_property(
                            "Whether more thoughts are needed than originally estimated",
                            false,
                        ),
                    ),
                    (
                        "analysisType",
                        string_property(
                            "Optional analysis type annotation such as bug or architecture",
                        ),
                    ),
                    (
                        "parentWorkflowId",
                        string_property("Optional parent workflow session identifier"),
                    ),
                    (
                        "conclusion",
                        string_property(
                            "Structured ConfessionReport used when completing the session",
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "Workflow" => tool_definition(
            registry_name,
            "Runs structured workflows with optional sequential-thinking submachines.",
            object_schema(
                vec![
                    (
                        "sessionId",
                        string_property("Existing workflow session identifier to continue"),
                    ),
                    (
                        "workflowId",
                        string_or_string_array_property(
                            "Workflow identifier or list of workflow identifiers to start",
                        ),
                    ),
                    (
                        "response",
                        string_property("Response to the current step; include [[WikiLinks]]"),
                    ),
                    (
                        "thoughts",
                        string_property("Ambient or meta-thoughts; include [[WikiLinks]]"),
                    ),
                    (
                        "status",
                        enum_string_property(
                            "Terminal status for the workflow session",
                            &["completed", "cancelled"],
                        )
                        .into(),
                    ),
                    (
                        "conclusion",
                        string_property(
                            "Structured ConfessionReport used when completing the session",
                        ),
                    ),
                    (
                        "view",
                        bool_property("Display the workflow queue status", false),
                    ),
                    (
                        "append",
                        string_property(
                            "Workflow identifier or identifiers to append to the queue",
                        ),
                    ),
                    (
                        "submachineSessionId",
                        string_property(
                            "SequentialThinking session identifier registered as a submachine",
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "Pipeline" => tool_definition(
            registry_name,
            "Creates and manages pipeline sessions composed of DAG nodes.",
            object_schema(
                vec![
                    (
                        "sessionId",
                        string_property("Existing pipeline session identifier"),
                    ),
                    (
                        "definition",
                        json!({
                            "type": "object",
                            "description": "Pipeline definition with nodes[] and edges[]"
                        }),
                    ),
                    (
                        "maxConcurrency",
                        integer_property_with_default("Max concurrent active nodes", json!(8)),
                    ),
                    (
                        "run",
                        bool_property("Start autonomous execution for an existing pipeline", false),
                    ),
                    (
                        "prompt",
                        string_property("Ambient prompt for autonomous pipeline execution"),
                    ),
                    (
                        "pause",
                        bool_property("Pause a running pipeline after active nodes drain", false),
                    ),
                    ("resume", bool_property("Resume a paused pipeline", false)),
                    (
                        "cancel",
                        bool_property("Cancel a running pipeline and stop future execution", false),
                    ),
                    (
                        "view",
                        bool_property("Return current pipeline state", false),
                    ),
                    (
                        "dispatch",
                        bool_property("Claim ready nodes as active", false),
                    ),
                    (
                        "list",
                        bool_property("List all pipeline sessions with status summaries", false),
                    ),
                    (
                        "rewind",
                        json!({
                            "type": "object",
                            "description": "Rewind the specified nodeIds and all downstream nodes",
                            "properties": {
                                "nodeIds": {
                                    "type": "array",
                                    "description": "Node identifiers to rewind before cascading downstream resets",
                                    "items": { "type": "string" }
                                }
                            },
                            "required": ["nodeIds"],
                            "additionalProperties": false
                        }),
                    ),
                    (
                        "delete",
                        bool_property(
                            "Delete the pipeline session and all associated workflow sessions",
                            false,
                        ),
                    ),
                    (
                        "graft",
                        json!({
                            "type": "object",
                            "description": "Add new nodes and edges onto an existing pipeline session",
                            "properties": {
                                "nodes": {
                                    "type": "array",
                                    "description": "New pipeline nodes to append",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "id": {
                                                "type": "string",
                                                "description": "New node identifier"
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
                                                "description": "Prompt text for prompt nodes"
                                            },
                                            "agent": {
                                                "type": "string",
                                                "description": "ACP agent for prompt/thinking nodes"
                                            },
                                            "toolMode": {
                                                "type": "string",
                                                "description": "Tool mode for prompt nodes"
                                            },
                                            "topic": {
                                                "type": "string",
                                                "description": "Topic for sequential thinking nodes"
                                            },
                                            "totalThoughts": {
                                                "type": "integer",
                                                "description": "Thought count for thinking nodes"
                                            },
                                            "analysisType": {
                                                "type": "string",
                                                "description": "Analysis type for thinking nodes"
                                            },
                                            "toolName": {
                                                "type": "string",
                                                "description": "MCP tool name for tool nodes"
                                            },
                                            "toolArgs": {
                                                "type": "object",
                                                "description": "Tool arguments for tool nodes"
                                            }
                                        },
                                        "required": ["id"]
                                    }
                                },
                                "edges": {
                                    "type": "array",
                                    "description": "New pipeline edges to add",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "source": {
                                                "type": "string",
                                                "description": "Upstream node identifier"
                                            },
                                            "target": {
                                                "type": "string",
                                                "description": "Downstream node identifier"
                                            },
                                            "type": {
                                                "type": "string",
                                                "description": "Edge type controlling downstream activation",
                                                "enum": ["success", "always"]
                                            }
                                        },
                                        "required": ["source", "target"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["nodes", "edges"],
                            "additionalProperties": false
                        }),
                    ),
                    (
                        "join",
                        json!({
                            "type": "object",
                            "description": "Merge another pipeline session into this session using additional linking edges",
                            "properties": {
                                "sourceSessionId": {
                                    "type": "string",
                                    "description": "Pipeline session identifier to merge into the target pipeline"
                                },
                                "edges": {
                                    "type": "array",
                                    "description": "Edges that connect the source pipeline into the target pipeline",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "source": {
                                                "type": "string",
                                                "description": "Upstream node identifier"
                                            },
                                            "target": {
                                                "type": "string",
                                                "description": "Downstream node identifier"
                                            },
                                            "type": {
                                                "type": "string",
                                                "description": "Edge type controlling downstream activation",
                                                "enum": ["success", "always"]
                                            }
                                        },
                                        "required": ["source", "target"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["sourceSessionId", "edges"],
                            "additionalProperties": false
                        }),
                    ),
                    (
                        "fork",
                        json!({
                            "type": "object",
                            "description": "Clone the selected node and downstream subgraph into a new pipeline session",
                            "properties": {
                                "fromNodeId": {
                                    "type": "string",
                                    "description": "Node identifier where the forked subgraph starts"
                                },
                                "newSessionId": {
                                    "type": "string",
                                    "description": "Optional pipeline session identifier for the forked pipeline"
                                }
                            },
                            "required": ["fromNodeId"],
                            "additionalProperties": false
                        }),
                    ),
                    (
                        "nodeId",
                        string_property("Target node for status updates or restart"),
                    ),
                    (
                        "status",
                        enum_string_property("Node status update", &["completed", "abandoned"])
                            .into(),
                    ),
                    (
                        "conclusion",
                        string_property("Conclusion text when completing a node"),
                    ),
                    (
                        "restart",
                        bool_property("Restart an abandoned/stuck node", false),
                    ),
                    ("learn", bool_property("Return help documentation", false)),
                ],
                &[],
            ),
        ),
        "RecentActivity" => tool_definition(
            registry_name,
            "Reports recently indexed files with optional content excerpts.",
            object_schema(
                vec![
                    (
                        "limit",
                        integer_property_with_default(
                            "Maximum number of results to return",
                            json!(10),
                        ),
                    ),
                    (
                        "filter",
                        enum_string_property(
                            "Activity filter",
                            &["thinking", "memory", "assumptions", "all"],
                        )
                        .with_default(json!("all")),
                    ),
                    (
                        "timespan",
                        string_property_with_pattern(
                            "Positive timespan such as 24.00:00:00",
                            r"^(\d+\.)?\d{2}:\d{2}:\d{2}(\.\d{1,7})?$",
                        ),
                    ),
                    (
                        "includeContent",
                        bool_property(
                            "Include full H2 section content instead of headers only",
                            false,
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "AssumptionLedger" => tool_definition(
            registry_name,
            "Creates, updates, or reads assumptions linked to workflow or thinking context.",
            object_schema(
                vec![
                    (
                        "action",
                        enum_string_property(
                            "Ledger action to perform",
                            &["append", "update", "read"],
                        )
                        .into(),
                    ),
                    (
                        "assumption",
                        string_property("Assumption statement for append operations"),
                    ),
                    (
                        "context",
                        string_property("Workflow or thinking session reference"),
                    ),
                    (
                        "validationPlan",
                        string_property("Planned validation method"),
                    ),
                    (
                        "confidence",
                        string_property("Confidence level such as high, medium, or low"),
                    ),
                    (
                        "concepts",
                        super::support::string_array_property(
                            "Concept tags for knowledge graph integration",
                        ),
                    ),
                    (
                        "uri",
                        string_property("Memory URI for update or read actions"),
                    ),
                    (
                        "status",
                        enum_string_property(
                            "Updated assumption status",
                            &["active", "validated", "invalidated", "refined"],
                        )
                        .into(),
                    ),
                    (
                        "notes",
                        string_property("Additional notes for update operations"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["action"],
            ),
        ),
        _ => return None,
    };

    Some(tool)
}

#[cfg(test)]
mod tests {
    use super::tool;
    use serde_json::Value;

    #[test]
    fn pipeline_schema_includes_all_new_operation_fields_with_expected_types() {
        let tool = tool("Pipeline").expect("pipeline tool schema");
        let properties = tool
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .expect("pipeline properties");

        for field in ["run", "pause", "resume", "cancel", "delete", "list"] {
            let property = properties
                .get(field)
                .and_then(Value::as_object)
                .expect("boolean property");
            assert_eq!(
                property.get("type").and_then(Value::as_str),
                Some("boolean")
            );
            assert!(
                property
                    .get("description")
                    .and_then(Value::as_str)
                    .is_some_and(|description| !description.is_empty())
            );
        }

        let rewind = properties
            .get("rewind")
            .and_then(Value::as_object)
            .expect("rewind property");
        assert_eq!(rewind.get("type").and_then(Value::as_str), Some("object"));
        assert_eq!(
            rewind
                .get("required")
                .and_then(Value::as_array)
                .and_then(|required| required.first())
                .and_then(Value::as_str),
            Some("nodeIds")
        );
        assert_eq!(
            rewind
                .get("properties")
                .and_then(Value::as_object)
                .and_then(|properties| properties.get("nodeIds"))
                .and_then(Value::as_object)
                .and_then(|node_ids| node_ids.get("type"))
                .and_then(Value::as_str),
            Some("array")
        );

        let graft = properties
            .get("graft")
            .and_then(Value::as_object)
            .expect("graft property");
        assert_eq!(graft.get("type").and_then(Value::as_str), Some("object"));
        let graft_properties = graft
            .get("properties")
            .and_then(Value::as_object)
            .expect("graft properties");
        assert_eq!(
            graft_properties
                .get("nodes")
                .and_then(Value::as_object)
                .and_then(|nodes| nodes.get("type"))
                .and_then(Value::as_str),
            Some("array")
        );
        assert_eq!(
            graft_properties
                .get("edges")
                .and_then(Value::as_object)
                .and_then(|edges| edges.get("type"))
                .and_then(Value::as_str),
            Some("array")
        );

        let join = properties
            .get("join")
            .and_then(Value::as_object)
            .expect("join property");
        assert_eq!(join.get("type").and_then(Value::as_str), Some("object"));
        assert_eq!(
            join.get("required")
                .and_then(Value::as_array)
                .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
            Some(vec!["sourceSessionId", "edges"])
        );

        let fork = properties
            .get("fork")
            .and_then(Value::as_object)
            .expect("fork property");
        assert_eq!(fork.get("type").and_then(Value::as_str), Some("object"));
        assert_eq!(
            fork.get("required")
                .and_then(Value::as_array)
                .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
            Some(vec!["fromNodeId"])
        );
        assert_eq!(
            fork.get("properties")
                .and_then(Value::as_object)
                .and_then(|properties| properties.get("newSessionId"))
                .and_then(Value::as_object)
                .and_then(|property| property.get("type"))
                .and_then(Value::as_str),
            Some("string")
        );
    }
}
