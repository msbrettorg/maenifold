use super::params::PipelineParams;
use super::support::{schema_value_no_null_options, tool_definition};
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Pipeline" => tool_definition(
            registry_name,
            "Creates and manages pipeline sessions composed of DAG nodes.",
            schema_value_no_null_options::<PipelineParams>(),
        ),
        _ => return None,
    };

    Some(tool)
}
