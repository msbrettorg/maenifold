using System.Text;
using System.Text.RegularExpressions;
using Markdig;
using Markdig.Extensions.Yaml;
using Markdig.Syntax;
using Markdig.Syntax.Inlines;
using YamlDotNet.Serialization;

namespace Maenifold.Utils;

public static class MarkdownReader
{
    private static readonly MarkdownPipeline Pipeline = new MarkdownPipelineBuilder()
        .UseYamlFrontMatter()
        .Build();

    private static readonly IDeserializer YamlDeserializer = new DeserializerBuilder()
            .Build();

    // T-REL-001: RTM FR-1.1 - WikiLink regex with negative lookbehind/lookahead to reject malformed triple/quadruple brackets
    private static readonly Regex WikiLinkPattern = new(@"(?<!\[)\[\[([^\[\]]+)\]\](?!\])", RegexOptions.Compiled);

    public static (Dictionary<string, object>? frontmatter, string content, string checksum) ReadMarkdown(string path)
    {
        if (!File.Exists(path))
            throw new FileNotFoundException($"File not found: {path}");

        var fullText = File.ReadAllText(path);
        var checksum = MarkdownWriter.GenerateChecksum(fullText);


        var document = Markdown.Parse(fullText, Pipeline);
        var yamlBlock = document.Descendants<YamlFrontMatterBlock>().FirstOrDefault();

        Dictionary<string, object>? frontmatter = null;
        string content = fullText;

        if (yamlBlock != null)
        {

            var yamlText = fullText.Substring(yamlBlock.Span.Start, yamlBlock.Span.Length);


            yamlText = yamlText.Trim('-', '\r', '\n').Trim();

            if (!string.IsNullOrWhiteSpace(yamlText))
            {
                try
                {
                    frontmatter = YamlDeserializer.Deserialize<Dictionary<string, object>>(yamlText);


                    // Do not parse embedding-related fields from frontmatter; embeddings are database-only.
                }
                catch
                {

                }
            }


            var contentStart = yamlBlock.Span.End + 1;
            if (contentStart < fullText.Length)
            {
                content = fullText.Substring(contentStart).TrimStart('\r', '\n');
            }
            else
            {
                content = string.Empty;
            }
        }

        return (frontmatter, content, checksum);
    }

    public static (Dictionary<string, object>? frontmatter, string content, string checksum)
            ReadSession(string thinkingType, string sessionId)
    {
        var path = MarkdownWriter.GetSessionPath(thinkingType, sessionId);
        return ReadMarkdown(path);
    }

    public static List<(string heading, string content)> ExtractH2Sections(string markdown)
    {
        var document = Markdown.Parse(markdown, Pipeline);
        var sections = new List<(string heading, string content)>();
        var blocks = document.ToList();

        for (int i = 0; i < blocks.Count; i++)
        {
            if (blocks[i] is HeadingBlock heading && heading.Level == 2)
            {

                var headingText = "";
                if (heading.Inline != null)
                {
                    var inline = heading.Inline.FirstChild;
                    while (inline != null)
                    {
                        headingText += inline.ToString();
                        inline = inline.NextSibling;
                    }
                }
                headingText = headingText.Trim();

                var content = new StringBuilder();


                for (int j = i + 1; j < blocks.Count; j++)
                {
                    if (blocks[j] is HeadingBlock nextHeading && nextHeading.Level == 2)
                        break;


                    var blockText = GetBlockText(markdown, blocks[j]);
                    if (!string.IsNullOrEmpty(blockText))
                        content.AppendLine(blockText);
                }

                sections.Add((headingText.Trim(), content.ToString().Trim()));
            }
        }

        return sections;
    }

    public static List<string> ExtractWikiLinks(string content)
    {
        return ExtractWikiLinksMarkdownAware(content);
    }

