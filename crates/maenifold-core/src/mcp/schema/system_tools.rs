use super::params::{
    AcpPromptParams, AdoptParams, GetConfigParams, GetHelpParams, ListAgentsParams,
    ListAssetsParams, MemoryStatusParams, ReadMcpResourceParams, RunFullBenchmarkParams,
    StartWatcherParams, StopWatcherParams, UpdateAssetsParams,
};
use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "MemoryStatus" => derived_tool_definition::<MemoryStatusParams>(
            registry_name,
            "Provides high-level memory system statistics and health information.",
        ),
        "GetConfig" => derived_tool_definition::<GetConfigParams>(
            registry_name,
            "Displays maenifold configuration settings and resolved paths.",
        ),
        "GetHelp" => derived_tool_definition::<GetHelpParams>(
            registry_name,
            "Retrieves detailed help documentation for a specific tool.",
        ),
        "UpdateAssets" => derived_tool_definition::<UpdateAssetsParams>(
            registry_name,
            "Refreshes runtime assets from packaged assets with optional dry-run mode.",
        ),
        "StartWatcher" => derived_tool_definition::<StartWatcherParams>(
            registry_name,
            "Starts background watching of the memory directory for incremental sync.",
        ),
        "StopWatcher" => derived_tool_definition::<StopWatcherParams>(
            registry_name,
            "Stops the background watcher used for incremental sync.",
        ),
        "RunFullBenchmark" => derived_tool_definition::<RunFullBenchmarkParams>(
            registry_name,
            "Runs system performance benchmarks across search, sync, and graph traversal.",
        ),
        "Adopt" => derived_tool_definition::<AdoptParams>(
            registry_name,
            "Loads a role, color, or perspective asset as an adoption instruction.",
        ),
        "ListAssets" => derived_tool_definition::<ListAssetsParams>(
            registry_name,
            "Lists available asset types or metadata for a specific asset type.",
        ),
        "ReadMcpResource" => derived_tool_definition::<ReadMcpResourceParams>(
            registry_name,
            "Reads MCP resource content by URI.",
        ),
        "ListAgents" => derived_tool_definition::<ListAgentsParams>(
            registry_name,
            "Lists configured ACP agents and their serialized configuration fields.",
        ),
        "AcpPrompt" => derived_tool_definition::<AcpPromptParams>(
            registry_name,
            "Spawns an ACP agent, creates a session, sends a single prompt, and returns the response text.",
        ),
        _ => return None,
    };

    Some(tool)
}
