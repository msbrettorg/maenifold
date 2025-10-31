using ModelContextProtocol.Server;
using System.ComponentModel;
using System.Text.RegularExpressions;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public partial class MemoryTools
{
    [McpServerTool, Description(@"Creates new knowledge files with [[WikiLinks]] that automatically integrate into Ma Core's graph database.
Select when AI needs to persist new learning, research findings, or structured knowledge for future retrieval.
Requires title, content with [[concepts]], optional folder organization and tag categorization.
Connects to SearchMemories for discovery, Sync for graph updates, BuildContext for relationship mapping.
Returns memory:// URI for future reference, checksum for safe editing, confirms graph integration.")]
    public static string WriteMemory(
        [Description("Title for this MEMORY FILE")] string title,
        [Description("Content with [[Concept Names]] in brackets - REQUIRED: at least one [[concept]]!")] string content,
        [Description("Optional folder path for organizing FILES")] string? folder = null,
        [Description("Optional tags for categorizing this FILE")] string[]? tags = null)
    {

        title = SanitizeUserInput(title);


        var concepts = MarkdownIO.ExtractWikiLinks(content);
        if (concepts.Count == 0)
        {
            return "ERROR: Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.\n" +
                   "Example: 'Learning about [[Machine Learning]] and [[Data Science]]'\n" +
                   "This ensures your note is connected to the knowledge graph and not orphaned.";
        }


        if (!string.IsNullOrEmpty(folder))
        {
            var validationError = ValidatePathSecurity(folder);
            if (validationError != null)
            {
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
        Directory.CreateDirectory(folderPath);
        MarkdownIO.WriteMarkdown(filePath, frontmatter, fullContent);


        var checksum = MarkdownIO.GenerateChecksum(File.ReadAllText(filePath));
        var uri = PathToUri(filePath);

        return ToolResponse.WithHint(
                    $"Created memory FILE: {uri}\nChecksum: {checksum}",
                    "WriteMemory",
                    "Use ReadMemory with this URI to access content"
                );
    }


}