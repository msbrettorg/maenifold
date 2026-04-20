use super::embedding_models_params::EmbeddingModelsParams;
use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    match registry_name {
        "EmbeddingModels" => Some(derived_tool_definition::<EmbeddingModelsParams>(
            registry_name,
            "Manages embedding models. List available curated models, show current model status.",
        )),
        _ => None,
    }
}
