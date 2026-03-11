// T-13.5: RTM FR-8 — ToolResponse helpers ported from C# Utils/ToolResponse.cs
//
// C# signature: `public static string WithNextSteps(string result, params string[] nextTools)`
// C# signature: `public static string WithHint(string result, string toolName, string hint)`
//
// These helpers append structured guidance to tool responses, helping LLM
// consumers discover follow-up actions.  Used ~26 times across C# tool
// implementations (VectorSearchTools, MemorySearchTools, MemoryTools).

/// Appends a formatted "Next steps" hint listing suggested follow-up tools.
///
/// Matches C# `ToolResponse.WithNextSteps(result, params string[] nextTools)`.
///
/// # Example
/// ```
/// # use maenifold_core::tools::with_next_steps;
/// let output = with_next_steps("Found 3 matches", &["ReadMemory", "BuildContext"]);
/// assert!(output.contains("💡 Next: Consider using ReadMemory, BuildContext"));
/// ```
pub fn with_next_steps(result: &str, next_tools: &[&str]) -> String {
    let steps = next_tools.join(", ");
    format!("{result}\n💡 Next: Consider using {steps} for related operations.")
}

/// Appends a contextual hint with a `GetHelp` reference for the named tool.
///
/// Matches C# `ToolResponse.WithHint(result, toolName, hint)`.
///
/// # Example
/// ```
/// # use maenifold_core::tools::with_hint;
/// let output = with_hint("Created memory FILE: memory://test", "WriteMemory", "Use ReadMemory with this URI to access content");
/// assert!(output.contains("💡 Use ReadMemory"));
/// assert!(output.contains("GetHelp(\"WriteMemory\")"));
/// ```
pub fn with_hint(result: &str, tool_name: &str, hint: &str) -> String {
    format!("{result}\n💡 {hint} Run GetHelp(\"{tool_name}\") for complete documentation.")
}

#[cfg(test)]
mod tests {
    use super::*;

    // T-13.5: RTM FR-8 — verify with_next_steps appends formatted list
    #[test]
    fn with_next_steps_appends_formatted_list() {
        let result = with_next_steps("Found 3 results", &["ReadMemory", "BuildContext"]);
        assert_eq!(
            result,
            "Found 3 results\n💡 Next: Consider using ReadMemory, BuildContext for related operations."
        );
    }

    // T-13.5: RTM FR-8 — verify single tool in next steps
    #[test]
    fn with_next_steps_single_tool() {
        let result = with_next_steps("Done", &["Sync"]);
        assert_eq!(
            result,
            "Done\n💡 Next: Consider using Sync for related operations."
        );
    }

    // T-13.5: RTM FR-8 — verify three tools in next steps
    #[test]
    fn with_next_steps_three_tools() {
        let result = with_next_steps(
            "Search complete",
            &["ReadMemory", "BuildContext", "FindSimilarConcepts"],
        );
        assert_eq!(
            result,
            "Search complete\n💡 Next: Consider using ReadMemory, BuildContext, FindSimilarConcepts for related operations."
        );
    }

    // T-13.5: RTM FR-8 — verify with_hint appends formatted hint
    #[test]
    fn with_hint_appends_formatted_hint() {
        let result = with_hint(
            "Created memory FILE: memory://test\nChecksum: abc123",
            "WriteMemory",
            "Use ReadMemory with this URI to access content",
        );
        assert_eq!(
            result,
            "Created memory FILE: memory://test\nChecksum: abc123\n💡 Use ReadMemory with this URI to access content Run GetHelp(\"WriteMemory\") for complete documentation."
        );
    }

    // T-13.5: RTM FR-8 — verify hint matches C# format exactly
    #[test]
    fn with_hint_matches_csharp_format() {
        // C#: $"{result}\n💡 {hint} Run GetHelp(\"{toolName}\") for complete documentation."
        let result = with_hint(
            "ERROR: Query contains no informative keywords.",
            "SearchMemories",
            "Provide at least one non-stopword keyword",
        );
        assert!(result.starts_with("ERROR: Query contains no informative keywords."));
        assert!(result.contains("💡 Provide at least one non-stopword keyword"));
        assert!(result.contains("Run GetHelp(\"SearchMemories\") for complete documentation."));
    }

    // T-13.5: RTM FR-8 — verify with_next_steps preserves multiline result
    #[test]
    fn with_next_steps_preserves_multiline_result() {
        let multiline = "Line 1\nLine 2\nLine 3";
        let result = with_next_steps(multiline, &["BuildContext", "SearchMemories"]);
        assert!(result.starts_with("Line 1\nLine 2\nLine 3\n💡"));
    }
}
