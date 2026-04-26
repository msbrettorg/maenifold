use super::support::derived_tool_definition;
use crate::mcp::{McpToolDefinition, discovery_tools::DiscoverMcpToolsParams};

pub(crate) fn discover_mcp_tools_tool() -> McpToolDefinition {
    derived_tool_definition::<DiscoverMcpToolsParams>(
        "DiscoverMcpTools",
        "Discovers tools exposed by configured MCP servers, including gateway-managed servers.",
    )
}

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    match registry_name {
        "DiscoverMcpTools" => Some(discover_mcp_tools_tool()),
        _ => None,
    }
}
