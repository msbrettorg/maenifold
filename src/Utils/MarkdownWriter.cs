using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using YamlDotNet.Serialization;

namespace Maenifold.Utils;

public static class MarkdownWriter
{
    private static readonly ISerializer YamlSerializer = new SerializerBuilder()
            .Build();

    public static void WriteMarkdown(string path, Dictionary<string, object>? frontmatter, string content)
    {
        var sb = new StringBuilder();
        if (frontmatter != null && frontmatter.Count > 0)
        {
            sb.AppendLine("---");
            var frontmatterCopy = new Dictionary<string, object>(frontmatter);
            // Strip any embedding-related fields; embeddings live only in the database.
            SanitizeFrontmatter(frontmatterCopy);
            var yaml = YamlSerializer.Serialize(frontmatterCopy);
            sb.Append(yaml.TrimEnd());
            sb.AppendLine();
            sb.AppendLine("---");
            sb.AppendLine();
        }
        sb.Append(content);
        var dir = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(dir))
            Directory.CreateDirectory(dir);
        File.WriteAllText(path, sb.ToString());
    }

    public static void UpdateMarkdown(string path, Dictionary<string, object>? frontmatter, string content, string? expectedChecksum)
    {
        if (expectedChecksum != null)
        {
            var (_, _, actualChecksum) = MarkdownReader.ReadMarkdown(path);
            if (actualChecksum != expectedChecksum)
            {
                throw new InvalidOperationException($"Checksum mismatch. File has been modified by another process.");
            }
        }

        WriteMarkdown(path, frontmatter, content);
    }

    public static string GetSessionPath(string thinkingType, string sessionId)
    {
        var timestamp = long.Parse(sessionId.AsSpan(sessionId.LastIndexOf('-') + 1), CultureInfo.InvariantCulture);
        var date = DateTimeOffset.FromUnixTimeMilliseconds(timestamp).UtcDateTime;
        return Path.Combine(Config.MemoryPath, "thinking", thinkingType,
            date.ToString("yyyy", CultureInfo.InvariantCulture), date.ToString("MM", CultureInfo.InvariantCulture), date.ToString("dd", CultureInfo.InvariantCulture), $"{sessionId}.md");
    }

    public static bool SessionExists(string thinkingType, string sessionId)
    {
        try
        {
            var path = GetSessionPath(thinkingType, sessionId);
            return File.Exists(path);
        }
        catch (FormatException)
        {
            return false;
        }
    }

    public static void CreateSession(string thinkingType, string sessionId,
            Dictionary<string, object> frontmatter, string content)
    {
        var path = GetSessionPath(thinkingType, sessionId);
        var dir = Path.GetDirectoryName(path);
        if (dir != null)
            Directory.CreateDirectory(dir);
        WriteMarkdown(path, frontmatter, content);
    }

    public static void UpdateSession(string thinkingType, string sessionId,
            Dictionary<string, object> frontmatter, string content)
    {
        var path = GetSessionPath(thinkingType, sessionId);
        WriteMarkdown(path, frontmatter, content);
    }

    public static void AppendToSession(string thinkingType, string sessionId,
            string heading, string content)
    {
        var path = GetSessionPath(thinkingType, sessionId);
        AppendH2Section(path, heading, content);
    }

    public static void AppendH2Section(string path, string heading, string content)
    {
        var existingContent = File.Exists(path) ? File.ReadAllText(path) : "";
        if (!string.IsNullOrEmpty(existingContent) && !existingContent.EndsWith('\n'))
            existingContent += "\n";
        File.WriteAllText(path, existingContent + $"\n## {heading}\n\n{content}\n");
    }

    public static void CreateMarkdownWithH1(string path, string title, string initialContent = "")
    {
        var markdown = $"# {title}\n\n{initialContent}";
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, markdown);
    }

    public static string PathToUri(string path, string basePath)
    {
        var relativePath = Path.GetRelativePath(basePath, path);
        if (relativePath.EndsWithOrdinalIgnoreCase(".md"))
            relativePath = relativePath.Substring(0, relativePath.Length - 3);
        return $"memory://{relativePath.Replace('\\', '/')}";
    }

    public static string UriToPath(string uri, string basePath)
    {
        uri = uri.Replace("memory://", "").Replace('/', Path.DirectorySeparatorChar);
        var path = Path.Combine(basePath, uri);
        if (!path.EndsWithOrdinalIgnoreCase(".md"))
            path += ".md";


        var fullPath = Path.GetFullPath(path);
        var baseFullPath = Path.GetFullPath(basePath).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
        if (!fullPath.StartsWith(baseFullPath, StringComparison.Ordinal))
            throw new ArgumentException($"Invalid URI attempts to escape memory directory: {uri}");

        return path;
    }

    public static string NormalizeConcept(string concept)
    {
        // RTM: NORMALIZE-HYPHEN-COMPRESSION-001 - Collapse multiple hyphens
        var normalized = concept
            .Trim()
            .ToLowerInvariant()
            .Replace(' ', '-')
            .Replace('_', '-')
            .Replace('/', '-');

        // Collapse multiple adjacent hyphens into single hyphen
        normalized = Regex.Replace(normalized, @"-{2,}", "-");

        // Trim leading/trailing hyphens (defensive)
        normalized = normalized.Trim('-');

        return normalized;
    }

    public static string Slugify(string text)
    {
        text = text.ToLowerInvariant();
        text = Regex.Replace(text, @"[\s_]+", "-");
        text = Regex.Replace(text, @"[^a-z0-9\-]", "");
        text = Regex.Replace(text, @"-+", "-");
        text = text.Trim('-');
        return string.IsNullOrEmpty(text) ? "untitled" : text;
    }

    public static string GenerateChecksum(string content)
    {
        var bytes = Encoding.UTF8.GetBytes(content);
        var hash = SHA256.HashData(bytes);
        return Convert.ToBase64String(hash);
    }

    private static void SanitizeFrontmatter(Dictionary<string, object> frontmatter)
    {
        // Ignore any provided embedding array or metadata keys
        frontmatter.Remove("embedding");
        frontmatter.Remove("embedding_base64");
        frontmatter.Remove("embedding_model");
        frontmatter.Remove("embedding_date");
    }
}
