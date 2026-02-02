using ModelContextProtocol.Server;
using System.ComponentModel;
using System.Text;
using System.Text.Json;
using Maenifold.Utils;

namespace Maenifold.Tools;

public enum SearchMode
{
    Hybrid,
    Semantic,
    FullText
}

[McpServerToolType]
public partial class MemorySearchTools
{
    private static string BasePath => Config.MemoryPath;

    private static string QueryLacksInformativeTermsResponse()
    {
        return ToolResponse.WithHint(
            "ERROR: Query contains no informative keywords. Try adding specific terms (e.g. \"machine learning\", \"NLP\").",
            "SearchMemories",
            "Provide at least one non-stopword keyword");
    }

    [McpServerTool, Description(@"Discovers existing knowledge files through flexible search modes across maenifold's memory system.
Select when AI needs to find related information, verify existing knowledge, or explore concept connections.
Supports three search modes: Hybrid (default - best of both), Semantic (concept similarity), FullText (exact matching).
Returns ranked results with detailed scoring (text, semantic, fused) to help AI understand relevance.
Integrates with ReadMemory for content access, BuildContext for relationship exploration, WriteMemory for knowledge building.")]
    public static string SearchMemories(
            [Description("Search query - looks in FILE contents")] string query,
            [Description("Search mode: Hybrid (default), Semantic, or FullText")] SearchMode mode = SearchMode.Hybrid,
            [Description("Max FILES to return")] int pageSize = 10,
            [Description("Page number for results")] int page = 1,
            [Description("Filter by folder path")] string? folder = null,
            [Description("Filter by tags")] string[]? tags = null,
            [Description("Minimum similarity score threshold (0.0-1.0)")] double minScore = 0.0,
            [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(SearchMemories));

        var searchPath = folder != null ? Path.Combine(BasePath, folder) : BasePath;
        if (!Directory.Exists(searchPath))
            return "No memories found in specified folder";


        switch (mode)
        {
            case SearchMode.Semantic:
                return PerformSemanticSearch(query, searchPath, pageSize, page, tags, minScore);
            case SearchMode.FullText:
                return PerformFullTextSearch(query, searchPath, pageSize, page, tags, minScore);
            case SearchMode.Hybrid:
            default:
                return PerformHybridSearch(query, searchPath, pageSize, page, tags, minScore);
        }
    }

    private static string PerformHybridSearch(string query, string searchPath, int pageSize, int page, string[]? tags, double minScore)
    {

        if (GetInformativeTerms(query).Length == 0)
            return QueryLacksInformativeTermsResponse();


        var vectorResults = GetVectorSearchResults(query, searchPath, pageSize * 3, tags);


        var textResults = GetTextSearchResults(query, searchPath, pageSize * 3, tags);


        var mergedResults = ApplyReciprocalRankFusion(vectorResults, textResults, query, k: 60);


        var filteredResults = mergedResults.Where(r => r.fusedScore >= minScore).ToList();


        var paginatedResults = filteredResults
                    .Skip((page - 1) * pageSize)
                    .Take(pageSize)
                    .ToList();

        // T-CLI-JSON-001: RTM FR-8.2, FR-8.3 - Return JSON when flag is set
        if (OutputContext.IsJsonMode)
        {
            var results = paginatedResults.Select(r => new
            {
                title = r.title,
                path = r.path,
                fusedScore = r.fusedScore,
                textScore = r.textScore,
                semanticScore = r.semanticScore,
                snippet = r.snippet
            }).ToList();

            return JsonToolResponse.Ok(new
            {
                mode = "Hybrid",
                query = query,
                totalCount = filteredResults.Count,
                page = page,
                pageSize = pageSize,
                textResultCount = textResults.Count,
                semanticResultCount = vectorResults.Count,
                results = results
            }).ToJson();
        }

        var sb = new StringBuilder();
        sb.AppendLineInvariant($"\uD83D\uDD0D **Hybrid Search** (combining text + semantic similarity)");
        sb.AppendLineInvariant($"Found {filteredResults.Count} matches (page {page}):");
        sb.AppendLineInvariant($"Text results: {textResults.Count}, Semantic results: {vectorResults.Count}\n");

        foreach (var result in paginatedResults)
        {

            sb.AppendLineInvariant($"\uD83D\uDCC4 **{result.title}** ({result.path})");
            sb.AppendLineInvariant($"   \uD83D\uDCCA Scores - Fused: {result.fusedScore:F3} | Text: {result.textScore:F3} | Semantic: {result.semanticScore:F3}");
            sb.AppendLineInvariant($"   \uD83D\uDCDD {result.snippet}");
            sb.AppendLine();
        }

        return ToolResponse.WithNextSteps(
                    sb.ToString(),
                    "ReadMemory", "BuildContext"
                );
    }

    private static string PerformSemanticSearch(string query, string searchPath, int pageSize, int page, string[]? tags, double minScore)
    {
        if (GetInformativeTerms(query).Length == 0)
            return QueryLacksInformativeTermsResponse();

        var vectorResults = GetVectorSearchResults(query, searchPath, pageSize * 2, tags);


        var filteredVectorResults = vectorResults.Where(r => r.score >= minScore).ToList();


        // T-GRAPH-DECAY-001.1: RTM FR-7.5 - Apply decay weighting to semantic scores
        var resultsWithInfo = filteredVectorResults.Select(r =>
                {
                    var (title, snippet) = GetFileDisplayInfo(r.path, query);
                    var decayWeight = GetDecayWeightForFile(r.path);
                    var decayedScore = r.score * decayWeight;
                    return new { path = r.path, score = decayedScore, title, snippet };
                })
                .OrderByDescending(r => r.score)
                .ToList();


        var paginatedResults = resultsWithInfo
                    .Skip((page - 1) * pageSize)
                    .Take(pageSize)
                    .ToList();

        // T-CLI-JSON-001: RTM FR-8.2, FR-8.3 - Return JSON when flag is set
        if (OutputContext.IsJsonMode)
        {
            var results = paginatedResults.Select(r => new
            {
                title = r.title,
                path = r.path,
                score = r.score,
                snippet = r.snippet
            }).ToList();

            return JsonToolResponse.Ok(new
            {
                mode = "Semantic",
                query = query,
                totalCount = resultsWithInfo.Count,
                page = page,
                pageSize = pageSize,
                results = results
            }).ToJson();
        }

        var sb = new StringBuilder();
        sb.AppendLineInvariant($"\uD83E\uDDE0 **Semantic Search** (concept similarity)");
        sb.AppendLineInvariant($"Found {resultsWithInfo.Count} matches (page {page}):\n");

        foreach (var result in paginatedResults)
        {

            sb.AppendLineInvariant($"\uD83D\uDCC4 **{result.title}** ({result.path})");
            sb.AppendLineInvariant($"   \uD83C\uDFAF Semantic similarity: {result.score:F3}");
            sb.AppendLineInvariant($"   \uD83D\uDCDD {result.snippet}");
            sb.AppendLine();
        }

        return ToolResponse.WithNextSteps(
                    sb.ToString(),
                    "ReadMemory", "BuildContext", "FindSimilarConcepts"
                );
    }

    private static string PerformFullTextSearch(string query, string searchPath, int pageSize, int page, string[]? tags, double minScore)
    {
        if (GetInformativeTerms(query).Length == 0)
            return QueryLacksInformativeTermsResponse();

        var textResults = GetTextSearchResults(query, searchPath, pageSize * 2, tags);


        var normalizedAndFiltered = new List<(string path, double score)>();
        if (textResults.Any())
        {
            var maxScore = textResults.Max(r => r.score);
            if (maxScore > 0)
            {
                normalizedAndFiltered = textResults
                    .Select(r => (r.path, score: r.score / maxScore))
                    .Where(r => r.score >= minScore)
                    .ToList();
            }
        }


        // T-GRAPH-DECAY-001.1: RTM FR-7.5 - Apply decay weighting to full-text scores
        var resultsWithInfo = normalizedAndFiltered.Select(r =>
                {
                    var (title, snippet) = GetFileDisplayInfo(r.path, query);
                    var decayWeight = GetDecayWeightForFile(r.path);
                    var decayedScore = r.score * decayWeight;
                    return new { path = r.path, score = decayedScore, title, snippet };
                })
                .OrderByDescending(r => r.score)
                .ToList();


        var paginatedResults = resultsWithInfo
                    .Skip((page - 1) * pageSize)
                    .Take(pageSize)
                    .ToList();

        // T-CLI-JSON-001: RTM FR-8.2, FR-8.3 - Return JSON when flag is set
        if (OutputContext.IsJsonMode)
        {
            var results = paginatedResults.Select(r => new
            {
                title = r.title,
                path = r.path,
                score = r.score,
                snippet = r.snippet
            }).ToList();

            return JsonToolResponse.Ok(new
            {
                mode = "FullText",
                query = query,
                totalCount = resultsWithInfo.Count,
                page = page,
                pageSize = pageSize,
                results = results
            }).ToJson();
        }

        var sb = new StringBuilder();
        sb.AppendLineInvariant($"\uD83D\uDCDD **Full-Text Search** (keyword matching)");
        sb.AppendLineInvariant($"Found {resultsWithInfo.Count} matches (page {page}):\n");

        foreach (var result in paginatedResults)
        {

            sb.AppendLineInvariant($"\uD83D\uDCC4 **{result.title}** ({result.path})");
            sb.AppendLineInvariant($"   \uD83D\uDCCA Text relevance: {result.score:F2}");
            sb.AppendLineInvariant($"   \uD83D\uDCDD {result.snippet}");
            sb.AppendLine();
        }

        return ToolResponse.WithNextSteps(
                    sb.ToString(),
                    "ReadMemory", "BuildContext"
                );
    }
}
