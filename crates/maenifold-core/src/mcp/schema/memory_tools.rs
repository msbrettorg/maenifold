use super::params::{
    DeleteMemoryParams, EditMemoryParams, ExtractConceptsFromFileParams, ListMemoriesParams,
    MoveMemoryParams, ReadMemoryParams, SearchMemoriesParams, WriteMemoryParams,
};
use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "WriteMemory" => derived_tool_definition::<WriteMemoryParams>(
            registry_name,
            "Creates new knowledge files with [[WikiLinks]] for graph integration.",
        ),
        "ReadMemory" => derived_tool_definition::<ReadMemoryParams>(
            registry_name,
            "Retrieves an existing knowledge file by URI or title.",
        ),
        "EditMemory" => derived_tool_definition::<EditMemoryParams>(
            registry_name,
            "Modifies an existing knowledge file while preserving WikiLinks.",
        ),
        "DeleteMemory" => derived_tool_definition::<DeleteMemoryParams>(
            registry_name,
            "Deletes an existing knowledge file after explicit confirmation.",
        ),
        "MoveMemory" => derived_tool_definition::<MoveMemoryParams>(
            registry_name,
            "Moves or renames a knowledge file while preserving WikiLinks.",
        ),
        "SearchMemories" => derived_tool_definition::<SearchMemoriesParams>(
            registry_name,
            "Searches knowledge files using hybrid, semantic, or full-text ranking.",
        ),
        "ExtractConceptsFromFile" => derived_tool_definition::<ExtractConceptsFromFileParams>(
            registry_name,
            "Extracts WikiLink concepts from a specific knowledge file.",
        ),
        "ListMemories" => derived_tool_definition::<ListMemoriesParams>(
            registry_name,
            "Lists knowledge files or folders relative to the memory root.",
        ),
        _ => return None,
    };

    Some(tool)
}
