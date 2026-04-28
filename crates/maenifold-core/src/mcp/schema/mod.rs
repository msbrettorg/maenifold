mod assumption_params;
mod assumption_tools;
pub(crate) mod discovery_tools;
mod embedding_models_params;
mod embedding_models_tools;
mod files_tools;
pub(crate) mod gateway_tools;
#[cfg(test)]
mod gateway_tools_schema_tests;
#[cfg(test)]
mod generic_tool_tests;
mod graph_params;
#[cfg(test)]
mod graph_params_tests;
mod graph_tools;
mod memory_params;
mod memory_tools;
mod params;
mod pipeline_node_schema;
mod pipeline_node_schema_data_variants;
mod pipeline_node_schema_workflow_variants;
mod pipeline_node_types;
mod pipeline_params;
#[cfg(test)]
mod pipeline_params_tests;
mod pipeline_tools;
mod support;
mod system_params;
mod system_tools;
mod thinking_tools;
mod thinking_workflow_params;
mod workflow_tools;

use crate::error::Result;
use crate::mcp::McpToolDefinition;
use serde_json::json;

pub(crate) use params::{
    AcpExecContextBlock, AcpExecImageContext, AcpExecResourceContext, AcpExecTextContext,
    EmbeddingModelsAction, EmbeddingModelsParams,
};

pub(crate) fn mcp_tool_from_registry_name(registry_name: &str) -> Result<McpToolDefinition> {
    Ok(memory_tools::tool(registry_name)
        .or_else(|| graph_tools::tool(registry_name))
        .or_else(|| thinking_tools::tool(registry_name))
        .or_else(|| pipeline_tools::tool(registry_name))
        .or_else(|| workflow_tools::tool(registry_name))
        .or_else(|| assumption_tools::tool(registry_name))
        .or_else(|| files_tools::tool(registry_name))
        .or_else(|| discovery_tools::tool(registry_name))
        .or_else(|| system_tools::tool(registry_name))
        .or_else(|| embedding_models_tools::tool(registry_name))
        .unwrap_or_else(|| {
            tracing::warn!("No schema mapping for tool {registry_name}, using generic schema");
            McpToolDefinition {
                name: registry_name.to_string(),
                description: "Gateway tool (use learn=true for details)".to_string(),
                input_schema: json!({ "type": "object" }),
            }
        }))
}
