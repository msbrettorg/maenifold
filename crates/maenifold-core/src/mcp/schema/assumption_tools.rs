use super::params::AssumptionLedgerParams;
use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "AssumptionLedger" => derived_tool_definition::<AssumptionLedgerParams>(
            registry_name,
            "Creates, updates, or reads assumptions linked to workflow or thinking context.",
        ),
        _ => return None,
    };

    Some(tool)
}
