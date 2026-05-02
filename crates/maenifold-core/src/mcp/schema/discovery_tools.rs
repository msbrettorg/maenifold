use super::support::derived_tool_definition;
use crate::mcp::{McpToolDefinition, discovery_tools::DiscoverMcpToolsParams};

pub(crate) fn discover_mcp_tools_tool() -> McpToolDefinition {
    derived_tool_definition::<DiscoverMcpToolsParams>(
        "DiscoverMcpTools",
        "Inspects the live tool inventory exposed by configured downstream MCP servers, grouped by server. Use this when you need per-server MCP inventory or diagnostics. Do not use it as the first-call gateway domain lister; GatewayIndex is the live domain and command discovery tool.",
    )
}

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    match registry_name {
        "DiscoverMcpTools" => Some(discover_mcp_tools_tool()),
        _ => None,
    }
}
