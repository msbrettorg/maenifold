using System.ComponentModel;
using ModelContextProtocol.Server;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public class AdoptTools
{
    private static string AssetsPath => Config.AssetsPath;

    [McpServerTool]
    [Description("Adopt a role, color, or perspective by reading its JSON configuration from assets")]
    public static async Task<string> Adopt(
        [Description("Type of asset to adopt: 'role', 'color', or 'perspective'")] string type,
        [Description("Identifier of the asset (filename without .json extension)")] string identifier,
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


        var filePath = Path.Combine(AssetsPath, folderName, $"{identifier}.json");


        if (!File.Exists(filePath))
        {
            throw new FileNotFoundException($"Asset not found: {type}/{identifier}");
        }


        return await File.ReadAllTextAsync(filePath);
    }
}