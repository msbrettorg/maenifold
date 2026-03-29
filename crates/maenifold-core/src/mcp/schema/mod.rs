mod assumption_tools;
mod graph_tools;
mod memory_tools;
mod pipeline_node_schema;
mod pipeline_tools;
mod support;
mod system_tools;
mod thinking_tools;
mod workflow_tools;

#[cfg(test)]
mod pipeline_tools_tests;

use crate::error::{MaenifoldError, Result};
use crate::mcp::McpToolDefinition;

pub(crate) fn mcp_tool_from_registry_name(registry_name: &str) -> Result<McpToolDefinition> {
    memory_tools::tool(registry_name)
        .or_else(|| graph_tools::tool(registry_name))
        .or_else(|| thinking_tools::tool(registry_name))
        .or_else(|| pipeline_tools::tool(registry_name))
        .or_else(|| workflow_tools::tool(registry_name))
        .or_else(|| assumption_tools::tool(registry_name))
        .or_else(|| system_tools::tool(registry_name))
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "Missing MCP tool schema mapping for registry tool {registry_name}"
            ))
        })
}
