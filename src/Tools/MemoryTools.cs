using ModelContextProtocol;
using ModelContextProtocol.Server;
using System.ComponentModel;
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;

namespace Maenifold.Tools;

[McpServerToolType]
public class MemoryTools
{
    private static string BasePath => Config.MemoryPath;

    static MemoryTools()
    {
        Config.EnsureDirectories();
    }

    private static string PathToUri(string path) => MarkdownIO.PathToUri(path, BasePath);

    private static string UriToPath(string uri) => MarkdownIO.UriToPath(uri, BasePath);

    // T-GRAPH-DECAY-002.1: RTM NFR-7.6.1 - Update last_accessed timestamp on explicit read
    private static void UpdateLastAccessed(string memoryUri)
    {
        try
        {
            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.Open();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "UPDATE file_content SET last_accessed = @timestamp WHERE file_path = @path";
            cmd.Parameters.AddWithValue("@timestamp", TimeZoneConverter.GetUtcNowIso());
            cmd.Parameters.AddWithValue("@path", memoryUri);
            cmd.ExecuteNonQuery();
        }
        catch (Exception ex) when (ex is not OutOfMemoryException and not StackOverflowException)
        {
            // Non-critical: if database update fails, continue without updating timestamp
            // The file read itself still succeeds
        }
    }

    private static string? FindFileByTitle(string title)
    {
        if (string.IsNullOrWhiteSpace(title))
            return null;

        var normalized = title.Replace('\\', '/').Trim('/');
        if (normalized.Length == 0)
            return null;


        var relativePath = normalized.Replace('/', Path.DirectorySeparatorChar);
        var directPath = Path.Combine(BasePath, relativePath);

        if (File.Exists(directPath))
            return directPath;

        if (!Path.HasExtension(relativePath))
        {
            var mdPath = directPath + ".md";
            if (File.Exists(mdPath))
                return mdPath;
        }
        else if (!directPath.EndsWith(".md", StringComparison.OrdinalIgnoreCase))
        {
            var withMdExtension = Path.Combine(BasePath, Path.ChangeExtension(relativePath, ".md"));
            if (File.Exists(withMdExtension))
                return withMdExtension;
        }


        var files = Directory.GetFiles(BasePath, "*.md", SearchOption.AllDirectories);
        var slug = MarkdownIO.Slugify(title);

        return files.FirstOrDefault(f =>
            Path.GetFileNameWithoutExtension(f).Equals(slug, StringComparison.OrdinalIgnoreCase));
    }

    /// <summary>
    /// Performs simple string replacement using String.Replace().
    ///
    /// WARNING: This creates nested WikiLinks if replacing text INSIDE existing [[brackets]].
    /// Example: [[machine learning]] with findText="machine learning" and replaceText="[[ML]]"
    /// becomes [[[[ML]]]] (nested brackets).
    ///
    /// To replace entire WikiLinks, include brackets in findText: "[[machine learning]]" → "[[ML]]".
    /// This is Ma Protocol-compliant: simple behavior, no "smart" escaping logic.
    /// </summary>
    private static string PerformFindReplace(string content, string findText, string replaceText, int? expectedCount)
    {
        var matches = content.Split(findText).Length - 1;
        if (expectedCount.HasValue && matches != expectedCount.Value)
            throw new McpException($"Expected {expectedCount.Value} matches but found {matches}. Find text: '{findText}'");

        return content.Replace(findText, replaceText);
    }

    private static string ReplaceSection(string content, string sectionName, string newContent)
    {
        // Match: section heading + all content until next heading or end of file
        // Pattern breakdown:
        // (^|\n) - Start of line (captured as $1 for preserving newline)
        // (#+\s*{sectionName}[^\n]*\n) - Heading with section name (captured as $2)
        // (.*?) - Section content (captured as $3, to be replaced)
        // (?=\n#|\z) - Lookahead: next heading or end of file
        var pattern = $@"(^|\n)(#+\s*{Regex.Escape(sectionName)}[^\n]*\n)(.*?)(?=\n#|\z)";

        var match = Regex.Match(content, pattern, RegexOptions.Multiline | RegexOptions.Singleline);

        if (!match.Success)
            throw new InvalidOperationException($"Section not found: '{sectionName}'. Cannot replace a section that doesn't exist.");

        // Replace: preserve leading newline + heading + new content (old section content is discarded)
        var replacement = $"$1$2{newContent}";
        return Regex.Replace(
            content,
            pattern,
            replacement,
            RegexOptions.Multiline | RegexOptions.Singleline);
    }

