use super::params::PipelineParams;
use super::support::{schema_value_no_null_options, tool_definition};
use crate::mcp::McpToolDefinition;
use rmcp::model::{TaskSupport, ToolExecution};

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Pipeline" => {
            let mut tool = tool_definition(
                registry_name,
                "Creates and manages pipeline sessions composed of DAG nodes.",
                schema_value_no_null_options::<PipelineParams>(),
            );
            tool.execution =
                Some(ToolExecution::new().with_task_support(TaskSupport::Optional));
            tool
        }
        _ => return None,
    };

    Some(tool)
}
