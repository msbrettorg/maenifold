use super::params::{
    AnalyzeConceptCorruptionParams, BuildContextParams, FindSimilarConceptsParams,
    RepairConceptsParams, SyncParams, VisualizeParams,
};
use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Sync" => derived_tool_definition::<SyncParams>(
            registry_name,
            "Synchronizes WikiLink concepts from memory files into the graph database.",
        ),
        "BuildContext" => derived_tool_definition::<BuildContextParams>(
            registry_name,
            "Builds graph context around a concept by traversing related knowledge.",
        ),
        "Visualize" => derived_tool_definition::<VisualizeParams>(
            registry_name,
            "Generates a Mermaid diagram for concept relationships.",
        ),
        "RepairConcepts" => derived_tool_definition::<RepairConceptsParams>(
            registry_name,
            "Repairs concept variants across memory files by replacing them with a canonical form.",
        ),
        "AnalyzeConceptCorruption" => {
            derived_tool_definition::<AnalyzeConceptCorruptionParams>(
                registry_name,
                "Analyzes concept families to find duplicate or corrupted WikiLink variants.",
            )
        }
        "FindSimilarConcepts" => derived_tool_definition::<FindSimilarConceptsParams>(
            registry_name,
            "Finds semantically similar concepts using vector embeddings.",
        ),
        _ => return None,
    };

    Some(tool)
}
