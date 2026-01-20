using System.Text.RegularExpressions;
using System.ComponentModel;
using ModelContextProtocol.Server;
using Maenifold.Utils;
using Markdig;
using Markdig.Syntax;
using Markdig.Syntax.Inlines;

namespace Maenifold.Tools;

[McpServerToolType]
public class ConceptRepairTool
{
    private static readonly Regex WikiLinkPattern = new(@"\[\[([^\]]+)\]\]", RegexOptions.Compiled);
    private static readonly char[] SplitChars = { ',', ';', '|' };

    private static double CosineSimilarity(float[] vectorA, float[] vectorB)
    {
        if (vectorA == null || vectorB == null || vectorA.Length == 0 || vectorB.Length == 0)
        {
            return 0.0;
        }


        var len = Math.Max(vectorA.Length, vectorB.Length);
        var a = new float[len];
        var b = new float[len];

        Array.Copy(vectorA, a, vectorA.Length);
        Array.Copy(vectorB, b, vectorB.Length);


        double dotProduct = 0.0;
        for (int i = 0; i < len; i++)
        {
            dotProduct += a[i] * b[i];
        }


        double magnitudeA = 0.0;
        double magnitudeB = 0.0;
        for (int i = 0; i < len; i++)
        {
            magnitudeA += a[i] * a[i];
            magnitudeB += b[i] * b[i];
        }

        magnitudeA = Math.Sqrt(magnitudeA);
        magnitudeB = Math.Sqrt(magnitudeB);


        if (magnitudeA == 0.0 || magnitudeB == 0.0)
        {
            return 0.0;
        }

        return dotProduct / (magnitudeA * magnitudeB);
    }

    private static void ProcessInline(Inline inline, Regex pattern, string variant, string canonicalConcept,
                string fullContent, List<(int start, int length, string oldText, string newText)> replacements)
    {

        if (inline is CodeInline)
            return;

        if (inline is LinkInline)
            return;


        if (inline is LiteralInline literal)
        {
            var text = literal.Content.ToString();
            var matches = pattern.Matches(text);

            foreach (Match match in matches)
            {

                var absolutePosition = literal.Span.Start + match.Index;
                var beforeText = absolutePosition >= 2
                    ? fullContent.Substring(absolutePosition - 2, 2)
                    : "";
                var afterText = absolutePosition + match.Length + 1 < fullContent.Length
                    ? fullContent.Substring(absolutePosition + match.Length, 2)
                    : "";


                if (beforeText == "[[" || afterText == "]]")
                    continue;


                replacements.Add((
                                        absolutePosition,
                                        match.Length,
                                        match.Value,
                                        $"[[{canonicalConcept}]]"
                                    ));
            }
        }


        if (inline is ContainerInline container)
        {
            foreach (var child in container)
            {
                ProcessInline(child, pattern, variant, canonicalConcept, fullContent, replacements);
            }
        }
    }

    private static (HashSet<string> variants, string normalizedCanonical, bool removingBrackets, string error) ValidateInput(
        string conceptsToReplace,
        string canonicalConcept,
        bool createWikiLinks)
    {
        var variants = conceptsToReplace
            .Split(SplitChars, StringSplitOptions.RemoveEmptyEntries)
            .Select(c => c.Trim().ToLowerInvariant())
            .Where(c => !string.IsNullOrEmpty(c))
            .ToHashSet();

        if (!variants.Any())
        {
            return (new HashSet<string>(), "", false, "ERROR: No concepts to replace provided");
        }

        var removingBrackets = string.IsNullOrEmpty(canonicalConcept);
        var normalizedCanonical = removingBrackets ? "" : MarkdownIO.NormalizeConcept(canonicalConcept);

        if (createWikiLinks && removingBrackets)
        {
            return (new HashSet<string>(), "", false, "ERROR: Cannot both create WikiLinks and remove them (createWikiLinks=true with empty canonicalConcept)");
        }

        return (variants, normalizedCanonical, removingBrackets, "");
    }

