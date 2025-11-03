using ModelContextProtocol.Server;
using System.ComponentModel;
using System.Text;
using Maenifold.Utils;

namespace Maenifold.Tools;

public partial class MemoryTools
{
    [McpServerTool, Description(@"Retrieves existing knowledge files by URI or title with full content and metadata from Maenifold storage.
Select when AI needs to access specific stored knowledge, verify information, or examine file structure.
Requires memory:// URI or file title, optional checksum inclusion for edit safety validation.
Integrates with SearchMemories for discovery, EditMemory for modifications, MoveMemory for organization.
Returns formatted content with timestamps, location, checksum, and full markdown content ready for analysis.")]
    public static string ReadMemory(
        [Description("Memory FILE identifier (memory://uri points to a FILE, or title of a FILE)")] string identifier,
        [Description("Return checksum with content for safe editing")] bool includeChecksum = true,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(ReadMemory).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(ReadMemory)}";
            return File.ReadAllText(helpPath);
        }

        var path = identifier.StartsWithOrdinal("memory://") ? UriToPath(identifier) : FindFileByTitle(identifier);

        if (path == null || !File.Exists(path))
            return $"ERROR: Memory file not found: {identifier}";

        var (frontmatter, content, checksum) = MarkdownIO.ReadMarkdown(path);
        var uri = PathToUri(path);


        var title = frontmatter?.TryGetValue("title", out var titleValue) == true && !string.IsNullOrWhiteSpace(titleValue?.ToString())
            ? titleValue.ToString()
            : Path.GetFileNameWithoutExtension(path);

        var sb = new StringBuilder();
        sb.AppendLineInvariant($"# {title}");
        sb.AppendLineInvariant($"URI: {uri}");
        sb.AppendLineInvariant($"Location: {Path.GetRelativePath(BasePath, path)}");


        if (frontmatter?.ContainsKey("created") == true)
        {
            var createdLocal = TimeZoneConverter.ToLocalDisplay(frontmatter["created"]?.ToString());
            sb.AppendLineInvariant($"Created: {createdLocal}");
        }
        if (frontmatter?.ContainsKey("modified") == true)
        {
            var modifiedLocal = TimeZoneConverter.ToLocalDisplay(frontmatter["modified"]?.ToString());
            sb.AppendLineInvariant($"Modified: {modifiedLocal}");
        }

        if (includeChecksum)
            sb.AppendLineInvariant($"Checksum: {checksum}");

        sb.AppendLine("\n---\n");
        sb.Append(content);

        return sb.ToString();
    }

