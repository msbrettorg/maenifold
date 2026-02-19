using System.Globalization;
using Maenifold.Utils;

namespace Maenifold.Tools;

public partial class MemorySearchTools
{

    private static readonly HashSet<string> Stopwords = new HashSet<string>(new[]
    {
        "a","an","and","are","as","at","be","but","by","for","if","in","into","is","it",
        "no","not","of","on","or","such","that","the","their","then","there","these",
        "they","this","to","was","will","with","we","you","your","our","from"
    }, StringComparer.OrdinalIgnoreCase);


    private static string[] GetInformativeTerms(string query)
    {
        if (string.IsNullOrWhiteSpace(query)) return Array.Empty<string>();
        var terms = query.ToLowerInvariant()
            .Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Where(t => !Stopwords.Contains(t))
            .ToArray();
        return terms;
    }
    private static double CalculateSearchScore(string query, string content, Dictionary<string, object>? frontmatter)
    {
        var terms = GetInformativeTerms(query);
        if (terms.Length == 0) return 0;

        var contentLower = content.ToLowerInvariant();
        var titleLower = frontmatter?.ContainsKey("title") == true
            ? frontmatter["title"].ToString()!.ToLower(CultureInfo.InvariantCulture)
            : "";

        double score = 0;
        foreach (var term in terms)
        {

            score += contentLower.Split(term, StringSplitOptions.None).Length - 1;

            if (!string.IsNullOrEmpty(titleLower) && titleLower.Contains(term, StringComparison.Ordinal))
                score += 5;
        }

        return score;
    }

    private static string ExtractSnippet(string content, string query, int maxLength)
    {
        if (string.IsNullOrEmpty(content))
            return string.Empty;

        var terms = GetInformativeTerms(query);
        int firstTermIndex = -1;

        if (terms.Length > 0)
        {
            // Find position of first informative term
            foreach (var term in terms)
            {
                var idx = content.IndexOf(term, StringComparison.OrdinalIgnoreCase);
                if (idx >= 0 && (firstTermIndex == -1 || idx < firstTermIndex))
                    firstTermIndex = idx;
            }
        }

        if (firstTermIndex < 0)
            firstTermIndex = 0;

        // Extract snippet centered around the first term
        var start = Math.Max(0, firstTermIndex - 20);
        var length = Math.Min(maxLength, content.Length - start);
        var snippet = content.Substring(start, length).Replace("\r\n", " ").Replace("\n", " ").Trim();

        // Return the real content snippet without any placeholder ellipsis markers
        // This ensures search results always show actual file content, never placeholder text
        return snippet;
    }

    private static List<(string path, double score)> GetTextSearchResults(string query, string searchPath, int maxResults, string[]? tags)
    {
        var files = Directory.GetFiles(searchPath, "*.md", SearchOption.AllDirectories);
        var results = new List<(string path, double score)>();

        foreach (var file in files.Take(maxResults * 2))
        {
            try
            {
                var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(file);


                if (tags != null && tags.Length > 0)
                {
                    var fileTags = frontmatter?.ContainsKey("tags") == true
                        ? (frontmatter["tags"] as IEnumerable<object>)?.Select(t => t.ToString()).ToArray() ?? Array.Empty<string>()
                        : Array.Empty<string>();

                    if (!tags.All(t => fileTags.Contains(t, StringComparer.OrdinalIgnoreCase)))
                        continue;
                }

                var score = CalculateSearchScore(query, content, frontmatter);
                if (score > 0)
                {

                    var uri = MarkdownIO.PathToUri(file, Config.MemoryPath);
                    results.Add((uri, score));
                }
            }
            catch (Exception ex) when (ex is IOException or FormatException or InvalidOperationException) { /* Skip malformed files */ }
        }

        return results.OrderByDescending(r => r.score).Take(maxResults).ToList();
    }
}
