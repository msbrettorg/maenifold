pub(super) fn usage() -> String {
    [
        "maenifold CLI",
        "",
        "Usage:",
        "  MCP Server mode:   maenifold --mcp",
        "  HTTP Server mode:  maenifold --http [--port <PORT>]",
        "  App mode:          maenifold --app [--port <PORT>]",
        "  Tool mode:         maenifold tool <name> '<json-payload>' [--json]",
        "  Schedule mode:     maenifold schedule run <task-id>",
        "",
        "  ACP operations are available only through the MCP AcpExec tool.",
        "  The prompt, think, and workflow CLI aliases have been removed.",
        "",
        "  (For development from source: cargo run -p maenifold-cli -- tool <name> '<json>')",
        "",
        "Options:",
        "  --json             Output structured JSON instead of markdown (FR-8.1)",
        "",
        "Examples:",
        "  maenifold tool WriteMemory '{\"title\":\"Test\",\"content\":\"Content with [[WikiLink]]\"}'",
        "  maenifold tool SearchMemories '{\"query\":\"authentication\",\"mode\":\"Hybrid\"}'",
        "  maenifold tool BuildContext '{\"conceptName\":\"Machine Learning\",\"depth\":2}'",
        "",
        "Available tools:",
        "  Memory: WriteMemory, ReadMemory, EditMemory, DeleteMemory, MoveMemory",
        "          SearchMemories, ExtractConceptsFromFile, RecentActivity",
        "  Vector: FindSimilarConcepts",
        "  Graph:  Sync, BuildContext, Visualize",
        "  Think:  SequentialThinking",
        "  Flow:   Workflow",
        "  System: MemoryStatus, ListMemories, GetConfig, GetHelp",
        "  Repair: RepairConcepts, AnalyzeConceptCorruption",
        "  MCP:    ListAssets, ReadMcpResource, AcpExec",
        "  Bench:  RunFullBenchmark",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::usage;

    #[test]
    fn usage_text_does_not_mention_unregistered_tools() {
        let usage_text = usage();

        assert!(!usage_text.contains("StartWatcher"));
        assert!(!usage_text.contains("StopWatcher"));
    }
}
