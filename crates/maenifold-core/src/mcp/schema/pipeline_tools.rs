use super::params::PipelineParams;
use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Pipeline" => derived_tool_definition::<PipelineParams>(
            registry_name,
            "Creates and manages pipeline sessions composed of DAG nodes.",
        ),
        _ => return None,
    };

    Some(tool)
}
