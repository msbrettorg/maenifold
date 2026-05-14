use super::support::derived_tool_definition;
use crate::mcp::{
    McpToolDefinition,
    discovery_tools::{DiscoverMcpPromptsParams, DiscoverMcpToolsParams, GetMcpPromptParams},
};

pub(crate) fn discover_mcp_tools_tool() -> McpToolDefinition {
    derived_tool_definition::<DiscoverMcpToolsParams>(
        "DiscoverMcpTools",
        "Inspects the live tool inventory exposed by configured downstream MCP servers, grouped by server. Use this when you need per-server MCP inventory or diagnostics. Do not use it as the first-call gateway domain lister; GatewayIndex is the live domain and command discovery tool.",
    )
}

pub(crate) fn discover_mcp_prompts_tool() -> McpToolDefinition {
    derived_tool_definition::<DiscoverMcpPromptsParams>(
        "DiscoverMcpPrompts",
        "Inspects the live prompt inventory exposed by configured downstream MCP servers, grouped by server. Use this when you need per-server MCP prompt names, descriptions, and arguments.",
    )
}

pub(crate) fn get_mcp_prompt_tool() -> McpToolDefinition {
    derived_tool_definition::<GetMcpPromptParams>(
        "GetMcpPrompt",
        "Retrieves one prompt from a specific downstream MCP server. Requires explicit server and name parameters so overlapping prompt names across servers are resolved without ambiguity.",
    )
}

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    match registry_name {
        "DiscoverMcpTools" => Some(discover_mcp_tools_tool()),
        "DiscoverMcpPrompts" => Some(discover_mcp_prompts_tool()),
        "GetMcpPrompt" => Some(get_mcp_prompt_tool()),
        _ => None,
    }
}
