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
        // T-META-001.1: FR-24.1 — Envelope management: ensure created/modified are always set.
        // Work on a local copy so the caller's dict is never mutated.
        var envelope = frontmatter != null
            ? new Dictionary<string, object>(frontmatter)
            : new Dictionary<string, object>();
        if (!envelope.ContainsKey("created"))
        {
            // T-FIX-001.3: FR-24.6 — For existing files with missing/corrupt frontmatter,
            // use filesystem creation time instead of "now" to prevent silent timestamp reset.
            envelope["created"] = File.Exists(path)
                ? TimeZoneConverter.FileTimeToIso(File.GetCreationTimeUtc(path))
                : TimeZoneConverter.GetUtcNowIso();
        }
        envelope["modified"] = TimeZoneConverter.GetUtcNowIso();

        var sb = new StringBuilder();
        if (envelope.Count > 0)
        {
            sb.AppendLine("---");
            // Strip any embedding-related fields; embeddings live only in the database.
            SanitizeFrontmatter(envelope);
            var yaml = YamlSerializer.Serialize(envelope);
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
        // T-RTM-001: Strip prefix (e.g. "session-" or "workflow-"), then parse timestamp before next dash
        var afterPrefix = sessionId.AsSpan(sessionId.IndexOf('-') + 1);
        var nextDash = afterPrefix.IndexOf('-');
        var timestampSpan = nextDash >= 0 ? afterPrefix[..nextDash] : afterPrefix;
        var timestamp = long.Parse(timestampSpan, CultureInfo.InvariantCulture);
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
        // T-META-001.2: FR-24.2 — Route through WriteMarkdown for envelope management.
        Dictionary<string, object>? frontmatter;
        string existingContent;
        if (File.Exists(path))
        {
            (frontmatter, existingContent, _) = MarkdownReader.ReadMarkdown(path);
        }
        else
        {
            frontmatter = null;
            existingContent = "";
        }
        if (!string.IsNullOrEmpty(existingContent) && !existingContent.EndsWith('\n'))
            existingContent += "\n";
        var newContent = existingContent + $"\n## {heading}\n\n{content}\n";
        WriteMarkdown(path, frontmatter, newContent);
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