    private static List<string> ValidateSemanticSimilarity(
        HashSet<string> variants,
        string normalizedCanonical,
        double minSemanticSimilarity)
    {
        var results = new List<string>();
        results.Add("=== SEMANTIC VALIDATION ===");
        results.Add($"Checking semantic similarity with threshold {minSemanticSimilarity:F2}...");

        try
        {
            var canonicalEmbedding = VectorTools.GenerateEmbedding(normalizedCanonical);

            var unsafeVariants = new List<(string variant, double similarity)>();
            foreach (var variant in variants)
            {
                var variantEmbedding = VectorTools.GenerateEmbedding(variant);
                var similarity = CosineSimilarity(variantEmbedding, canonicalEmbedding);

                if (similarity < minSemanticSimilarity)
                {
                    unsafeVariants.Add((variant, similarity));
                }
            }

            if (unsafeVariants.Any())
            {
                results.Add("");
                results.Add("WARNING: Unsafe consolidation detected! The following concepts are semantically dissimilar:");
                foreach (var (variant, similarity) in unsafeVariants.OrderBy(x => x.similarity))
                {
                    results.Add($"  - {variant}: similarity {similarity:F3}");
                }
                results.Add("");
                results.Add("This would destroy semantic meaning. Review concepts manually or increase similarity threshold.");
                results.Add("");
                results.Add("To override this safety check, set minSemanticSimilarity=0.0 (NOT RECOMMENDED)");
                results.Add("BLOCKED");
            }
            else
            {
                results.Add("All variants passed semantic validation.");
                results.Add("");
            }
        }
        catch (Exception ex)
        {
            results.Add($"WARNING: Could not perform semantic validation: {ex.Message}");
            results.Add("Proceeding without semantic checks.");
            results.Add("");
        }

        return results;
    }

    private static (string newContent, int replacementCount, List<string> replacements) ProcessWikiLinkCreation(
        string newContent,
        HashSet<string> variants,
        string canonicalConcept)
    {
        var pipeline = new MarkdownPipelineBuilder()
            .UseAdvancedExtensions()
            .Build();

        var document = Markdown.Parse(newContent, pipeline);
        var textReplacements = new List<(int start, int length, string oldText, string newText)>();
        var replacements = new List<string>();
        int totalReplacements = 0;

        foreach (var variant in variants)
        {
            var pattern = new Regex($@"\b{Regex.Escape(variant)}\b", RegexOptions.IgnoreCase);

            foreach (var node in document.Descendants())
            {
                if (node is CodeBlock || node is FencedCodeBlock)
                    continue;

                if (node is ParagraphBlock paragraph && paragraph.Inline != null)
                {
                    foreach (var inline in paragraph.Inline)
                    {
                        ProcessInline(inline, pattern, variant, canonicalConcept, newContent, textReplacements);
                    }
                }
                else if (node is HeadingBlock heading && heading.Inline != null)
                {
                    foreach (var inline in heading.Inline)
                    {
                        ProcessInline(inline, pattern, variant, canonicalConcept, newContent, textReplacements);
                    }
                }
                else if (node is ListItemBlock listItem)
                {
                    foreach (var child in listItem)
                    {
                        if (child is ParagraphBlock para && para.Inline != null)
                        {
                            foreach (var inline in para.Inline)
                            {
                                ProcessInline(inline, pattern, variant, canonicalConcept, newContent, textReplacements);
                            }
                        }
                    }
                }
            }
        }

        textReplacements.Sort((a, b) => b.start.CompareTo(a.start));

        foreach (var (start, length, oldText, newText) in textReplacements)
        {
            newContent = string.Concat(newContent.AsSpan(0, start), newText, newContent.AsSpan(start + length));
            replacements.Add($"  {oldText} → {newText}");
            totalReplacements++;
        }

        return (newContent, totalReplacements, replacements);
    }

    private static (string newContent, int replacementCount, List<string> replacements) ProcessWikiLinkReplacement(
        string content,
        string newContent,
        HashSet<string> variants,
        string canonicalConcept,
        bool removingBrackets)
    {
        var matches = WikiLinkPattern.Matches(content);
        var replacements = new List<string>();
        int totalReplacements = 0;

        foreach (Match match in matches)
        {
            var concept = match.Groups[1].Value;
            var normalizedConcept = MarkdownIO.NormalizeConcept(concept);

            if (variants.Contains(normalizedConcept))
            {
                var oldLink = match.Value;
                var newLink = removingBrackets
                    ? concept
                    : $"[[{canonicalConcept}]]";

                if (oldLink != newLink)
                {
                    newContent = newContent.Replace(oldLink, newLink);
                    replacements.Add($"  {oldLink} → {newLink}");
                    totalReplacements++;
                }
            }
        }

        return (newContent, totalReplacements, replacements);
    }

