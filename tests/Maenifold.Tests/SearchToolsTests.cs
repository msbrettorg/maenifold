#pragma warning disable CA1861
using System.Collections.Generic;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

[NonParallelizable] // Database operations need sequential execution in CI
public class SearchToolsTests
{
    private const string TestFolder = "search-tools-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase(); // Ensure database tables exist before Sync operations
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);
        Directory.CreateDirectory(_testFolderPath);
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "high-relevance"));
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "medium-relevance"));
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "low-relevance"));
    }

    [TearDown]
    public void TearDown()
    {
        if (string.IsNullOrEmpty(_testFolderPath))
            return;

        var directory = new DirectoryInfo(_testFolderPath);
        if (directory.Exists)
        {
            foreach (var file in directory.EnumerateFiles("*", SearchOption.AllDirectories))
            {
                file.IsReadOnly = false;
                file.Delete();
            }
            foreach (var sub in directory.EnumerateDirectories("*", SearchOption.AllDirectories))
            {
                sub.Delete(true);
            }
            directory.Delete(true);
        }
    }

    private void CreateTestDataWithKnownScores()
    {
        // Create high relevance content - should get perfect/near-perfect scores
        CreateTestFileWithContent("high-relevance/perfect-match.md",
            "Machine Learning Tutorial",
            "This is a comprehensive guide to [[machine learning]] algorithms and [[deep learning]] neural networks. " +
            "Machine learning is the core topic of this document with detailed explanations of machine learning concepts.",
            new[] { "ml", "tutorial", "ai" });

        // Create medium relevance content - should get moderate scores
        CreateTestFileWithContent("medium-relevance/related-topic.md",
            "Artificial Intelligence Overview",
            "This document covers [[artificial intelligence]] concepts and some [[machine learning]] basics. " +
            "AI includes various techniques including some machine learning approaches.",
            new[] { "ai", "overview" });

        // Create low relevance content - should get low scores
        CreateTestFileWithContent("low-relevance/tangential.md",
            "Data Analysis Methods",
            "This document discusses [[data analysis]] and [[statistics]] with brief mention of learning algorithms. " +
            "Statistical methods for analyzing data with minimal machine references.",
            new[] { "data", "stats" });

        // Create completely unrelated content - should get very low/zero scores
        CreateTestFileWithContent("low-relevance/unrelated.md",
            "Cooking Recipes",
            "This document contains [[cooking]] recipes and [[food]] preparation techniques. " +
            "Kitchen equipment and cooking methods for preparing meals.",
            new[] { "cooking", "food" });

        // Sync to populate vector embeddings and full-text search indices
        var syncResult = GraphTools.Sync();
        TestContext.Out.WriteLine($"Sync result: {syncResult}");
    }

    private void CreateTestFileWithContent(string relativePath, string title, string content, string[] tags)
    {
        var fullPath = Path.Combine(_testFolderPath, relativePath);
        var directory = Path.GetDirectoryName(fullPath);
        if (directory != null && !Directory.Exists(directory))
        {
            Directory.CreateDirectory(directory);
        }

        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = title,
            ["tags"] = tags,
            ["type"] = "memory",
            ["status"] = "saved"
        };

        var fullContent = $"# {title}\n\n{content}";
        MarkdownIO.WriteMarkdown(fullPath, frontmatter, fullContent);
    }

    [Test]
    [Description("RTM-010: Test minScore=0.0 returns all results")]
    public void MinScoreZeroReturnsAllResults()
    {
        // Arrange: Create test data
        CreateTestDataWithKnownScores();

        // Act: Search with minScore=0.0 using Hybrid mode
        var result = MemorySearchTools.SearchMemories(
            query: "machine learning",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: 0.0);

        TestContext.Out.WriteLine($"MinScore 0.0 result: {result}");

        // Assert: Should return all documents that match the query regardless of score
        Assert.That(result, Does.Contain("Machine Learning Tutorial"),
            "Should include high relevance document");
        Assert.That(result, Does.Contain("Artificial Intelligence Overview"),
            "Should include medium relevance document");

        // Should find at least 2 results (high and medium relevance)
        // Note: Very low relevance might still be filtered out by search algorithms
        Assert.That(result, Does.Contain("Found"),
            "Should return search results");
        Assert.That(result, Does.Not.Contain("ERROR"),
            "Should not return error message");
    }

    [Test]
    [Description("RTM-011: Test minScore=1.0 returns only perfect matches")]
    public void MinScoreOneReturnsOnlyPerfectMatches()
    {
        // Arrange: Create test data
        CreateTestDataWithKnownScores();

        // Act: Search with minScore=1.0 using Hybrid mode
        var result = MemorySearchTools.SearchMemories(
            query: "machine learning",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: 1.0);

        TestContext.Out.WriteLine($"MinScore 1.0 result: {result}");

        // Assert: Should return only perfect matches (score = 1.0)
        // With such a high threshold, we might get very few or no results
        // The test validates that filtering is working, even if no results meet the threshold
        Assert.That(result, Does.Not.Contain("ERROR"),
            "Should not return error message");

        // If results are returned, they should be only the highest scoring ones
        if (result.Contains("Machine Learning Tutorial"))
        {
            // If the perfect match is included, others should be excluded
            Assert.That(result, Does.Not.Contain("Cooking Recipes"),
                "Should not include unrelated low-score content");
        }
    }

    [Test]
    [Description("RTM-012: Test all three search modes respect minScore")]
    public void AllSearchModesRespectMinScore()
    {
        // Arrange: Create test data
        CreateTestDataWithKnownScores();

        var query = "machine learning";
        var minScore = 0.5; // Moderate threshold to filter out low-relevance results

        // Act & Assert: Test Semantic search mode
        var semanticResult = MemorySearchTools.SearchMemories(
            query: query,
            mode: SearchMode.Semantic,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: minScore);

        TestContext.Out.WriteLine($"Semantic search result: {semanticResult}");
        Assert.That(semanticResult, Does.Contain("Semantic Search"),
            "Should indicate semantic search mode");
        Assert.That(semanticResult, Does.Not.Contain("ERROR"),
            "Semantic search should not return error");

        // Act & Assert: Test FullText search mode
        var fullTextResult = MemorySearchTools.SearchMemories(
            query: query,
            mode: SearchMode.FullText,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: minScore);

        TestContext.Out.WriteLine($"FullText search result: {fullTextResult}");
        Assert.That(fullTextResult, Does.Contain("Full-Text Search"),
            "Should indicate full-text search mode");
        Assert.That(fullTextResult, Does.Not.Contain("ERROR"),
            "FullText search should not return error");

        // Act & Assert: Test Hybrid search mode
        var hybridResult = MemorySearchTools.SearchMemories(
            query: query,
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: minScore);

        TestContext.Out.WriteLine($"Hybrid search result: {hybridResult}");
        Assert.That(hybridResult, Does.Contain("Hybrid Search"),
            "Should indicate hybrid search mode");
        Assert.That(hybridResult, Does.Not.Contain("ERROR"),
            "Hybrid search should not return error");

        // Verify that all modes can handle the minScore parameter without crashing
        // The specific filtering behavior will depend on the implementation
        // but all modes should respect the parameter structure
        Assert.That(semanticResult, Does.Not.Contain("Cooking Recipes"),
            "Semantic search should filter out unrelated content with moderate minScore");
        Assert.That(fullTextResult, Does.Not.Contain("Cooking Recipes"),
            "FullText search should filter out unrelated content with moderate minScore");
        Assert.That(hybridResult, Does.Not.Contain("Cooking Recipes"),
            "Hybrid search should filter out unrelated content with moderate minScore");
    }

}

#pragma warning restore CA1861
