using System.ComponentModel;
using System.Text.Json;
using ModelContextProtocol.Server;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public class AdoptTools
{
    private static string AssetsPath => Config.AssetsPath;

    [McpServerTool]
    [Description("Adopt a role, color, or perspective by reading its JSON configuration from assets. Use ListMcpResources or ReadMcpResource with asset://catalog to discover available roles, colors, and perspectives before calling this tool.")]
    public static async Task<string> Adopt(
        [Description("Type of asset to adopt: 'role', 'color', or 'perspective'")] string type,
        [Description("Identifier of the asset (use id from catalog, not filename)")] string identifier,
        [Description("Return help documentation instead of executing")] bool learn = false
    )
    {
        if (learn)
        {
            var toolName = nameof(Adopt).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(Adopt)}";
            return File.ReadAllText(helpPath);
        }

        var validTypes = new[] { "role", "color", "perspective" };
        if (!validTypes.Contains(type.ToLowerInvariant(), StringComparer.OrdinalIgnoreCase))
        {
            throw new ArgumentException($"Invalid type '{type}'. Must be one of: {string.Join(", ", validTypes)}");
        }

        var folderName = type.ToLowerInvariant() + "s";

        // Use id-based lookup to handle filename != id cases (e.g., perspectives)
        var filePath = FindAssetFileById(folderName, identifier);

        if (filePath == null)
        {
            throw new FileNotFoundException($"Asset not found: {type}/{identifier}");
        }

        return await File.ReadAllTextAsync(filePath);
    }

    /// <summary>
    /// Find asset file by searching for matching JSON id field (handles filename != id cases)
    /// </summary>
    private static string? FindAssetFileById(string assetType, string id)
    {
        var assetPath = Path.Combine(AssetsPath, assetType);
        if (!Directory.Exists(assetPath))
        {
            return null;
        }

        // First try direct filename match (fast path for most workflows/colors)
        var directPath = Path.Combine(assetPath, $"{id}.json");
        if (File.Exists(directPath))
        {
            return directPath;
        }

        // Search all files for matching id field (handles perspectives/roles with mismatched names)
        foreach (var file in Directory.GetFiles(assetPath, "*.json"))
        {
            try
            {
                var json = File.ReadAllText(file);
                using var doc = JsonDocument.Parse(json);
                if (doc.RootElement.TryGetProperty("id", out var idProp) && idProp.GetString() == id)
                {
                    return file;
                }
            }
            catch
            {
                // Skip malformed files
            }
        }

        return null;
    }
}