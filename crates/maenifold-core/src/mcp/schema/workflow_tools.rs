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
            "Creates and manages pipeline sessions composed of workflow nodes.",
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
                        "view",
                        bool_property("Return current pipeline state", false),
                    ),
                    (
                        "dispatch",
                        bool_property("Claim ready nodes as active", false),
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
