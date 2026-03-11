use super::support::{
    bool_property, enum_string_property, integer_property_with_default, object_schema,
    string_property, tool_definition,
};
use crate::mcp::McpToolDefinition;
use serde_json::json;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "MemoryStatus" => tool_definition(
            registry_name,
            "Provides high-level memory system statistics and health information.",
            object_schema(
                vec![(
                    "learn",
                    bool_property("Return help documentation instead of executing", false),
                )],
                &[],
            ),
        ),
        "GetConfig" => tool_definition(
            registry_name,
            "Displays maenifold configuration settings and resolved paths.",
            object_schema(
                vec![(
                    "learn",
                    bool_property("Return help documentation instead of executing", false),
                )],
                &[],
            ),
        ),
        "GetHelp" => tool_definition(
            registry_name,
            "Retrieves detailed help documentation for a specific tool.",
            object_schema(
                vec![
                    ("toolName", string_property("Tool name to load help for")),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["toolName"],
            ),
        ),
        "UpdateAssets" => tool_definition(
            registry_name,
            "Refreshes runtime assets from packaged assets with optional dry-run mode.",
            object_schema(
                vec![
                    (
                        "dryRun",
                        bool_property("Preview changes without modifying files", true),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "StartWatcher" => tool_definition(
            registry_name,
            "Starts background watching of the memory directory for incremental sync.",
            object_schema(
                vec![(
                    "debounceMs",
                    integer_property_with_default("Debounce interval in milliseconds", json!(150)),
                )],
                &[],
            ),
        ),
        "StopWatcher" => tool_definition(
            registry_name,
            "Stops the background watcher used for incremental sync.",
            object_schema(vec![], &[]),
        ),
        "RunFullBenchmark" => tool_definition(
            registry_name,
            "Runs system performance benchmarks across search, sync, and graph traversal.",
            object_schema(
                vec![
                    (
                        "iterations",
                        integer_property_with_default(
                            "Benchmark iterations per category",
                            json!(5),
                        ),
                    ),
                    (
                        "maxTestFiles",
                        integer_property_with_default(
                            "Maximum test files to use during benchmarks",
                            json!(1000),
                        ),
                    ),
                    (
                        "includeDeepTraversal",
                        bool_property("Include expensive deep traversal benchmarks", true),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "Adopt" => tool_definition(
            registry_name,
            "Loads a role, color, or perspective asset as an adoption instruction.",
            object_schema(
                vec![
                    (
                        "type",
                        enum_string_property(
                            "Asset type to adopt",
                            &["role", "color", "perspective"],
                        )
                        .into(),
                    ),
                    (
                        "identifier",
                        string_property("Asset identifier or filename without the .json extension"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["type", "identifier"],
            ),
        ),
        "ListAssets" => tool_definition(
            registry_name,
            "Lists available asset types or metadata for a specific asset type.",
            object_schema(
                vec![
                    (
                        "type",
                        enum_string_property(
                            "Optional asset type to list",
                            &["workflow", "role", "color", "perspective"],
                        )
                        .into(),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "ReadMcpResource" => tool_definition(
            registry_name,
            "Reads MCP resource content by URI.",
            object_schema(
                vec![
                    (
                        "uri",
                        string_property("Resource URI such as asset://catalog"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["uri"],
            ),
        ),
        _ => return None,
    };

    Some(tool)
}