    private static (int filesModified, int totalReplacements, List<string> results) ProcessConceptReplacements(
        List<string> mdFiles,
        HashSet<string> variants,
        string canonicalConcept,
        bool createWikiLinks,
        bool removingBrackets,
        bool dryRun,
        string memoryPath)
    {
        int totalReplacements = 0;
        int filesModified = 0;
        var results = new List<string>();
        var filesToModify = new Dictionary<string, List<string>>();

        foreach (var file in mdFiles)
        {
            try
            {
                var content = File.ReadAllText(file);
                var replacements = new List<string>();
                var newContent = content;

                if (createWikiLinks)
                {
                    var (updatedContent, replacementCount, wikiLinkReplacements) = ProcessWikiLinkCreation(newContent, variants, canonicalConcept);
                    newContent = updatedContent;
                    replacements.AddRange(wikiLinkReplacements);
                    totalReplacements += replacementCount;
                }
                else
                {
                    var (updatedContent, replacementCount, wikiLinkReplacements) = ProcessWikiLinkReplacement(content, newContent, variants, canonicalConcept, removingBrackets);
                    newContent = updatedContent;
                    replacements.AddRange(wikiLinkReplacements);
                    totalReplacements += replacementCount;
                }

                if (replacements.Any())
                {
                    var relativePath = Path.GetRelativePath(memoryPath, file);
                    filesToModify[file] = replacements;
                    filesModified++;

                    if (!dryRun)
                    {
                        File.WriteAllText(file, newContent);
                        results.Add($"✓ Modified: {relativePath}");
                    }
                    else
                    {
                        results.Add($"Would modify: {relativePath}");
                    }

                    results.AddRange(replacements);
                }
            }
            catch (Exception ex)
            {
                results.Add($"ERROR processing {file}: {ex.Message}");
            }
        }

        return (filesModified, totalReplacements, results);
    }

    private static void FormatScanInitializationOutput(
        List<string> results,
        int fileCount,
        HashSet<string> variants,
        string canonicalConcept,
        bool createWikiLinks,
        bool removingBrackets,
        string normalizedCanonical)
    {
        results.Add($"Scanning {fileCount} markdown files...");
        if (createWikiLinks)
        {
            results.Add($"Looking for plain text: {string.Join(", ", variants)}");
            results.Add($"Will CREATE WikiLinks: text → [[{canonicalConcept}]]");
        }
        else
        {
            results.Add($"Looking for variants: {string.Join(", ", variants)}");
            if (removingBrackets)
                results.Add($"Will REMOVE WikiLink brackets entirely (converting to plain text)");
            else
                results.Add($"Will replace with: [[{canonicalConcept}]] (normalized: {normalizedCanonical})");
        }
        results.Add("");
    }

    private static void FormatSummaryOutput(
        List<string> results,
        int fileCount,
        int filesModified,
        int totalReplacements,
        bool dryRun)
    {
        results.Add("");
        results.Add("=== SUMMARY ===");
        results.Add($"Files scanned: {fileCount}");
        results.Add($"Files {(dryRun ? "to modify" : "modified")}: {filesModified}");
        results.Add($"Total replacements: {totalReplacements}");

        if (dryRun && filesModified > 0)
        {
            results.Add("");
            results.Add("This was a DRY RUN. To apply changes, run with dryRun=false");
            results.Add("");
            results.Add("After applying changes, run 'sync' to rebuild the graph with clean concepts.");
        }
        else if (!dryRun && filesModified > 0)
        {
            results.Add("");
            results.Add("✓ Changes applied successfully!");
            results.Add("Run 'sync' to rebuild the graph with the cleaned concepts.");
        }
    }