    public static int CountConceptOccurrences(string content, string concept)
    {


        var normalizedConcept = MarkdownWriter.NormalizeConcept(concept);
        var document = Markdown.Parse(content, Pipeline);
        int count = 0;

        foreach (var node in document.Descendants())
        {

            if (node is CodeBlock || node is FencedCodeBlock)
                continue;


            if (node is ParagraphBlock paragraph)
            {
                count += CountConceptsInBlock(content, paragraph, normalizedConcept);
            }
            else if (node is HeadingBlock heading)
            {
                count += CountConceptsInBlock(content, heading, normalizedConcept);
            }
        }

        return count;
    }

    private static int CountConceptsInBlock(string fullContent, LeafBlock block, string normalizedConcept)
    {

        if (block.Span.Start < 0 || block.Span.End > fullContent.Length)
            return 0;

        var blockText = fullContent.Substring(block.Span.Start, block.Span.Length);


        if (block.Inline != null)
        {
            var codeInlineRanges = new List<(int start, int length)>();

            foreach (var inline in block.Inline.Descendants())
            {
                if (inline is CodeInline codeInline)
                {

                    var delimiter = codeInline.Delimiter != '\0' ? codeInline.Delimiter.ToString() : "`";
                    var codeContent = codeInline.Content.ToString();

                    var searchPattern = delimiter + codeContent + delimiter;
                    var index = blockText.IndexOf(searchPattern, StringComparison.Ordinal);
                    if (index >= 0)
                    {
                        codeInlineRanges.Add((index, searchPattern.Length));
                    }
                }
            }


            codeInlineRanges.Sort((a, b) => b.start.CompareTo(a.start));
            foreach (var (start, length) in codeInlineRanges)
            {
                if (start >= 0 && start + length <= blockText.Length)
                {
                    blockText = blockText.Remove(start, length).Insert(start, new string(' ', length));
                }
            }
        }


        var matches = WikiLinkPattern.Matches(blockText);
        int count = 0;
        foreach (Match match in matches)
        {
            var conceptText = match.Groups[1].Value;
            var normalized = MarkdownWriter.NormalizeConcept(conceptText);
            if (normalized == normalizedConcept)
            {
                count++;
            }
        }

        return count;
    }

    private static List<string> ExtractWikiLinksMarkdownAware(string content)
    {
        var concepts = new HashSet<string>();

        try
        {
            var document = Markdown.Parse(content, Pipeline);

            foreach (var node in document.Descendants())
            {

                if (node is CodeBlock || node is FencedCodeBlock)
                    continue;


                if (node is ParagraphBlock paragraph)
                {
                    ExtractConceptsFromBlock(content, paragraph, concepts);
                }
                else if (node is HeadingBlock heading)
                {
                    ExtractConceptsFromBlock(content, heading, concepts);
                }
            }
        }
        catch (ArgumentException ex) when (ex.Message.Contains("depth limit"))
        {
            // Fallback: Use regex-only extraction when Markdig depth limit exceeded
            // This handles adversarial inputs like deeply nested lists (100+ levels)
            return ExtractWikiLinksRegexOnly(content);
        }

        return concepts.ToList();
    }

    private static List<string> ExtractWikiLinksRegexOnly(string content)
    {
        var concepts = new HashSet<string>();
        var matches = WikiLinkPattern.Matches(content);

        foreach (Match match in matches)
        {
            var concept = match.Groups[1].Value;
            var normalized = MarkdownWriter.NormalizeConcept(concept);
            concepts.Add(normalized);
        }

        return concepts.ToList();
    }

