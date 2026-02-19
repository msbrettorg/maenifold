// T-APP-001.1: MCP App — ExploreConceptMap tool
// T-APP-001.6: MCP App — concept-map UI resource
using System.ComponentModel;
using ModelContextProtocol.Server;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public class ConceptMapApp
{
    // NOTE: [McpMeta] with JsonValue/ui/resourceUri is not available in ModelContextProtocol 0.8.0-preview.1.
    // The ui link is documented here for future SDK support.
    // Resource URI: ui://maenifold/concept-map
    [McpServerTool]
    [Description("Open an interactive concept map explorer centered on a concept")]
    public static object ExploreConceptMap(
        [Description("Concept name to explore (e.g. 'authentication', 'mcp')")] string conceptName,
        [Description("Graph traversal depth (1-3)")] int depth = 2,
        [Description("Max entities to include")] int maxEntities = 30)
    {
        // T-APP-001.8: Enforce documented parameter bounds
        depth = Math.Clamp(depth, 1, 3);
        maxEntities = Math.Clamp(maxEntities, 1, 100);
        return GraphTools.BuildContext(conceptName, depth, maxEntities, includeContent: false);
    }
}

[McpServerResourceType]
public class ConceptMapResources
{
    [McpServerResource(
        UriTemplate = "ui://maenifold/concept-map",
        Name = "concept_map_ui",
        MimeType = "text/html;profile=mcp-app")]
    [Description("Interactive concept map explorer UI")]
    public static string GetConceptMapUI()
    {
        var path = Path.Combine(Config.AssetsPath, "apps", "concept-map.html");
        if (!File.Exists(path))
        {
            throw new FileNotFoundException("Concept map app not found. Run 'update_assets' to install.", path);
        }

        return File.ReadAllText(path);
    }
}
