using System.ComponentModel;
using System.Text.RegularExpressions;
using ModelContextProtocol.Server;

namespace Maenifold.Tools;

[McpServerToolType]
public static class McpResourceTools
{
    [McpServerTool]
    [Description("Lists all available MCP resources with metadata (CLI-accessible)")]
    public static string ListMcpResources()
    {
        return AssetResources.GetCatalog();
    }

    [McpServerTool]
    [Description("Reads MCP resource content by URI (CLI-accessible)")]
    public static string ReadMcpResource(
        [Description("Resource URI (e.g., 'asset://catalog', 'asset://workflows/deductive-reasoning')")]
        string uri)
    {
        if (string.IsNullOrWhiteSpace(uri))
            throw new ArgumentException("URI is required", nameof(uri));

        if (uri == "asset://catalog")
            return AssetResources.GetCatalog();

        var match = Regex.Match(uri, @"^asset://([^/]+)/(.+)$");
        if (!match.Success)
            throw new ArgumentException($"Invalid resource URI format: {uri}. Expected 'asset://type/id'", nameof(uri));

        var type = match.Groups[1].Value;
        var id = match.Groups[2].Value;

        return type switch
        {
            "workflows" => AssetResources.GetWorkflow(id),
            "roles" => AssetResources.GetRole(id),
            "colors" => AssetResources.GetColor(id),
            "perspectives" => AssetResources.GetPerspective(id),
            _ => throw new ArgumentException($"Unknown resource type: {type}. Valid types: workflows, roles, colors, perspectives", nameof(uri))
        };
    }
}