    [McpServerTool, Description(@"Modifies existing knowledge files with [[WikiLink]] preservation and checksum safety for graph integrity.
Select when AI needs to update, append, or restructure existing knowledge while maintaining connections.
Requires identifier, operation type, new content with [[concepts]], optional checksum and search patterns.
Connects to ReadMemory for current state, Sync for graph updates, SearchMemories for impact analysis.
Returns updated URI with new checksum, confirms successful modification and continued graph connectivity.")]
    public static string EditMemory(
        [Description("Memory FILE identifier")] string identifier,
        [Description("Operation: append, prepend, find_replace, replace_section")] string operation,
        [Description("Content to add/replace - REQUIRED: must contain at least one [[concept]]!")] string content,
        [Description("File checksum from last read (prevents stale edits)")] string? checksum = null,
        [Description("Text to find (for find_replace)")] string? findText = null,
        [Description("Section name (for replace_section)")] string? sectionName = null,
        [Description("Expected match count (for find_replace validation)")] int? expectedCount = null,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(EditMemory).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(EditMemory)}";
            return File.ReadAllText(helpPath);
        }

        var path = identifier.StartsWithOrdinal("memory://") ? UriToPath(identifier) : FindFileByTitle(identifier);

        if (path == null || !File.Exists(path))
            return $"ERROR: Memory file not found: {identifier}";

        var (frontmatter, existingContent, actualChecksum) = MarkdownIO.ReadMarkdown(path);

        if (checksum != null && checksum != actualChecksum)
            return $"ERROR: Checksum mismatch. Expected: {checksum}, Actual: {actualChecksum}";


        var contentConcepts = MarkdownIO.ExtractWikiLinks(content);
        if (contentConcepts.Count == 0)
        {
            var sb = new StringBuilder();
            sb.AppendLine("ERROR: Edited content must contain at least one [[concept]] in double brackets.");
            sb.AppendLine("Concepts are the building blocks of the knowledge graph—they create connections between files.");
            sb.AppendLine();

            // Extract existing concepts from the file to suggest in error message
            var existingConcepts = MarkdownIO.ExtractWikiLinks(existingContent);
            if (existingConcepts.Count > 0)
            {
                sb.AppendLine("Concepts already in this file that you could reference:");
                var suggestionCount = Math.Min(3, existingConcepts.Count);
                for (int i = 0; i < suggestionCount; i++)
                {
                    sb.AppendLineInvariant($"  • [[{existingConcepts[i]}]]");
                }
                sb.AppendLine();
            }

            sb.AppendLine("Examples of how to add concepts:");
            sb.AppendLine("  • \"This relates to [[machine-learning]] and [[neural-networks]]\"");
            sb.AppendLine("  • \"See also: [[data-processing]], [[feature-engineering]]\"");
            sb.AppendLine("  • \"The [[algorithm]] uses [[graph-theory]] concepts\"");

            return sb.ToString();
        }

        var newContent = operation.ToLowerInvariant() switch
        {
            "append" => existingContent + "\n\n" + content,
            "prepend" => content + "\n\n" + existingContent,
            "find_replace" => PerformFindReplace(existingContent, findText!, content, expectedCount),
            "replace_section" => ReplaceSection(existingContent, sectionName!, content),
            _ => throw new ArgumentException($"Unknown operation: {operation}")
        };


        var resultConcepts = MarkdownIO.ExtractWikiLinks(newContent);
        if (resultConcepts.Count == 0)
        {
            var sb = new StringBuilder();
            sb.AppendLine("ERROR: This edit would remove all [[concepts]] from the file.");
            sb.AppendLine("Every memory file must maintain at least one [[concept]] to stay connected to the knowledge graph.");
            sb.AppendLine();
            sb.AppendLine("To fix this, ensure your edited content includes at least one [[concept]].");
            sb.AppendLine("Examples:");
            sb.AppendLine("  • Add a reference: \"Related to [[my-topic]]\"");
            sb.AppendLine("  • Tag the section: \"[[process]]: Step-by-step instructions...\"");
            sb.AppendLine("  • Link concepts: \"Uses [[tool]] with [[feature]]\"");

            return sb.ToString();
        }

        frontmatter ??= new Dictionary<string, object>();
        if (!frontmatter.ContainsKey("type"))
            frontmatter["type"] = "memory";
        if (!frontmatter.ContainsKey("status"))
            frontmatter["status"] = "saved";


        frontmatter["modified"] = TimeZoneConverter.GetUtcNowIso();

        MarkdownIO.UpdateMarkdown(path, frontmatter, newContent, checksum);


        var newChecksum = MarkdownIO.GenerateChecksum(File.ReadAllText(path));
        return $"Updated memory FILE: {PathToUri(path)}\nNew checksum: {newChecksum}";
    }

    [McpServerTool, Description(@"Permanently removes knowledge files from Maenifold storage with confirmation safety mechanism.
Select when AI needs to clean up outdated, duplicate, or incorrect knowledge that impacts accuracy.
Requires file identifier and explicit confirm=true parameter to prevent accidental data loss.
Integrates with SearchMemories for cleanup discovery, ReadMemory for content verification before deletion.
Returns deletion confirmation with removed URI, enables cleanup of orphaned references.")]
    public static string DeleteMemory(
        [Description("Memory FILE identifier")] string identifier,
        [Description("Confirm deletion of the FILE")] bool confirm = false,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(DeleteMemory).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(DeleteMemory)}";
            return File.ReadAllText(helpPath);
        }

        if (!confirm)
            return "ERROR: Must set confirm=true to delete a memory file";

        var path = identifier.StartsWithOrdinal("memory://") ? UriToPath(identifier) : FindFileByTitle(identifier);

        if (path == null || !File.Exists(path))
            return $"ERROR: Memory file not found: {identifier}";

        var uri = PathToUri(path);
        File.Delete(path);

        return $"Deleted memory FILE: {uri}";
    }

