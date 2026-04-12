use super::params::PipelineParams;
use super::support::{schema_value_no_null_options, tool_definition};
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Pipeline" => tool_definition(
            registry_name,
            "Create and manage DAG pipeline sessions with autonomous execution, lifecycle controls, topology edits (graft, join, fork, rewind), and runtime inputs.",
            schema_value_no_null_options::<PipelineParams>(),
        ),
        _ => return None,
    };

    Some(tool)
}
