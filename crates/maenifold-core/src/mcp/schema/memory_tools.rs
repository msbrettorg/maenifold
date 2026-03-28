use super::support::{
    bool_property, enum_string_property, integer_property, integer_property_with_default,
    number_property_with_default, object_schema, string_array_property, string_property,
    tool_definition,
};
use crate::mcp::McpToolDefinition;
use serde_json::json;

#[allow(clippy::too_many_lines)]
pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "WriteMemory" => tool_definition(
            registry_name,
            "Creates new knowledge files with [[WikiLinks]] for graph integration.",
            object_schema(
                vec![
                    ("title", string_property("Title for this memory file")),
                    (
                        "content",
                        string_property(
                            "Content with [[Concept Names]]; at least one [[concept]] is required",
                        ),
                    ),
                    (
                        "folder",
                        string_property("Optional folder path for organizing files"),
                    ),
                    (
                        "tags",
                        string_array_property("Optional tags for categorizing this file"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["title", "content"],
            ),
        ),
        "ReadMemory" => tool_definition(
            registry_name,
            "Retrieves an existing knowledge file by URI or title.",
            object_schema(
                vec![
                    (
                        "identifier",
                        string_property("Memory file identifier: URI or exact title"),
                    ),
                    (
                        "includeChecksum",
                        bool_property("Return checksum with content for safe editing", true),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["identifier"],
            ),
        ),
        "EditMemory" => tool_definition(
            registry_name,
            "Modifies an existing knowledge file while preserving WikiLinks.",
            object_schema(
                vec![
                    (
                        "identifier",
                        string_property("Memory file identifier: URI or exact title"),
                    ),
                    (
                        "operation",
                        enum_string_property(
                            "Edit operation to apply",
                            &["append", "prepend", "find_replace", "replace_section"],
                        )
                        .into(),
                    ),
                    (
                        "content",
                        string_property(
                            "Content to add or replace; must include at least one [[concept]]",
                        ),
                    ),
                    (
                        "checksum",
                        string_property(
                            "File checksum from the last read for stale-edit protection",
                        ),
                    ),
                    (
                        "findText",
                        string_property("Text to find when operation is find_replace"),
                    ),
                    (
                        "sectionName",
                        string_property("Section name when operation is replace_section"),
                    ),
                    (
                        "expectedCount",
                        integer_property("Expected match count when operation is find_replace"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["identifier", "operation", "content"],
            ),
        ),
        "DeleteMemory" => tool_definition(
            registry_name,
            "Deletes an existing knowledge file after explicit confirmation.",
            object_schema(
                vec![
                    (
                        "identifier",
                        string_property("Memory file identifier: URI or exact title"),
                    ),
                    (
                        "confirm",
                        bool_property("Confirm deletion of the file", false),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["identifier"],
            ),
        ),
        "MoveMemory" => tool_definition(
            registry_name,
            "Moves or renames a knowledge file while preserving WikiLinks.",
            object_schema(
                vec![
                    (
                        "source",
                        string_property("Source memory file identifier: URI or exact title"),
                    ),
                    (
                        "destination",
                        string_property("Destination file name or relative path"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["source", "destination"],
            ),
        ),
        "SearchMemories" => tool_definition(
            registry_name,
            "Searches knowledge files using hybrid, semantic, or full-text ranking.",
            object_schema(
                vec![
                    ("query", string_property("Search query over file contents")),
                    (
                        "mode",
                        enum_string_property("Search mode", &["Hybrid", "Semantic", "FullText"])
                            .with_default(json!("Hybrid")),
                    ),
                    (
                        "pageSize",
                        integer_property_with_default("Maximum files to return", json!(10)),
                    ),
                    (
                        "page",
                        integer_property_with_default("Page number for results", json!(1)),
                    ),
                    (
                        "folder",
                        string_property("Optional folder path filter under the memory root"),
                    ),
                    (
                        "tags",
                        string_array_property("Optional tag filter; all provided tags must match"),
                    ),
                    (
                        "minScore",
                        number_property_with_default("Minimum score threshold", json!(0.0)),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["query"],
            ),
        ),
        "ExtractConceptsFromFile" => tool_definition(
            registry_name,
            "Extracts WikiLink concepts from a specific knowledge file.",
            object_schema(
                vec![
                    (
                        "identifier",
                        string_property("Memory file identifier: URI or exact title"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["identifier"],
            ),
        ),
        "ListMemories" => tool_definition(
            registry_name,
            "Lists knowledge files or folders relative to the memory root.",
            object_schema(
                vec![
                    (
                        "path",
                        string_property("Optional directory path relative to the memory root"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        _ => return None,
    };

    Some(tool)
}
