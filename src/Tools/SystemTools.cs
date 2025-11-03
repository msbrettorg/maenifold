using System.Globalization;
using ModelContextProtocol.Server;
using System.ComponentModel;
using Microsoft.Data.Sqlite;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public class SystemTools
{
    private static string MemoryPath => Config.MemoryPath;
    private static string DbPath => Config.DatabasePath;

    [McpServerTool, Description(@"Displays Maenifold system configuration settings and operational parameters for troubleshooting.
Select when AI needs system configuration verification, debugging support, or operational parameter analysis.
No parameters required - returns complete configuration state and system settings.
Integrates with all tools for configuration-dependent behavior, MemoryStatus for system health correlation.
Returns configuration settings with values, paths, and system state information for operational insight.")]
    public static string GetConfig([Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(GetConfig).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(GetConfig)}";
            return File.ReadAllText(helpPath);
        }

        return Config.GetConfigSummary();
    }

    [McpServerTool, Description(@"Provides Maenifold system statistics including file counts, graph metrics, and storage health monitoring.
Select when AI needs system overview, storage analysis, or health checks before major operations.
No parameters required - returns comprehensive system state and resource utilization metrics.
Integrates with ListMemories for structure analysis, RecentActivity for usage patterns, Sync for graph health.
Returns detailed statistics with file counts, concept metrics, and system health indicators.")]
    public static string MemoryStatus([Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(MemoryStatus).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(MemoryStatus)}";
            return File.ReadAllText(helpPath);
        }

        var stats = new System.Text.StringBuilder("# Memory System Status\n\n");


        var files = Directory.GetFiles(MemoryPath, "*.md", SearchOption.AllDirectories);
        var folders = Directory.GetDirectories(MemoryPath, "*", SearchOption.AllDirectories);
        stats.AppendLineInvariant($"### Files\n- Total files: {files.Length}");
        stats.AppendLineInvariant($"- Total folders: {folders.Length}");

        var totalSize = files.Sum(f => new FileInfo(f).Length);
        stats.AppendLineInvariant($"- Total size: {totalSize / 1024.0 / 1024.0:F2} MB");


        if (File.Exists(DbPath))
        {
            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenReadOnly();

            var conceptCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concepts");
            var relationCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_graph");
            var mentionCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_mentions");

            stats.AppendLine("\n### Graph Database");
            stats.AppendLineInvariant($"- Concepts: {conceptCount}");
            stats.AppendLineInvariant($"- Relations: {relationCount}");
            stats.AppendLineInvariant($"- Mentions: {mentionCount}");

            var dbInfo = new FileInfo(DbPath);
            stats.AppendLineInvariant($"- Database size: {dbInfo.Length / 1024.0 / 1024.0:F2} MB");
        }
        else
        {
            stats.AppendLine("\n### Graph Database\n- Not initialized (run sync first)");
        }

        return stats.ToString();
    }

    [McpServerTool, Description(@"Explores memory system folder structures with file counts and sizes for knowledge organization analysis.
Select when AI needs to understand knowledge organization, plan file placement, or analyze storage patterns.
Requires optional path parameter for targeted directory exploration, defaults to memory root structure.
Connects to WriteMemory for organization planning, MoveMemory for restructuring, SearchMemories for content discovery.
Returns hierarchical directory listing with file counts, sizes, and folder organization for structure understanding.")]
    public static string ListMemories(
        [Description("Directory path (relative to memory)")] string? path = null,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(ListMemories).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(ListMemories)}";
            return File.ReadAllText(helpPath);
        }

        var targetPath = string.IsNullOrEmpty(path)
            ? MemoryPath
            : Path.Combine(MemoryPath, path);

        if (!Directory.Exists(targetPath))
            return $"Directory not found: {path}";

        var output = new System.Text.StringBuilder($"# Directory: {path ?? "/"}\n\n");


        var dirs = Directory.GetDirectories(targetPath).OrderBy(d => d);
        if (dirs.Any())
        {
            output.AppendLine("### Folders");
            foreach (var dir in dirs)
            {
                var name = Path.GetFileName(dir);
                var fileCount = Directory.GetFiles(dir, "*.md").Length;
                output.AppendLineInvariant($"\uD83D\uDCC1 {name}/ ({fileCount} files)");
            }
            output.AppendLine();
        }


        var files = Directory.GetFiles(targetPath, "*.md").OrderBy(f => f);
        if (files.Any())
        {
            output.AppendLine("### Files");
            foreach (var file in files)
            {
                var name = Path.GetFileName(file);
                var info = new FileInfo(file);
                output.AppendLineInvariant($"\uD83D\uDCC4 {name} ({info.Length / 1024.0:F1} KB)");
            }
        }

        return output.ToString();
    }

    [McpServerTool, Description(@"Retrieves comprehensive tool documentation from Maenifold's help file system for detailed usage guidance.
Select when AI needs complete parameter documentation, usage examples, or troubleshooting information for any tool.
Requires tool name parameter to load specific help file from /src/assets/usage/tools/{toolname}.md.
Integrates with all Maenifold tools by providing detailed usage patterns, JSON examples, and troubleshooting guidance.
Returns complete tool manual with parameters, examples, common patterns, and troubleshooting guidance.")]
    public static string GetHelp(
        [Description("Tool name to get help for (e.g., 'WriteMemory', 'SearchMemories')")] string toolName)
    {
        var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName.ToLower(CultureInfo.InvariantCulture)}.md");

        if (!File.Exists(helpPath))
        {
            var availableTools = Directory.GetFiles(Path.Combine(Config.AssetsPath, "usage", "tools"), "*.md")
                .Select(f => Path.GetFileNameWithoutExtension(f))
                .OrderBy(n => n);

            return $"No help file found for tool: {toolName}\n\n" +
                   $"Available tools with help:\n" +
                   string.Join("\n", availableTools.Select(t => $"  - {t}"));
        }

        return File.ReadAllText(helpPath);
    }

    [McpServerTool, Description(@"Updates persistent assets from packaged assets on upgrades with dry-run capability.
Select when asset changes (workflows, documentation, models) need refreshing after Maenifold upgrades.
Supports dry-run mode to preview changes before applying, with detailed summary of modifications.
Integrates with asset initialization to provide explicit refresh mechanism after deployment.
Returns summary of added/updated files with error reporting if refresh encounters issues.")]
    public static string UpdateAssets(
        [Description("Preview changes without modifying files (default=true)")] bool dryRun = true,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(UpdateAssets).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(UpdateAssets)}";
            return File.ReadAllText(helpPath);
        }

        var result = AssetManager.UpdateAssets(dryRun);
        return result.ToString();
    }
}
