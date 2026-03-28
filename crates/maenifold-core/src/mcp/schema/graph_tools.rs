use super::support::{
    bool_property, integer_property_with_default, number_property_with_default, object_schema,
    string_property, tool_definition,
};
use crate::mcp::McpToolDefinition;
use serde_json::json;

#[allow(clippy::too_many_lines)]
pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "Sync" => tool_definition(
            registry_name,
            "Synchronizes WikiLink concepts from memory files into the graph database.",
            object_schema(
                vec![(
                    "learn",
                    bool_property("Return help documentation instead of executing", false),
                )],
                &[],
            ),
        ),
        "BuildContext" => tool_definition(
            registry_name,
            "Builds graph context around a concept by traversing related knowledge.",
            object_schema(
                vec![
                    (
                        "conceptName",
                        string_property("Concept name to build context around"),
                    ),
                    (
                        "depth",
                        integer_property_with_default("How many graph hops to traverse", json!(2)),
                    ),
                    (
                        "maxEntities",
                        integer_property_with_default(
                            "Maximum related entities to return",
                            json!(20),
                        ),
                    ),
                    (
                        "includeContent",
                        bool_property("Include content previews in the context response", false),
                    ),
                ],
                &["conceptName"],
            ),
        ),
        "Visualize" => tool_definition(
            registry_name,
            "Generates a Mermaid diagram for concept relationships.",
            object_schema(
                vec![
                    (
                        "conceptName",
                        string_property("Central concept for the visualization"),
                    ),
                    (
                        "depth",
                        integer_property_with_default("Graph traversal depth", json!(2)),
                    ),
                    (
                        "maxNodes",
                        integer_property_with_default(
                            "Maximum nodes to include in the diagram",
                            json!(30),
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["conceptName"],
            ),
        ),
        "RepairConcepts" => tool_definition(
            registry_name,
            "Repairs concept variants across memory files by replacing them with a canonical form.",
            object_schema(
                vec![
                    (
                        "conceptsToReplace",
                        string_property("Delimited concept variants to replace"),
                    ),
                    (
                        "canonicalConcept",
                        string_property("Canonical concept to keep after consolidation"),
                    ),
                    (
                        "folder",
                        string_property("Optional folder under the memory root to process"),
                    ),
                    (
                        "dryRun",
                        bool_property("Preview changes without modifying files", true),
                    ),
                    (
                        "createWikiLinks",
                        bool_property(
                            "Create WikiLinks from plain text instead of replacing existing ones",
                            false,
                        ),
                    ),
                    (
                        "minSemanticSimilarity",
                        number_property_with_default(
                            "Minimum semantic similarity required for safe consolidation",
                            json!(0.7),
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["conceptsToReplace", "canonicalConcept"],
            ),
        ),
        "AnalyzeConceptCorruption" => tool_definition(
            registry_name,
            "Analyzes concept families to find duplicate or corrupted WikiLink variants.",
            object_schema(
                vec![
                    (
                        "conceptFamily",
                        string_property("Concept family to analyze, such as tool or agent"),
                    ),
                    (
                        "maxResults",
                        integer_property_with_default("Maximum variants to include", json!(50)),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["conceptFamily"],
            ),
        ),
        "FindSimilarConcepts" => tool_definition(
            registry_name,
            "Finds semantically similar concepts using vector embeddings.",
            object_schema(
                vec![
                    (
                        "conceptName",
                        string_property("Concept name to search for related concepts"),
                    ),
                    (
                        "maxResults",
                        integer_property_with_default(
                            "Maximum number of similar concepts to return",
                            json!(10),
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["conceptName"],
            ),
        ),
        _ => return None,
    };

    Some(tool)
}
