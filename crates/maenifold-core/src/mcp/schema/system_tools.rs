use super::params::{
    AcpExecParams, AdoptParams, GetConfigParams, GetHelpParams, ListAssetsParams,
    ListSessionsParams, MemoryStatusParams, ReadMcpResourceParams, RunFullBenchmarkParams,
    UpdateAssetsParams,
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
            "Loads canonical help for a named public MCP tool or gateway command. Use it after you know which tool or command needs deeper usage details. It complements GatewayIndex; it is not the first cold-start domain discovery call.",
        ),
        "UpdateAssets" => derived_tool_definition::<UpdateAssetsParams>(
            registry_name,
            "Refreshes runtime assets from packaged assets with optional dry-run mode.",
        ),
        // StartWatcher/StopWatcher removed — vestigial, caused empty schema errors
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
            "Reads a concrete MCP resource URI such as asset://catalog or asset://.../id. Use it when you already have a resource URI and need packaged resource content. It does not discover gateway domains or read memory:// notes; use GatewayIndex for domains.",
        ),
        "ListSessions" => derived_tool_definition::<ListSessionsParams>(
            registry_name,
            "Lists resumable ACP sessions across configured agents.",
        ),
        "AcpExec" => derived_tool_definition::<AcpExecParams>(
            registry_name,
            "Unified ACP agent invocation. Supports prompt (single message), think (sequential thinking), and workflow (structured workflow) modes. All modes share a global agent pool for process reuse.",
        ),
        _ => return None,
    };

    Some(tool)
}