    [McpServerTool, Description(@"Relocates and renames knowledge files while preserving [[WikiLinks]] and updating metadata timestamps.
Select when AI needs to reorganize knowledge structure, rename files, or move content between folders.
Requires source identifier and destination path, handles both simple renaming and folder restructuring.
Connects to ReadMemory for source verification, SearchMemories for organization planning, ListMemories for structure.
Returns movement confirmation showing old → new URIs, maintains all content and graph connections.")]
    public static string MoveMemory(
        [Description("Source FILE identifier")] string source,
        [Description("Destination FILE name or path")] string destination,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(MoveMemory).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(MoveMemory)}";
            return File.ReadAllText(helpPath);
        }

        if (string.IsNullOrWhiteSpace(destination))
            return "ERROR: Destination parameter is required and cannot be empty.";

        var sourcePath = source.StartsWithOrdinal("memory://") ? UriToPath(source) : FindFileByTitle(source);

        if (sourcePath == null || !File.Exists(sourcePath))
            return $"ERROR: Source memory file not found: {source}";

        var destRel = destination.Replace('/', Path.DirectorySeparatorChar);

        // Slugify the filename component for consistency with WriteMemory
        var destDir = Path.GetDirectoryName(destRel) ?? string.Empty;
        var destFile = Path.GetFileName(destRel);
        destFile = MarkdownIO.Slugify(Path.GetFileNameWithoutExtension(destFile));

        if (!destFile.EndsWith(".md", StringComparison.OrdinalIgnoreCase))
            destFile += ".md";

        destRel = string.IsNullOrEmpty(destDir) ? destFile : Path.Combine(destDir, destFile);
        var destPath = Path.Combine(BasePath, destRel);


        var baseFullPath = Path.GetFullPath(BasePath)
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar) + Path.DirectorySeparatorChar;
        var destFullPath = Path.GetFullPath(destPath);
        if (!destFullPath.StartsWith(baseFullPath, StringComparison.Ordinal))
            return "ERROR: Destination path escapes memory root";


        if (File.Exists(destFullPath))
            return $"ERROR: Destination file already exists: {destination}. Cannot overwrite without explicit confirmation.";


        var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(sourcePath);
        frontmatter ??= new Dictionary<string, object>();
        frontmatter["modified"] = TimeZoneConverter.GetUtcNowIso();

        Directory.CreateDirectory(Path.GetDirectoryName(destFullPath)!);


        MarkdownIO.WriteMarkdown(destFullPath, frontmatter, content);
        File.Delete(sourcePath);

        return $"Moved memory FILE: {PathToUri(sourcePath)} → {PathToUri(destFullPath)}";
    }
    [McpServerTool, Description(@"Analyzes knowledge files to identify [[WikiLink]] concepts for graph relationship discovery and validation.
Select when AI needs to understand file connectivity, validate graph integration, or analyze concept density.
Requires memory file identifier (URI or title) for targeted concept analysis.
Connects to ReadMemory for file access, BuildContext for relationship mapping, Sync for graph validation.
Returns concept list with count statistics, enables graph connectivity analysis and relationship discovery.")]
    public static string ExtractConceptsFromFile(
        [Description("Memory FILE identifier")] string identifier,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(ExtractConceptsFromFile).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(ExtractConceptsFromFile)}";
            return File.ReadAllText(helpPath);
        }

        var path = identifier.StartsWithOrdinal("memory://") ? UriToPath(identifier) : FindFileByTitle(identifier);

        if (path == null || !File.Exists(path))
            return $"ERROR: Memory file not found: {identifier}";

        var (_, content, _) = MarkdownIO.ReadMarkdown(path);
        var concepts = MarkdownIO.ExtractWikiLinks(content);

        if (concepts.Count == 0)
            return "No [[WikiLink]] concepts found in file";

        return $"Found {concepts.Count} CONCEPTS in file:\n" + string.Join("\n", concepts.Select(c => $"  • [[{c}]]"));
    }
}