    private static string? ValidatePathSecurity(string folderPath)
    {

        if (Path.IsPathRooted(folderPath))
        {
            return "Absolute paths are not allowed. Use relative paths only.";
        }


        var normalizedPath = folderPath.Replace('\\', '/');


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

    private static string SanitizeUserInput(string input)
    {
        if (string.IsNullOrWhiteSpace(input))
            return "Untitled";


        if (input.Length > 255)
            input = input.Substring(0, 255);

        // SEC-002: URL-decode input first to prevent encoded path traversal bypass
        // This handles %2e%2e%2f (../) and other encoded variants
        input = Uri.UnescapeDataString(input);

        input = Regex.Replace(input, @"<[^>]*>", "", RegexOptions.IgnoreCase | RegexOptions.Compiled);


        input = Regex.Replace(input, @"[:|>\-\[\]{}#&*!@`']", "", RegexOptions.Compiled);


        var dangerousChars = new char[] { '$', '(', ')', ';', '&', '|', '`', '\\', '<', '>', '"' };
        foreach (var ch in dangerousChars)
        {
            input = input.Replace(ch.ToString(), "");
        }


        input = input.Replace("..", "").Replace("/", "").Replace("\\", "");


        input = Regex.Replace(input, @"[\x00-\x1F\x7F-\x9F]", "", RegexOptions.Compiled);


        input = Regex.Replace(input, @"\s+", " ", RegexOptions.Compiled).Trim();


        if (string.IsNullOrWhiteSpace(input))
            return "Untitled";

        return input;
    }

    [McpServerTool, Description(@"Creates new knowledge files with [[WikiLinks]] that automatically integrate into maenifold's graph database.
Select when AI needs to persist new learning, research findings, or structured knowledge for future retrieval.
Requires title, content with [[concepts]], optional folder organization and tag categorization.
Connects to SearchMemories for discovery, Sync for graph updates, BuildContext for relationship mapping.
Returns memory:// URI for future reference, checksum for safe editing, confirms graph integration.")]
    public static string WriteMemory(
        [Description("Title for this MEMORY FILE")] string title,
        [Description("Content with [[Concept Names]] in brackets - REQUIRED: at least one [[concept]]!")] string content,
        [Description("Optional folder path for organizing FILES")] string? folder = null,
        [Description("Optional tags for categorizing this FILE")] string[]? tags = null,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(WriteMemory));

        title = SanitizeUserInput(title);


        var concepts = MarkdownIO.ExtractWikiLinks(content);
        var (blocked, reasons) = WikiLinkFilter.CheckFilter(concepts);
        if (blocked.Count > 0)
        {
            var sb = new StringBuilder();
            sb.AppendLine("ERROR: Content contains filtered WikiLinks that are excluded from the knowledge graph:");
            foreach (var concept in blocked)
                sb.AppendLine(CultureInfo.InvariantCulture, $"  - [[{concept}]]: {reasons[concept]}");
            sb.AppendLine();
            sb.AppendLine("Remove these WikiLinks from your content and retry.");
            if (OutputContext.IsJsonMode)
                return JsonToolResponse.Fail("FILTERED_WIKILINKS", sb.ToString()).ToJson();
            return sb.ToString();
        }
        if (concepts.Count == 0)
        {
            // T-CLI-JSON-001: RTM FR-8.4 - Structured error
            if (OutputContext.IsJsonMode)
            {
                return JsonToolResponse.Fail("VALIDATION_ERROR",
                    "Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.").ToJson();
            }
            return "ERROR: Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.\n" +
                   "Example: 'Learning about [[Machine Learning]] and [[Data Science]]'\n" +
                   "This ensures your note is connected to the knowledge graph and not orphaned.";
        }


        if (!string.IsNullOrEmpty(folder))
        {
            var validationError = ValidatePathSecurity(folder);
            if (validationError != null)
            {
                // T-CLI-JSON-001: RTM FR-8.4 - Structured error
                if (OutputContext.IsJsonMode)
                {
                    return JsonToolResponse.Fail("PATH_VALIDATION_ERROR", validationError).ToJson();
                }
                return $"ERROR: Invalid folder path - {validationError}";
            }
        }

        var fileName = MarkdownIO.Slugify(title) + ".md";
        var folderPath = string.IsNullOrEmpty(folder) ? BasePath : Path.Combine(BasePath, folder);
        var filePath = Path.Combine(folderPath, fileName);

        var permalink = MarkdownIO.Slugify(title);
        var now = TimeZoneConverter.GetUtcNowIso();
        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = title,
            ["permalink"] = permalink,
            ["type"] = "memory",
            ["status"] = "saved",
            ["created"] = now,
            ["modified"] = now
        };

        if (tags != null && tags.Length > 0)
            frontmatter["tags"] = tags;


        // Embeddings should be persisted in the database only.
        // Do not generate or write any embedding_* fields to frontmatter.


        // BUG-DUP-H1-001: Prevent duplicate H1 headings
        var trimmedContent = content.TrimStart();
        var hasH1 = trimmedContent.StartsWith("# ", StringComparison.Ordinal);
        var fullContent = hasH1
            ? content  // Content already has H1, use as-is
            : $"# {title}\n\n{content}";  // No H1, prepend title

        // T-CLI-JSON-001: SEC-EDGE-002 - Catch PathTooLongException to prevent info leakage
        try
        {
            Directory.CreateDirectory(folderPath);
            MarkdownIO.WriteMarkdown(filePath, frontmatter, fullContent);
        }
        catch (PathTooLongException)
        {
            if (OutputContext.IsJsonMode)
                return JsonToolResponse.Fail("PATH_TOO_LONG", "Title exceeds maximum length - reduce to under 200 characters").ToJson();
            return "ERROR: Title too long - reduce to under 200 characters";
        }

        string checksum;
        try
        {
            checksum = MarkdownIO.GenerateChecksum(File.ReadAllText(filePath));
        }
        catch (PathTooLongException)
        {
            if (OutputContext.IsJsonMode)
                return JsonToolResponse.Fail("PATH_TOO_LONG", "Title exceeds maximum length - reduce to under 200 characters").ToJson();
            return "ERROR: Title too long - reduce to under 200 characters";
        }
        var uri = PathToUri(filePath);

        // T-CLI-JSON-001: RTM FR-8.2, FR-8.3 - Return JSON when flag is set
        if (OutputContext.IsJsonMode)
        {
            return JsonToolResponse.Ok(new
            {
                uri = uri,
                checksum = checksum,
                title = title,
                folder = folder
            }).ToJson();
        }

        return ToolResponse.WithHint(
                    $"Created memory FILE: {uri}\nChecksum: {checksum}",
                    "WriteMemory",
                    "Use ReadMemory with this URI to access content"
                );
    }

    [McpServerTool, Description(@"Retrieves existing knowledge files by URI or title with full content and metadata from maenifold storage.
Select when AI needs to access specific stored knowledge, verify information, or examine file structure.
Requires memory:// URI or file title, optional checksum inclusion for edit safety validation.
Integrates with SearchMemories for discovery, EditMemory for modifications, MoveMemory for organization.
Returns formatted content with timestamps, location, checksum, and full markdown content ready for analysis.")]
    public static string ReadMemory(
        [Description("Memory FILE identifier (memory://uri points to a FILE, or title of a FILE)")] string identifier,
        [Description("Return checksum with content for safe editing")] bool includeChecksum = true,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(ReadMemory));

        var path = identifier.StartsWithOrdinal("memory://") ? UriToPath(identifier) : FindFileByTitle(identifier);

        if (path == null || !File.Exists(path))
        {
            // T-CLI-JSON-001: RTM FR-8.4 - Structured error for not found
            if (OutputContext.IsJsonMode)
            {
                return JsonToolResponse.Fail("NOT_FOUND", $"Memory file not found: {identifier}").ToJson();
            }
            return $"ERROR: Memory file not found: {identifier}";
        }

        var (frontmatter, content, checksum) = MarkdownIO.ReadMarkdown(path);
        var uri = PathToUri(path);

        // T-GRAPH-DECAY-002.1: RTM NFR-7.6.1 - Update last_accessed on explicit read (intentional access)
        UpdateLastAccessed(uri);

        var title = frontmatter?.TryGetValue("title", out var titleValue) == true && !string.IsNullOrWhiteSpace(titleValue?.ToString())
            ? titleValue.ToString()
            : Path.GetFileNameWithoutExtension(path);

        // T-CLI-JSON-001: RTM FR-8.2, FR-8.3 - Return JSON when flag is set
        if (OutputContext.IsJsonMode)
        {
            return JsonToolResponse.Ok(new
            {
                uri = uri,
                title = title,
                location = Path.GetRelativePath(BasePath, path),
                created = frontmatter?.ContainsKey("created") == true ? frontmatter["created"]?.ToString() : null,
                modified = frontmatter?.ContainsKey("modified") == true ? frontmatter["modified"]?.ToString() : null,
                checksum = includeChecksum ? checksum : null,
                content = content
            }).ToJson();
        }

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
Returns updated URI with new checksum, confirms successful modification and continued graph connectivity.

WARNING: find_replace uses simple string replacement. Replacing text INSIDE existing [[WikiLinks]] creates nested brackets.
Example: [[machine learning]] with findText='machine learning' and content='[[ML]]' becomes [[[[ML]]]].
To avoid this, use findText='[[machine learning]]' to replace the entire WikiLink instead.")]
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
        if (learn) return ToolHelpers.GetLearnContent(nameof(EditMemory));

        var path = identifier.StartsWithOrdinal("memory://") ? UriToPath(identifier) : FindFileByTitle(identifier);

        if (path == null || !File.Exists(path))
            return $"ERROR: Memory file not found: {identifier}";

        var (frontmatter, existingContent, actualChecksum) = MarkdownIO.ReadMarkdown(path);

        if (checksum != null && checksum != actualChecksum)
            return $"ERROR: Checksum mismatch. Expected: {checksum}, Actual: {actualChecksum}";


        var contentConcepts = MarkdownIO.ExtractWikiLinks(content);
        var (blocked, reasons) = WikiLinkFilter.CheckFilter(contentConcepts);
        if (blocked.Count > 0)
        {
            var sb = new StringBuilder();
            sb.AppendLine("ERROR: Content contains filtered WikiLinks that are excluded from the knowledge graph:");
            foreach (var concept in blocked)
                sb.AppendLine(CultureInfo.InvariantCulture, $"  - [[{concept}]]: {reasons[concept]}");
            sb.AppendLine();
            sb.AppendLine("Remove these WikiLinks from your content and retry.");
            if (OutputContext.IsJsonMode)
                return JsonToolResponse.Fail("FILTERED_WIKILINKS", sb.ToString()).ToJson();
            return sb.ToString();
        }
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
            _ => throw new McpException($"Unknown operation: {operation}")
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

        // T-CLI-JSON-001: SEC-EDGE-002 - Catch PathTooLongException to prevent info leakage
        try
        {
            MarkdownIO.UpdateMarkdown(path, frontmatter, newContent, checksum);
        }
        catch (PathTooLongException)
        {
            if (OutputContext.IsJsonMode)
                return JsonToolResponse.Fail("PATH_TOO_LONG", "File path exceeds maximum length").ToJson();
            return "ERROR: File path too long";
        }

        string newChecksum;
        try
        {
            newChecksum = MarkdownIO.GenerateChecksum(File.ReadAllText(path));
        }
        catch (PathTooLongException)
        {
            if (OutputContext.IsJsonMode)
                return JsonToolResponse.Fail("PATH_TOO_LONG", "File path exceeds maximum length").ToJson();
            return "ERROR: File path too long";
        }
        return $"Updated memory FILE: {PathToUri(path)}\nNew checksum: {newChecksum}";
    }

    [McpServerTool, Description(@"Permanently removes knowledge files from maenifold storage with confirmation safety mechanism.
Select when AI needs to clean up outdated, duplicate, or incorrect knowledge that impacts accuracy.
Requires file identifier and explicit confirm=true parameter to prevent accidental data loss.
Integrates with SearchMemories for cleanup discovery, ReadMemory for content verification before deletion.
Returns deletion confirmation with removed URI, enables cleanup of orphaned references.")]
    public static string DeleteMemory(
        [Description("Memory FILE identifier")] string identifier,
        [Description("Confirm deletion of the FILE")] bool confirm = false,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(DeleteMemory));

        if (!confirm)
        {
            // T-CLI-JSON-001: RTM FR-8.4 - Structured error for confirmation required
            if (OutputContext.IsJsonMode)
            {
                return JsonToolResponse.Fail("CONFIRMATION_REQUIRED", "Must set confirm=true to delete a memory file").ToJson();
            }
            return "ERROR: Must set confirm=true to delete a memory file";
        }

        var path = identifier.StartsWithOrdinal("memory://") ? UriToPath(identifier) : FindFileByTitle(identifier);

        if (path == null || !File.Exists(path))
        {
            // T-CLI-JSON-001: RTM FR-8.4 - Structured error for not found
            if (OutputContext.IsJsonMode)
            {
                return JsonToolResponse.Fail("NOT_FOUND", $"Memory file not found: {identifier}").ToJson();
            }
            return $"ERROR: Memory file not found: {identifier}";
        }

        var uri = PathToUri(path);
        File.Delete(path);

        // T-CLI-JSON-001: RTM FR-8.2, FR-8.3 - Return JSON when flag is set
        if (OutputContext.IsJsonMode)
        {
            return JsonToolResponse.Ok(new { uri = uri, deleted = true }).ToJson();
        }

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
        if (learn) return ToolHelpers.GetLearnContent(nameof(MoveMemory));

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

        // T-CLI-JSON-001: SEC-EDGE-002 - Catch PathTooLongException to prevent info leakage
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(destFullPath)!);
            MarkdownIO.WriteMarkdown(destFullPath, frontmatter, content);
            File.Delete(sourcePath);
        }
        catch (PathTooLongException)
        {
            if (OutputContext.IsJsonMode)
                return JsonToolResponse.Fail("PATH_TOO_LONG", "Destination path exceeds maximum length - use a shorter filename").ToJson();
            return "ERROR: Destination path too long - use a shorter filename";
        }

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
        if (learn) return ToolHelpers.GetLearnContent(nameof(ExtractConceptsFromFile));

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
