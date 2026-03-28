use super::support::{
    bool_property, enum_string_property, integer_property_with_default, object_schema,
    string_property, tool_definition,
};
use crate::mcp::McpToolDefinition;
use serde_json::{Value, json};

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Pipeline" => pipeline_tool(registry_name),
        _ => return None,
    };

    Some(tool)
}

#[allow(clippy::too_many_lines)]
fn pipeline_tool(registry_name: &str) -> McpToolDefinition {
    tool_definition(
        registry_name,
        "Creates and manages pipeline sessions composed of DAG nodes.",
        object_schema(
            vec![
                (
                    "sessionId",
                    string_property("Existing pipeline session identifier"),
                ),
                ("definition", definition_schema()),
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
                ("rewind", rewind_schema()),
                (
                    "delete",
                    bool_property(
                        "Delete the pipeline session and all associated workflow sessions",
                        false,
                    ),
                ),
                ("graft", graft_schema()),
                ("join", join_schema()),
                ("fork", fork_schema()),
                (
                    "nodeId",
                    string_property("Target node for status updates or restart"),
                ),
                (
                    "status",
                    enum_string_property("Node status update", &["completed", "abandoned"]).into(),
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
    )
}

fn definition_schema() -> Value {
    json!({
        "type": "object",
        "description": "Pipeline definition with nodes[] and edges[]"
    })
}

fn rewind_schema() -> Value {
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
    })
}

fn graft_schema() -> Value {
    json!({
        "type": "object",
        "description": "Add new nodes and edges onto an existing pipeline session",
        "properties": {
            "nodes": {
                "type": "array",
                "description": "New pipeline nodes to append",
                "items": pipeline_node_schema()
            },
            "edges": pipeline_edges_schema("New pipeline edges to add")
        },
        "required": ["nodes", "edges"],
        "additionalProperties": false
    })
}

fn pipeline_node_schema() -> Value {
    json!({
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
    })
}

fn join_schema() -> Value {
    json!({
        "type": "object",
        "description": "Merge another pipeline session into this session using additional linking edges",
        "properties": {
            "sourceSessionId": {
                "type": "string",
                "description": "Pipeline session identifier to merge into the target pipeline"
            },
            "edges": pipeline_edges_schema(
                "Edges that connect the source pipeline into the target pipeline"
            )
        },
        "required": ["sourceSessionId", "edges"],
        "additionalProperties": false
    })
}

fn pipeline_edges_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
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
    })
}

fn fork_schema() -> Value {
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
    })
}