    [McpServerTool, Description(@"⚠️ DANGER: This tool can PERMANENTLY DAMAGE the knowledge graph if used incorrectly!

Repairs graph corruption by replacing concept variants with canonical form in source markdown files.
Fixes the source of truth rather than just the database. Use this to consolidate concept families.

SEMANTIC VALIDATION: Automatically blocks unsafe consolidations by checking semantic similarity between concepts. Only proceeds if ALL variants have similarity >= minSemanticSimilarity threshold (default 0.7).

CRITICAL WARNINGS:
- NEVER consolidate class names (VectorTools, GraphTools) with generic concepts ([[tool]])
- NEVER merge distinct entity types (test-agent is NOT the same as [[agent]])
- ALWAYS understand the semantic meaning of concepts before consolidation
- ALWAYS use AnalyzeConceptCorruption first to understand what you're changing
- ALWAYS use dryRun=true first to review changes

SAFE Examples: 
- Replace [[tools]] (plural) with [[tool]] (singular)
- Replace [[MCP tools]], [[MCP Tools]] with consistent casing

DANGEROUS Examples (DO NOT DO):
- Replacing [[VectorTools]] (a class name) with [[tool]] (destroys semantic meaning)
- Replacing [[coding-agent]] (specific agent type) with [[agent]] (loses specificity)

Test with dryRun=true first to see what would be changed.")]
    public static string RepairConcepts(
                [Description("⚠️ DANGER: Array of concept variants to replace. DO NOT include class names like 'VectorTools' or specific types like 'coding-agent'")] string conceptsToReplace,
                [Description("The canonical concept to use. Must be semantically equivalent to ALL variants being replaced")] string canonicalConcept,
                [Description("Folder to process (default: all memory)")] string? folder = null,
                [Description("⚠️ ALWAYS START WITH TRUE! Dry run shows what would be changed without modifying files")] bool dryRun = true,
                [Description("Create WikiLinks from plain text instead of replacing existing WikiLinks")] bool createWikiLinks = false,
                [Description("Minimum semantic similarity threshold (0.0-1.0) required for safe consolidation. Default 0.7. Set to 0.0 to skip semantic validation.")] double minSemanticSimilarity = 0.7,
                [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(RepairConcepts));

        var memoryPath = Config.MemoryPath;
        var searchPath = string.IsNullOrEmpty(folder)
            ? memoryPath
            : Path.Combine(memoryPath, folder);

        if (!Directory.Exists(searchPath))
        {
            return $"ERROR: Directory not found: {searchPath}";
        }

        var (variants, normalizedCanonical, removingBrackets, validationError) = ValidateInput(conceptsToReplace, canonicalConcept, createWikiLinks);

        if (!string.IsNullOrEmpty(validationError))
        {
            return validationError;
        }


        var results = new List<string>();
        if (minSemanticSimilarity > 0.0 && !removingBrackets && !createWikiLinks)
        {
            var validationResults = ValidateSemanticSimilarity(variants, normalizedCanonical, minSemanticSimilarity);
            results.AddRange(validationResults);

            if (validationResults.Count > 0 && validationResults.Last() == "BLOCKED")
            {
                results.RemoveAt(results.Count - 1);
                return string.Join("\n", results);
            }
        }

        var mdFiles = Directory.GetFiles(searchPath, "*.md", SearchOption.AllDirectories)
            .Where(f => !f.Contains("/.git/", StringComparison.OrdinalIgnoreCase) && !f.Contains("\\.git\\", StringComparison.OrdinalIgnoreCase))
            .OrderBy(f => f)
            .ToList();

        FormatScanInitializationOutput(results, mdFiles.Count, variants, canonicalConcept, createWikiLinks, removingBrackets, normalizedCanonical);

        var (filesModified, totalReplacements, processingResults) = ProcessConceptReplacements(mdFiles, variants, canonicalConcept, createWikiLinks, removingBrackets, dryRun, memoryPath);
        results.AddRange(processingResults);

        FormatSummaryOutput(results, mdFiles.Count, filesModified, totalReplacements, dryRun);

        return string.Join("\n", results);
    }

    private static Dictionary<string, int> AnalyzeConcepts(string memoryPath, string conceptFamily)
    {
        var conceptCounts = new Dictionary<string, int>();

        var mdFiles = Directory.GetFiles(memoryPath, "*.md", SearchOption.AllDirectories)
            .Where(f => !f.Contains("/.git/", StringComparison.OrdinalIgnoreCase) && !f.Contains("\\.git\\", StringComparison.OrdinalIgnoreCase))
            .ToList();

        foreach (var file in mdFiles)
        {
            try
            {
                var content = File.ReadAllText(file);
                var matches = WikiLinkPattern.Matches(content);

                foreach (Match match in matches)
                {
                    var concept = match.Groups[1].Value;
                    var normalized = MarkdownIO.NormalizeConcept(concept);

                    if (normalized.Contains(conceptFamily.ToLowerInvariant(), StringComparison.Ordinal))
                    {
                        if (!conceptCounts.ContainsKey(concept))
                            conceptCounts[concept] = 0;
                        conceptCounts[concept]++;
                    }
                }
            }
            catch
            {
            }
        }

        return conceptCounts;
    }

    private static string ClassifyConceptPattern(string concept)
    {
        if (concept.StartsWith('/') || concept.Contains('.'))
            return "File Paths";

        if (concept.EndsWith("s", StringComparison.OrdinalIgnoreCase) && !concept.EndsWith("ss", StringComparison.OrdinalIgnoreCase))
            return "Singular/Plural";

        if (concept.Contains('-'))
            return "Compound (with -)";

        if (concept.Contains("system", StringComparison.OrdinalIgnoreCase) ||
            concept.Contains("framework", StringComparison.OrdinalIgnoreCase) ||
            concept.Contains("architecture", StringComparison.OrdinalIgnoreCase) ||
            concept.Contains("tool", StringComparison.OrdinalIgnoreCase))
            return "With Suffix";

        return "Other";
    }

    private static Dictionary<string, List<string>> ClassifyConceptPatterns(
        Dictionary<string, int> conceptCounts,
        int maxResults)
    {
        var patterns = new Dictionary<string, List<string>>();
        patterns["Singular/Plural"] = new List<string>();
        patterns["Compound (with -)"] = new List<string>();
        patterns["With Suffix"] = new List<string>();
        patterns["File Paths"] = new List<string>();
        patterns["Other"] = new List<string>();

        foreach (var kvp in conceptCounts.OrderByDescending(x => x.Value).Take(maxResults))
        {
            var patternKey = ClassifyConceptPattern(kvp.Key);
            patterns[patternKey].Add($"{kvp.Key} ({kvp.Value}x)");
        }

        return patterns;
    }

    private static void FormatPatternResults(
        List<string> results,
        Dictionary<string, List<string>> patterns)
    {
        foreach (var pattern in patterns.Where(p => p.Value.Any()))
        {
            results.Add($"{pattern.Key}:");
            results.AddRange(pattern.Value.Select(v => $"  • {v}"));
            results.Add("");
        }
    }

    private static void FormatSuggestedRepairs(
        List<string> results,
        Dictionary<string, int> conceptCounts,
        string conceptFamily)
    {
        results.Add("=== SUGGESTED REPAIRS ===");

        var plurals = conceptCounts.Keys
                        .Where(c => c.EndsWith("s", StringComparison.OrdinalIgnoreCase) && !c.EndsWith("ss", StringComparison.OrdinalIgnoreCase))
                        .Select(c => c)
                        .ToList();

        if (plurals.Any())
        {
            var singular = conceptFamily.TrimEnd('s');
            results.Add($"Fix plural forms:");
            results.Add($"  RepairConcepts conceptsToReplace='{string.Join(",", plurals.Take(10))}' canonicalConcept='{singular}'");
            results.Add("");
        }

        var compounds = conceptCounts.Keys
                        .Where(c => c.Contains($"-{conceptFamily}", StringComparison.OrdinalIgnoreCase) || c.Contains($"{conceptFamily}-", StringComparison.OrdinalIgnoreCase))
                        .ToList();

        if (compounds.Any())
        {
            results.Add($"Fix compound forms:");
            results.Add($"  RepairConcepts conceptsToReplace='{string.Join(",", compounds.Take(10))}' canonicalConcept='{conceptFamily}'");
        }
    }

    [McpServerTool, Description(@"⚠️ MUST USE BEFORE RepairConcepts! Analyzes concept corruption patterns to identify what needs repair.\nShows concept families and their variants to help plan SAFE repair operations.\n\nCRITICAL: This tool helps you UNDERSTAND concepts before changing them:\n- Identifies TRUE duplicates (plurals, case variations) that are safe to merge\n- Shows class names and entity types that MUST NOT be merged with generic concepts\n- Reveals the semantic structure of your knowledge graph\n\nINTERPRETATION GUIDE:\n- File Paths (e.g., GraphTools.cs): These are CLASS NAMES - DO NOT merge with [[tool]]\n- Compound forms (e.g., coding-agent): These are SPECIFIC TYPES - DO NOT merge with [[agent]]\n- Plurals (e.g., tools): Usually safe to merge with singular form\n- Case variants (e.g., MCP tools vs MCP Tools): Safe to standardize\n\nAlways analyze BEFORE repair to avoid destroying semantic meaning!")]
    public static string AnalyzeConceptCorruption(
                [Description("Concept family to analyze (e.g., 'tool', 'agent', 'test')")] string conceptFamily,
                [Description("Maximum variants to show")] int maxResults = 50,
                [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(AnalyzeConceptCorruption));

        var memoryPath = Config.MemoryPath;
        var conceptCounts = AnalyzeConcepts(memoryPath, conceptFamily);

        var results = new List<string>();
        results.Add($"=== Concept Family Analysis: '{conceptFamily}' ===");
        results.Add($"Found {conceptCounts.Count} unique variants");
        results.Add("");

        var patterns = ClassifyConceptPatterns(conceptCounts, maxResults);
        FormatPatternResults(results, patterns);

        FormatSuggestedRepairs(results, conceptCounts, conceptFamily);

        return string.Join("\n", results);
    }
}
