use super::support::{
    bool_property, enum_string_property, object_schema, string_array_property, string_property,
    tool_definition,
};
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "AssumptionLedger" => assumption_ledger_tool(registry_name),
        _ => return None,
    };

    Some(tool)
}

fn assumption_ledger_tool(registry_name: &str) -> McpToolDefinition {
    tool_definition(
        registry_name,
        "Creates, updates, or reads assumptions linked to workflow or thinking context.",
        object_schema(
            vec![
                (
                    "action",
                    enum_string_property("Ledger action to perform", &["append", "update", "read"])
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
                    string_array_property("Concept tags for knowledge graph integration"),
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
    )
}
