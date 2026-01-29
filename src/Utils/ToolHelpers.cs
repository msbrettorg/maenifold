namespace Maenifold.Utils;

/// <summary>
/// Utility methods for MCP tool common operations.
/// </summary>
public static class ToolHelpers
{
    /// <summary>
    /// Returns help documentation content for a tool method.
    /// Reads from ~/maenifold/assets/usage/tools/{toolName}.md
    /// </summary>
    /// <param name="toolMethodName">Method name (typically from nameof() operator)</param>
    /// <returns>Help documentation content, or error message if help file not found</returns>
    /// <example>
    /// if (learn) return ToolHelpers.GetLearnContent(nameof(MyTool));
    /// </example>
    public static string GetLearnContent(string toolMethodName)
    {
        var toolName = toolMethodName.ToLowerInvariant();
        var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");

        if (!File.Exists(helpPath))
            return $"ERROR: Help file not found for {toolMethodName}";

        return File.ReadAllText(helpPath);
    }
}
