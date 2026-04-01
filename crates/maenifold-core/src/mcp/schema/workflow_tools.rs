use super::support::{
    bool_property, enum_string_property, integer_property_with_default, object_schema,
    string_or_string_array_property, string_property, string_property_with_pattern,
    tool_definition,
};
use crate::mcp::McpToolDefinition;
use serde_json::json;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Workflow" => workflow_tool(registry_name),
        "RecentActivity" => recent_activity_tool(registry_name),
        _ => return None,
    };

    Some(tool)
}

fn workflow_tool(registry_name: &str) -> McpToolDefinition {
    tool_definition(
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
                    string_property("Structured ConfessionReport used when completing the session"),
                ),
                (
                    "view",
                    bool_property("Display the workflow queue status", false),
                ),
                (
                    "append",
                    string_property("Workflow identifier or identifiers to append to the queue"),
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
    )
}

fn recent_activity_tool(registry_name: &str) -> McpToolDefinition {
    tool_definition(
        registry_name,
        "Reports recently indexed files with optional content excerpts.",
        object_schema(
            vec![
                (
                    "limit",
                    integer_property_with_default("Maximum number of results to return", json!(10)),
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
    )
}
