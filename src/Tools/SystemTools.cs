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

    // RT-SEC-001: Path security validation to prevent traversal attacks
    private static string? ValidatePathSecurity(string folderPath)
    {
        // Block absolute paths
        if (Path.IsPathRooted(folderPath))
        {
            return "Absolute paths are not allowed. Use relative paths only.";
        }

        // Normalize path separators
        var normalizedPath = folderPath.Replace('\\', '/');

        // Check each segment for dot-only sequences (., .., ..., ....)
        var segments = normalizedPath.Split('/', StringSplitOptions.RemoveEmptyEntries);
        foreach (var segment in segments)
        {
            // RT-SEC-009 FIX: Block all dot-only sequences (., .., ..., ....)
            if (!string.IsNullOrEmpty(segment) && segment.All(c => c == '.'))
            {
                return "Path traversal components (dot sequences) are not allowed.";
            }
        }

        return null;
    }

    [McpServerTool, Description(@"Displays maenifold system configuration settings and operational parameters for troubleshooting.
Select when AI needs system configuration verification, debugging support, or operational parameter analysis.
No parameters required - returns complete configuration state and system settings.
Integrates with all tools for configuration-dependent behavior, MemoryStatus for system health correlation.
Returns configuration settings with values, paths, and system state information for operational insight.")]
    public static string GetConfig([Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(GetConfig));

        return Config.GetConfigSummary();
    }

    [McpServerTool, Description(@"Provides maenifold system statistics including file counts, graph metrics, and storage health monitoring.
Select when AI needs system overview, storage analysis, or health checks before major operations.
No parameters required - returns comprehensive system state and resource utilization metrics.
Integrates with ListMemories for structure analysis, RecentActivity for usage patterns, Sync for graph health.
Returns detailed statistics with file counts, concept metrics, and system health indicators.")]
    public static string MemoryStatus([Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(MemoryStatus));

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
        if (learn) return ToolHelpers.GetLearnContent(nameof(ListMemories));

        // RT-SEC-001: Validate path for security before filesystem operations
        if (!string.IsNullOrEmpty(path))
        {
            var validationError = ValidatePathSecurity(path);
            if (validationError != null)
            {
                return $"ERROR: Invalid path - {validationError}";
            }
        }

        var targetPath = string.IsNullOrEmpty(path)
            ? MemoryPath
            : Path.Combine(MemoryPath, path);

        // RT-SEC-001: Additional boundary check - ensure resolved path stays within MemoryPath
        var basePathFull = Path.GetFullPath(MemoryPath);
        var resolvedPath = Path.GetFullPath(targetPath);

        // Normalize paths for comparison by ensuring consistent trailing separators
        var normalizedBase = basePathFull.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        var normalizedResolved = resolvedPath.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);

        // Check if resolved path is base path itself OR is a subdirectory of base path
        if (!normalizedResolved.Equals(normalizedBase, StringComparison.Ordinal) &&
            !normalizedResolved.StartsWith(normalizedBase + Path.DirectorySeparatorChar, StringComparison.Ordinal))
        {
            return "ERROR: Invalid path - Directory traversal detected";
        }

        if (!Directory.Exists(targetPath))
            return $"Directory not found: {path}";

        var output = new System.Text.StringBuilder($"# Directory: {path ?? "/"}\n\n");

        // T-GRAPH-DECAY-003.1: RTM FR-7.7 - Load last_accessed timestamps from database
        var lastAccessedCache = LoadLastAccessedTimestamps();

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

                // T-GRAPH-DECAY-003.1: RTM FR-7.7 - Display decay metadata for each file
                var decayMetadata = GetDecayMetadataForFile(file, lastAccessedCache);
                output.AppendLineInvariant($"   created: {decayMetadata.created}");
                output.AppendLineInvariant($"   last_accessed: {decayMetadata.lastAccessed}");
                output.AppendLineInvariant($"   decay_weight: {decayMetadata.decayWeight:F2}");
            }
        }

        return output.ToString();
    }

    // T-GRAPH-DECAY-003.1: RTM FR-7.7 - Load last_accessed timestamps from file_content table
    private static Dictionary<string, DateTime?> LoadLastAccessedTimestamps()
    {
        var cache = new Dictionary<string, DateTime?>(StringComparer.OrdinalIgnoreCase);

        if (!File.Exists(DbPath))
            return cache;

        try
        {
            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenReadOnly();

            using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT file_path, last_accessed FROM file_content WHERE last_accessed IS NOT NULL";

            using var reader = cmd.ExecuteReader();
            while (reader.Read())
            {
                var filePath = reader.GetString(0);
                var lastAccessedStr = reader.IsDBNull(1) ? null : reader.GetString(1);

                if (!string.IsNullOrEmpty(lastAccessedStr) && DateTime.TryParse(lastAccessedStr, out var lastAccessed))
                {
                    cache[filePath] = lastAccessed;
                }
            }
        }
        catch
        {
            // Non-critical: if database access fails, continue without last_accessed data
        }

        return cache;
    }

    // T-GRAPH-DECAY-003.1, T-GRAPH-DECAY-003.2: RTM FR-7.7, NFR-7.7.1 - Get decay metadata for a file
    private static (string created, string lastAccessed, double decayWeight) GetDecayMetadataForFile(
        string filePath, Dictionary<string, DateTime?> lastAccessedCache)
    {
        DateTime createdDate = DateTime.UtcNow;
        DateTime? lastAccessedDate = null;

        // Read frontmatter to get created timestamp
        try
        {
            var (frontmatter, _, _) = MarkdownIO.ReadMarkdown(filePath);
            if (frontmatter != null)
            {
                // Try to get created date from frontmatter
                if (frontmatter.TryGetValue("created", out var createdValue) &&
                    createdValue != null &&
                    DateTime.TryParse(createdValue.ToString(), out var parsedCreated))
                {
                    createdDate = parsedCreated;
                }

                // Try to get last_accessed from frontmatter as fallback
                if (frontmatter.TryGetValue("last_accessed", out var lastAccessedValue) &&
                    lastAccessedValue != null &&
                    DateTime.TryParse(lastAccessedValue.ToString(), out var parsedLastAccessed))
                {
                    lastAccessedDate = parsedLastAccessed;
                }
            }
        }
        catch
        {
            // If frontmatter read fails, use file creation time as fallback
            createdDate = File.GetCreationTimeUtc(filePath);
        }

        // T-GRAPH-DECAY-003.1: RTM FR-7.7 - Get last_accessed from database cache (memory:// URI format)
        var memoryUri = MarkdownIO.PathToUri(filePath, MemoryPath);
        if (lastAccessedCache.TryGetValue(memoryUri, out var dbLastAccessed) && dbLastAccessed.HasValue)
        {
            lastAccessedDate = dbLastAccessed.Value;
        }

        // Fallback: if no last_accessed, use created or modified date
        var effectiveLastAccessed = lastAccessedDate ?? createdDate;

        // T-GRAPH-DECAY-003.2: RTM NFR-7.7.1 - Calculate decay weight using tier-based grace periods
        var decayWeight = DecayCalculator.GetMemoryDecayWeight(filePath, createdDate, lastAccessedDate);

        // Format dates for display (date only, no time)
        var createdStr = createdDate.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);
        var lastAccessedStr = effectiveLastAccessed.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);

        return (createdStr, lastAccessedStr, decayWeight);
    }

    [McpServerTool, Description(@"Retrieves comprehensive tool documentation from maenifold's help file system for detailed usage guidance.
Select when AI needs complete parameter documentation, usage examples, or troubleshooting information for any tool.
Requires tool name parameter to load specific help file from /src/assets/usage/tools/{toolname}.md.
Integrates with all maenifold tools by providing detailed usage patterns, JSON examples, and troubleshooting guidance.
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
Select when asset changes (workflows, documentation, models) need refreshing after maenifold upgrades.
Supports dry-run mode to preview changes before applying, with detailed summary of modifications.
Integrates with asset initialization to provide explicit refresh mechanism after deployment.
Returns summary of added/updated files with error reporting if refresh encounters issues.")]
    public static string UpdateAssets(
        [Description("Preview changes without modifying files (default=true)")] bool dryRun = true,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(UpdateAssets));

        var result = AssetManager.UpdateAssets(dryRun);
        return result.ToString();
    }
}
