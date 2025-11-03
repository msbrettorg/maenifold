using ModelContextProtocol.Server;
using System.ComponentModel;
using Microsoft.Data.Sqlite;
using System.Text.Json;
using Maenifold.Models;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public class GraphTools
{


    private static string DbPath => Config.DatabasePath;

    static GraphTools()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
    }

    [McpServerTool, Description(@"Synchronizes [[WikiLink]] concepts from memory files into graph database for relationship mapping and discovery.
Select when AI needs to ensure graph accuracy, integrate new concepts, or refresh relationship data.
No parameters required - processes all memory files for comprehensive graph database synchronization.
Integrates with all memory tools for concept extraction, BuildContext for relationship verification, Visualize for graph display.
Returns synchronization status with concept counts, relationship updates, and database health confirmation.")]
    public static string Sync([Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(Sync).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(Sync)}";
            return File.ReadAllText(helpPath);
        }

        return ConceptSync.Sync();
    }

    [McpServerTool, Description(@"Traverses concept relationships in maenifold's graph database to discover connected knowledge networks.
Select when AI needs to understand concept relationships, explore knowledge clusters, or find related information.
Requires concept name with optional depth, entity limits, and content inclusion for comprehensive context building.
Connects to SearchMemories for concept discovery, ReadMemory for content access, Visualize for relationship mapping.
Returns related concepts with relationship types, file references, and connection strengths for knowledge exploration.")]
    public static BuildContextResult BuildContext(
            [Description("CONCEPT name to build context around (NOT a file!)")] string conceptName,
            [Description("How many hops in the CONCEPT GRAPH")] int depth = 2,
            [Description("Max entities to return")] int maxEntities = 20,
            [Description("Include full content")] bool includeContent = false)
    {
        // ISSUE-005: Validate depth parameter
        if (depth < 0)
        {
            throw new ArgumentException("depth must be >= 0", nameof(depth));
        }

        conceptName = MarkdownIO.NormalizeConcept(conceptName);

        using var conn = new SqliteConnection($"Data Source={DbPath}");
        conn.OpenReadOnly();


        var exists = conn.QuerySingle<bool>("SELECT COUNT(*) > 0 FROM concepts WHERE concept_name = @name", new { name = conceptName });

        var result = new BuildContextResult
        {
            ConceptName = conceptName,
            Depth = depth,
            DirectRelations = new(),
            ExpandedRelations = new()
        };

        if (!exists)
            return result;


        var directRelations = conn.Query<(string related, int count, string files)>(
                    @"SELECT
                CASE WHEN concept_a = @name THEN concept_b ELSE concept_a END as related,
                co_occurrence_count as count,
                source_files as files
            FROM concept_graph
            WHERE concept_a = @name OR concept_b = @name
            ORDER BY co_occurrence_count DESC
            LIMIT @maxEntities",
                    new { name = conceptName, maxEntities });

        foreach (var (related, count, files) in directRelations)
        {
            var fileList = JsonSerializer.Deserialize<List<string>>(files) ?? new();
            var relatedConcept = new RelatedConcept
            {
                Name = related,
                CoOccurrenceCount = count,
                Files = fileList.Take(3).ToList()
            };


            if (includeContent)
            {
                foreach (var filePath in relatedConcept.Files)
                {
                    try
                    {
                        var fullPath = MarkdownIO.UriToPath(filePath, Config.MemoryPath);
                        if (File.Exists(fullPath))
                        {
                            var (_, fileContent, _) = MarkdownIO.ReadMarkdown(fullPath);
                            // RTM: BUILDCONTEXT-PREVIEW-UX-001 - Sentence-aware content preview
                            var preview = CreateSmartPreview(fileContent, targetLength: 200, tolerance: 50);

                            relatedConcept.ContentPreview[filePath] = preview;
                        }
                    }
                    catch
                    {
                        // Silently skip files that can't be read
                    }
                }
            }

            result.DirectRelations.Add(relatedConcept);
        }

        if (depth > 1)
        {
            var expandedConcepts = new HashSet<string>();
            var visited = new HashSet<string> { conceptName };

            void TraverseRecursive(string currentConcept, int currentDepth)
            {
                if (currentDepth >= depth) return;

                var nextLevel = conn.Query<string>(
                                    @"SELECT DISTINCT
                        CASE WHEN concept_a = @name THEN concept_b ELSE concept_a END
                    FROM concept_graph
                    WHERE (concept_a = @name OR concept_b = @name)",
                                    new { name = currentConcept });

                foreach (var nextConcept in nextLevel)
                {
                    if (visited.Add(nextConcept))
                    {
                        expandedConcepts.Add(nextConcept);
                        TraverseRecursive(nextConcept, currentDepth + 1);
                    }
                }
            }

            foreach (var (related, _, _) in directRelations.Take(5))
            {
                TraverseRecursive(related, 1);
            }

            result.ExpandedRelations = expandedConcepts.Take(maxEntities).ToList();
        }

        return result;
    }

    [McpServerTool, Description(@"Generates Mermaid diagram representations of concept relationships from maenifold's graph database.
Select when AI needs visual understanding of knowledge connections, relationship mapping, or concept clustering analysis.
Requires concept name with optional depth limits and node count controls for focused visualization.
Integrates with BuildContext for relationship data, Sync for graph accuracy, SearchMemories for concept discovery.
Returns Mermaid diagram code ready for rendering, enables visual knowledge architecture understanding.")]
    public static string Visualize(
            [Description("Central concept")] string conceptName,
            [Description("Graph depth")] int depth = 2,
            [Description("Max nodes")] int maxNodes = 30,
            [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(Visualize).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(Visualize)}";
            return File.ReadAllText(helpPath);
        }

        return GraphAnalyzer.Visualize(conceptName, depth, maxNodes);
    }

    // RTM: BUILDCONTEXT-PREVIEW-UX-001 - Helper methods for sentence-aware content preview

    public static string CreateSmartPreview(string content, int targetLength = 200, int tolerance = 50)
    {
        if (content.Length <= targetLength)
            return content;

        int maxLength = targetLength + tolerance;
        string searchSpace = content.Substring(0, Math.Min(maxLength, content.Length));

        // Try sentence boundary (. ! ?) followed by space or newline
        int sentenceEnd = FindLastSentenceBoundary(searchSpace);
        if (sentenceEnd >= targetLength - tolerance)
            return content.Substring(0, sentenceEnd + 1).TrimEnd();

        // Fallback to paragraph boundary (double newline)
        int paragraphEnd = searchSpace.LastIndexOf("\n\n", StringComparison.Ordinal);
        if (paragraphEnd >= targetLength - tolerance)
            return content.Substring(0, paragraphEnd).TrimEnd() + "...";

        // Fallback to word boundary (space)
        int wordEnd = searchSpace.LastIndexOf(' ');
        if (wordEnd > 0)
            return content.Substring(0, wordEnd).TrimEnd() + "...";

        // Final fallback - hard truncate
        return string.Concat(content.AsSpan(0, targetLength), "...");
    }

    private static int FindLastSentenceBoundary(string text)
    {
        // Find last occurrence of . ! ? followed by space/newline/end
        var matches = new[] { ". ", ".\n", "! ", "!\n", "? ", "?\n" };
        int lastIndex = -1;

        foreach (var match in matches)
        {
            int index = text.LastIndexOf(match, StringComparison.Ordinal);
            if (index > lastIndex)
                lastIndex = index;
        }

        // Also check for sentence-ending punctuation at very end
        if (text.EndsWith('.') ||
            text.EndsWith('!') ||
            text.EndsWith('?'))
        {
            lastIndex = Math.Max(lastIndex, text.Length - 1);
        }

        return lastIndex;
    }
}
