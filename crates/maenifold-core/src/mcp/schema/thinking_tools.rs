use super::params::SequentialThinkingParams;
use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "SequentialThinking" => derived_tool_definition::<SequentialThinkingParams>(
            registry_name,
            "Creates structured thinking sessions with persistent branching, conclusions, and child submachines.",
        ),
        _ => return None,
    };

    Some(tool)
}
