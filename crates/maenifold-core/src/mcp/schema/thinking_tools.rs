use super::support::{
    bool_property, integer_property, integer_property_with_default, object_schema, string_property,
    tool_definition,
};
use crate::mcp::McpToolDefinition;
use serde_json::json;

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "SequentialThinking" => sequential_thinking_tool(registry_name),
        _ => return None,
    };

    Some(tool)
}

#[allow(clippy::too_many_lines)]
fn sequential_thinking_tool(registry_name: &str) -> McpToolDefinition {
    tool_definition(
        registry_name,
        "Creates structured thinking sessions with persistent branching and conclusions.",
        object_schema(
            vec![
                (
                    "response",
                    string_property(
                        "Main thought content; must include [[WikiLinks]] unless cancelling",
                    ),
                ),
                (
                    "nextThoughtNeeded",
                    bool_property("Whether another thought is needed after this one", false),
                ),
                (
                    "thoughtNumber",
                    integer_property_with_default("Current thought number", json!(0)),
                ),
                (
                    "totalThoughts",
                    integer_property_with_default("Current estimate for total thoughts", json!(0)),
                ),
                (
                    "sessionId",
                    string_property("Existing sequential thinking session identifier"),
                ),
                (
                    "cancel",
                    bool_property("Cancel the session instead of adding a thought", false),
                ),
                (
                    "thoughts",
                    string_property("Optional meta-thoughts; include [[WikiLinks]] when provided"),
                ),
                (
                    "isRevision",
                    bool_property("Marks this thought as a revision of prior thinking", false),
                ),
                (
                    "revisesThought",
                    integer_property("Thought number that this revision updates"),
                ),
                (
                    "branchFromThought",
                    integer_property("Thought number where this branch diverges"),
                ),
                (
                    "branchId",
                    string_property("Branch identifier for multi-agent coordination"),
                ),
                (
                    "needsMoreThoughts",
                    bool_property(
                        "Whether more thoughts are needed than originally estimated",
                        false,
                    ),
                ),
                (
                    "analysisType",
                    string_property(
                        "Optional analysis type annotation such as bug or architecture",
                    ),
                ),
                (
                    "parentWorkflowId",
                    string_property("Optional parent workflow session identifier"),
                ),
                (
                    "conclusion",
                    string_property("Structured ConfessionReport used when completing the session"),
                ),
                (
                    "learn",
                    bool_property("Return help documentation instead of executing", false),
                ),
            ],
            &[],
        ),
    )
}
