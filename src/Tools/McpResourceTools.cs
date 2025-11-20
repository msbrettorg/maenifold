using System.Collections.Generic;
using System.ComponentModel;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;
using ModelContextProtocol.Server;

namespace Maenifold.Tools;

[McpServerToolType]
public static class McpResourceTools
{
    private static readonly string[] s_primaryAssetTypes =
        { "workflow", "role", "color", "perspective" };

    private static readonly IReadOnlyDictionary<string, string> s_assetTypeMap =
        new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["workflow"] = "workflows",
            ["workflows"] = "workflows",
            ["role"] = "roles",
            ["roles"] = "roles",
            ["color"] = "colors",
            ["colors"] = "colors",
            ["perspective"] = "perspectives",
            ["perspectives"] = "perspectives"
        };

    private static readonly JsonSerializerOptions s_serializerOptions = new()
    {
        WriteIndented = true
    };

    [McpServerTool]
    [Description("Lists available asset types or the metadata for a specific asset type")]
    public static string ListAssets(
        [Description("Asset type to list: workflow, role, color, perspective (optional)")]
        string? type = null)
    {
        if (string.IsNullOrWhiteSpace(type))
        {
            return JsonSerializer.Serialize(s_primaryAssetTypes, s_serializerOptions);
        }

        var normalizedType = type.Trim();
        if (!s_assetTypeMap.TryGetValue(normalizedType, out var assetFolder))
        {
            throw new ArgumentException(
                $"Unknown asset type '{type}'. Valid types: workflow, role, color, perspective",
                nameof(type));
        }

        var metadata = AssetResources.GetAssetMetadata(assetFolder);
        return JsonSerializer.Serialize(metadata, s_serializerOptions);
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