    private static void ExtractConceptsFromBlock(string fullContent, LeafBlock block, HashSet<string> concepts)
    {

        if (block.Span.Start < 0 || block.Span.End > fullContent.Length)
            return;

        var blockText = fullContent.Substring(block.Span.Start, block.Span.Length);


        if (block.Inline != null)
        {
            var codeInlineRanges = new List<(int start, int length)>();

            foreach (var inline in block.Inline.Descendants())
            {
                if (inline is CodeInline codeInline)
                {

                    var delimiter = codeInline.Delimiter != '\0' ? codeInline.Delimiter.ToString() : "`";
                    var codeContent = codeInline.Content.ToString();

                    var searchPattern = delimiter + codeContent + delimiter;
                    var index = blockText.IndexOf(searchPattern, StringComparison.Ordinal);
                    if (index >= 0)
                    {
                        codeInlineRanges.Add((index, searchPattern.Length));
                    }
                }
            }


            codeInlineRanges.Sort((a, b) => b.start.CompareTo(a.start));
            foreach (var (start, length) in codeInlineRanges)
            {
                if (start >= 0 && start + length <= blockText.Length)
                {
                    blockText = blockText.Remove(start, length).Insert(start, new string(' ', length));
                }
            }
        }


        var matches = WikiLinkPattern.Matches(blockText);
        foreach (Match match in matches)
        {
            var concept = match.Groups[1].Value;
            var normalized = MarkdownWriter.NormalizeConcept(concept);
            concepts.Add(normalized);
        }
    }

    private static string GetBlockText(string markdown, Block block)
    {

        if (block.Span.Start >= 0 && block.Span.End <= markdown.Length)
        {
            return markdown.Substring(block.Span.Start, block.Span.Length);
        }
        return "";
    }

    // T-PROX-001.1: FR-23.1 — Extract WikiLinks tagged with their H2 section index.
    // Preamble (before first H2) = sectionIndex 0.
    // First H2 = sectionIndex 1, second H2 = sectionIndex 2, etc.
    // Concepts are deduplicated per (concept, sectionIndex) pair.
    // Falls back to regex-only (all at sectionIndex 0) if Markdig exceeds depth limit.
    public static (IReadOnlyList<(string concept, int sectionIndex)> concepts, bool usedFallback) ExtractWikiLinksWithSections(string content)
    {
        var accumulator = new HashSet<(string concept, int sectionIndex)>();

        try
        {
            var document = Markdown.Parse(content, Pipeline);
            var topLevelBlocks = document.ToList();
            int sectionIndex = 0;

            foreach (var block in topLevelBlocks)
            {
                if (block is HeadingBlock h && h.Level == 2)
                {
                    sectionIndex++;
                    continue;
                }

                ExtractFromBlockTree(content, block, sectionIndex, accumulator);
            }
        }
        catch (ArgumentException ex) when (ex.Message.Contains("depth limit"))
        {
            // Fallback: regex-only when Markdig hits depth limit (adversarial deep nesting).
            var fallbackConcepts = new HashSet<string>();
            var matches = WikiLinkPattern.Matches(content);
            foreach (Match match in matches)
            {
                var concept = match.Groups[1].Value;
                var normalized = MarkdownWriter.NormalizeConcept(concept);
                fallbackConcepts.Add(normalized);
            }

            var fallbackResult = fallbackConcepts
                .Select(c => (concept: c, sectionIndex: 0))
                .ToList();
            return (fallbackResult, true);
        }

        return (accumulator.ToList(), false);
    }

    private static void ExtractFromBlockTree(
        string content,
        Markdig.Syntax.Block block,
        int sectionIndex,
        HashSet<(string concept, int sectionIndex)> accumulator)
    {
        // Skip code blocks at all levels.
        if (block is CodeBlock || block is FencedCodeBlock)
            return;

        // Leaf blocks: extract concepts directly.
        if (block is ParagraphBlock paragraph)
        {
            var temp = new HashSet<string>();
            ExtractConceptsFromBlock(content, paragraph, temp);
            foreach (var c in temp)
                accumulator.Add((c, sectionIndex));
            return;
        }

        if (block is HeadingBlock heading && heading.Level != 2)
        {
            var temp = new HashSet<string>();
            ExtractConceptsFromBlock(content, heading, temp);
            foreach (var c in temp)
                accumulator.Add((c, sectionIndex));
            return;
        }

        // Container blocks: recurse into children.
        if (block is Markdig.Syntax.ContainerBlock container)
        {
            foreach (var child in container)
                ExtractFromBlockTree(content, child, sectionIndex, accumulator);
        }
    }

    // Embedding-related fields in frontmatter are ignored.
}
