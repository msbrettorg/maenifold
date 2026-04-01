use super::params::{RecentActivityParams, WorkflowParams};
use super::support::{derived_tool_definition, schema_value, tool_definition};
use crate::mcp::McpToolDefinition;
use serde_json::Value;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Workflow" => derived_tool_definition::<WorkflowParams>(
            registry_name,
            "Runs structured workflows with optional sequential-thinking submachines.",
        ),
        "RecentActivity" => recent_activity_tool(registry_name),
        _ => return None,
    };

    Some(tool)
}

fn recent_activity_tool(registry_name: &str) -> McpToolDefinition {
    let mut schema = schema_value::<RecentActivityParams>();
    if let Some(timespan) = schema
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .and_then(|properties| properties.get_mut("timespan"))
        .and_then(Value::as_object_mut)
    {
        timespan.insert(
            "pattern".to_string(),
            Value::String(r"^(\d+\.)?\d{2}:\d{2}:\d{2}(\.\d{1,7})?$".to_string()),
        );
    }

    tool_definition(
        registry_name,
        "Reports recently indexed files with optional content excerpts.",
        schema,
    )
}
