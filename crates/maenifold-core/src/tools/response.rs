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
