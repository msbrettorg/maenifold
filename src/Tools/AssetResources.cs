using System.ComponentModel;
using System.Text.Json;
using ModelContextProtocol.Server;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerResourceType]
public class AssetResources
{
    private static readonly JsonSerializerOptions s_jsonOptions = new() { WriteIndented = true };

    // Direct resource: catalog of available assets (metadata only, no content)
    [McpServerResource(UriTemplate = "asset://catalog", Name = "asset_catalog", MimeType = "application/json")]
    [Description("Catalog of all available assets with metadata (id, name, emoji, description) organized by type")]
    public static string GetCatalog()
    {
        var catalog = new
        {
            workflows = GetAssetMetadata("workflows"),
            roles = GetAssetMetadata("roles"),
            colors = GetAssetMetadata("colors"),
            perspectives = GetAssetMetadata("perspectives")
        };
        return JsonSerializer.Serialize(catalog, s_jsonOptions);
    }

    // Resource templates: fetch individual assets by ID
    [McpServerResource(UriTemplate = "asset://workflows/{id}", Name = "workflow", MimeType = "application/json")]
    [Description("Access workflow methodology JSON definition by ID")]
    public static string GetWorkflow(string id)
    {
        var path = Path.Combine(Config.AssetsPath, "workflows", $"{id}.json");
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"Workflow '{id}' not found", path);
        }
        return File.ReadAllText(path);
    }

    [McpServerResource(UriTemplate = "asset://roles/{id}", Name = "role", MimeType = "application/json")]
    [Description("Access role definition JSON file by ID")]
    public static string GetRole(string id)
    {
        var filePath = FindAssetFileById("roles", id);
        if (filePath == null)
        {
            throw new FileNotFoundException($"Role '{id}' not found");
        }
        return File.ReadAllText(filePath);
    }

    [McpServerResource(UriTemplate = "asset://colors/{id}", Name = "color", MimeType = "application/json")]
    [Description("Access color thinking hat JSON file by ID")]
    public static string GetColor(string id)
    {
        var path = Path.Combine(Config.AssetsPath, "colors", $"{id}.json");
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"Color '{id}' not found", path);
        }
        return File.ReadAllText(path);
    }

    [McpServerResource(UriTemplate = "asset://perspectives/{id}", Name = "perspective", MimeType = "application/json")]
    [Description("Access perspective linguistic frame JSON file by ID")]
    public static string GetPerspective(string id)
    {
        var filePath = FindAssetFileById("perspectives", id);
        if (filePath == null)
        {
            throw new FileNotFoundException($"Perspective '{id}' not found");
        }
        return File.ReadAllText(filePath);
    }

    /// <summary>
    /// Find asset file by searching for matching JSON id field (handles filename != id cases)
    /// </summary>
    private static string? FindAssetFileById(string assetType, string id)
    {
        var assetPath = Path.Combine(Config.AssetsPath, assetType);
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

    internal static object[] GetAssetMetadata(string assetType)
    {
        var assetPath = Path.Combine(Config.AssetsPath, assetType);
        if (!Directory.Exists(assetPath))
        {
            return Array.Empty<object>();
        }

        var metadata = new List<object>();

        foreach (var file in Directory.GetFiles(assetPath, "*.json").OrderBy(f => f))
        {
            try
            {
                var json = File.ReadAllText(file);
                using var doc = JsonDocument.Parse(json);
                var root = doc.RootElement;

                // Extract standard fields (id is required, others optional with defaults)
                var id = root.TryGetProperty("id", out var idProp)
                    ? idProp.GetString()
                    : Path.GetFileNameWithoutExtension(file);

                var name = root.TryGetProperty("name", out var nameProp)
                    ? nameProp.GetString()
                    : id;

                var emoji = root.TryGetProperty("emoji", out var emojiProp)
                    ? emojiProp.GetString()
                    : null;

                var description = root.TryGetProperty("description", out var descProp)
                    ? descProp.GetString()
                    : null;

                metadata.Add(new
                {
                    id = id,
                    name = name,
                    emoji = emoji,
                    description = description
                });
            }
            catch
            {
                // Skip malformed JSON files
            }
        }

        return metadata.ToArray();
    }
}
