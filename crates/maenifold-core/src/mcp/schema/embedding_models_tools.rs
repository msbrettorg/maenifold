use super::embedding_models_params::EmbeddingModelsParams;
use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    match registry_name {
        "EmbeddingModels" => Some(derived_tool_definition::<EmbeddingModelsParams>(
            registry_name,
            "Manages embedding models and ONNX Runtime downloads. List curated models, check status, and download runtime assets.",
        )),
        _ => None,
    }
}
