namespace Maenifold.Utils;

public static class MarkdownIO
{

    public static (Dictionary<string, object>? frontmatter, string content, string checksum) ReadMarkdown(string path)
            => MarkdownReader.ReadMarkdown(path);

    public static (Dictionary<string, object>? frontmatter, string content, string checksum) ReadSession(string thinkingType, string sessionId)
            => MarkdownReader.ReadSession(thinkingType, sessionId);

    public static List<(string heading, string content)> ExtractH2Sections(string markdown)
            => MarkdownReader.ExtractH2Sections(markdown);

    public static List<string> ExtractWikiLinks(string content)
            => MarkdownReader.ExtractWikiLinks(content);

    public static int CountConceptOccurrences(string content, string concept)
            => MarkdownReader.CountConceptOccurrences(content, concept);

    public static (IReadOnlyList<(string concept, int sectionIndex)> concepts, bool usedFallback) ExtractWikiLinksWithSections(string content)
            => MarkdownReader.ExtractWikiLinksWithSections(content);


    public static void WriteMarkdown(string path, Dictionary<string, object>? frontmatter, string content)
            => MarkdownWriter.WriteMarkdown(path, frontmatter, content);

    public static void UpdateMarkdown(string path, Dictionary<string, object>? frontmatter, string content, string? expectedChecksum)
            => MarkdownWriter.UpdateMarkdown(path, frontmatter, content, expectedChecksum);

    public static string GetSessionPath(string thinkingType, string sessionId)
            => MarkdownWriter.GetSessionPath(thinkingType, sessionId);

    public static bool SessionExists(string thinkingType, string sessionId)
            => MarkdownWriter.SessionExists(thinkingType, sessionId);

    public static void CreateSession(string thinkingType, string sessionId, Dictionary<string, object> frontmatter, string content)
            => MarkdownWriter.CreateSession(thinkingType, sessionId, frontmatter, content);

    public static void UpdateSession(string thinkingType, string sessionId, Dictionary<string, object> frontmatter, string content)
            => MarkdownWriter.UpdateSession(thinkingType, sessionId, frontmatter, content);

    public static void AppendToSession(string thinkingType, string sessionId, string heading, string content)
            => MarkdownWriter.AppendToSession(thinkingType, sessionId, heading, content);

    public static void AppendH2Section(string path, string heading, string content)
            => MarkdownWriter.AppendH2Section(path, heading, content);

    public static void CreateMarkdownWithH1(string path, string title, string initialContent = "")
            => MarkdownWriter.CreateMarkdownWithH1(path, title, initialContent);

    public static string PathToUri(string path, string basePath)
            => MarkdownWriter.PathToUri(path, basePath);

    public static string UriToPath(string uri, string basePath)
            => MarkdownWriter.UriToPath(uri, basePath);

    public static string NormalizeConcept(string concept)
            => MarkdownWriter.NormalizeConcept(concept);

    public static string Slugify(string text)
            => MarkdownWriter.Slugify(text);

    public static string GenerateChecksum(string content)
            => MarkdownWriter.GenerateChecksum(content);
}
