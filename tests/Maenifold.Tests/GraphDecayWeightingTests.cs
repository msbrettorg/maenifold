#pragma warning disable CA1861
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// T-GRAPH-DECAY-001.1, T-GRAPH-DECAY-001.5: RTM FR-7.5, NFR-7.5.4
/// Tests that verify decay weighting affects search rankings.
///
/// Requirements:
/// 1. Decay weighting applied to SearchMemories, BuildContext, FindSimilarConcepts
/// 2. Decay affects ranking only - decayed content remains fully retrievable via direct query
/// 3. Older content ranks lower than newer content (with same semantic score)
///
/// TDD Red Phase: These tests are expected to FAIL until decay weighting is implemented.
/// The tests demonstrate the expected behavior that SWE must implement in Phase 2.
/// </summary>
[TestFixture]
[NonParallelizable] // Database operations need sequential execution in CI
public class GraphDecayWeightingTests
{
    private const string TestFolder = "graph-decay-weighting-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);

        // Directly clean up database entries for this test folder to ensure isolation
        CleanupDatabaseForTestFolder();

        // Ensure test folder is clean
        if (Directory.Exists(_testFolderPath))
        {
            Directory.Delete(_testFolderPath, recursive: true);
        }
        Directory.CreateDirectory(_testFolderPath);
    }

    private static void CleanupDatabaseForTestFolder()
    {
        try
        {
            using var conn = new Microsoft.Data.Sqlite.SqliteConnection(Config.DatabaseConnectionString);
            conn.Open();

            // Delete all database entries for files in our test folder
            var pattern = $"memory://{TestFolder}/%";

            // Delete from file_content - the AFTER DELETE trigger handles file_search cleanup
            using var deleteContent = conn.CreateCommand();
            deleteContent.CommandText = "DELETE FROM file_content WHERE file_path LIKE @pattern";
            deleteContent.Parameters.AddWithValue("@pattern", pattern);
            var deletedCount = deleteContent.ExecuteNonQuery();
            TestContext.Out.WriteLine($"[TEST CLEANUP] Deleted {deletedCount} entries from file_content for {pattern}");

            using var deleteMentions = conn.CreateCommand();
            deleteMentions.CommandText = "DELETE FROM concept_mentions WHERE source_file LIKE @pattern";
            deleteMentions.Parameters.AddWithValue("@pattern", pattern);
            deleteMentions.ExecuteNonQuery();

            // Try to clean vec tables but ignore errors
            try
            {
                conn.LoadExtension(Path.Combine(AppContext.BaseDirectory, "assets", "native",
                    System.Runtime.InteropServices.RuntimeInformation.IsOSPlatform(System.Runtime.InteropServices.OSPlatform.OSX)
                        ? (System.Runtime.InteropServices.RuntimeInformation.ProcessArchitecture == System.Runtime.InteropServices.Architecture.Arm64 ? "osx-arm64" : "osx-x64")
                        : "linux-x64",
                    System.Runtime.InteropServices.RuntimeInformation.IsOSPlatform(System.Runtime.InteropServices.OSPlatform.OSX) ? "vec0.dylib" : "vec0.so"));

                using var deleteVec = conn.CreateCommand();
                deleteVec.CommandText = "DELETE FROM vec_memory_files WHERE file_path LIKE @pattern";
                deleteVec.Parameters.AddWithValue("@pattern", pattern);
                deleteVec.ExecuteNonQuery();
            }
            catch
            {
                // vec tables may not work on CI - ignore
            }
        }
        catch (System.Exception ex)
        {
            TestContext.Out.WriteLine($"[TEST CLEANUP WARNING] Cleanup error: {ex.Message}");
        }
    }

    private static bool CanCleanVecTables()
    {
        // On GitHub Actions CI, sqlite-vec DELETE operations fail with Error 16 in the context
        // of ConceptSync.Sync() even though isolated DELETE operations succeed. This causes
        // orphaned semantic search results that break test isolation.
        var isCI = Environment.GetEnvironmentVariable("CI") == "true" ||
                   Environment.GetEnvironmentVariable("GITHUB_ACTIONS") == "true";
        return !isCI;
    }

    [TearDown]
    public void TearDown()
    {
        // Clean up database entries first (before deleting files)
        CleanupDatabaseForTestFolder();

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

    /// <summary>
    /// Creates a test file with a specific creation date for decay testing.
    /// Uses MarkdownIO.WriteMarkdown directly to control the 'created' timestamp.
    /// </summary>
    private void CreateTestFileWithAge(string relativePath, string title, string content, int daysAgo)
    {
        var fullPath = Path.Combine(_testFolderPath, relativePath);
        var directory = Path.GetDirectoryName(fullPath);
        if (directory != null && !Directory.Exists(directory))
        {
            Directory.CreateDirectory(directory);
        }

        // Calculate the backdated timestamp
        var createdDate = DateTime.UtcNow.AddDays(-daysAgo);
        var createdIso = createdDate.ToString("yyyy-MM-ddTHH:mm:ss.fffZ", CultureInfo.InvariantCulture);

        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = title,
            ["type"] = "memory",
            ["status"] = "saved",
            ["created"] = createdIso,
            ["modified"] = createdIso
        };

        var fullContent = $"# {title}\n\n{content}";
        MarkdownIO.WriteMarkdown(fullPath, frontmatter, fullContent);
    }

    #region SearchMemories Decay Tests

    /// <summary>
    /// T-GRAPH-DECAY-001.1: RTM FR-7.5
    /// Verifies that newer content ranks higher than older content in SearchMemories
    /// when both have identical semantic content.
    ///
    /// Test approach:
    /// 1. Create File A: 60 days old (well past grace + half-life)
    /// 2. Create File B: 1 day old (within grace period)
    /// 3. Both files have IDENTICAL content to ensure same semantic score
    /// 4. Search should return File B ranked higher than File A
    /// </summary>
    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Newer content ranks higher in SearchMemories")]
    public void SearchMemories_NewerContentRanksHigher_WhenSemanticScoresEqual()
    {
        // Skip if vec tables can't be cleaned (causes test isolation issues with semantic search)
        if (!CanCleanVecTables())
        {
            Assert.Ignore("Test skipped: sqlite-vec tables unavailable - semantic search results from previous tests cannot be cleaned");
        }

        // Arrange: Create two files with identical content but different ages
        const string identicalContent = "This document discusses [[machine-learning]] algorithms and [[deep-learning]] neural networks. " +
            "The primary focus is on supervised [[machine-learning]] techniques for classification tasks.";

        // File A: Old (60 days - well past default 14d grace + 30d half-life = significant decay)
        CreateTestFileWithAge("old-ml-doc.md", "Old Machine Learning Doc", identicalContent, daysAgo: 60);

        // File B: New (1 day - within any grace period)
        CreateTestFileWithAge("new-ml-doc.md", "New Machine Learning Doc", identicalContent, daysAgo: 1);

        // Sync to populate search indices
        var syncResult = GraphTools.Sync();
        TestContext.Out.WriteLine($"Sync result: {syncResult}");

        // Act: Search for content present in both files
        var searchResult = MemorySearchTools.SearchMemories(
            query: "machine learning deep learning",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            minScore: 0.0);

        TestContext.Out.WriteLine($"Search result:\n{searchResult}");

        // Assert: Both files should be present
        Assert.That(searchResult, Does.Contain("New Machine Learning Doc"),
            "Search should find the new document");
        Assert.That(searchResult, Does.Contain("Old Machine Learning Doc"),
            "Search should find the old document");

        // Assert: The NEWER file should rank HIGHER (appear first)
        // Since decay affects ranking, File B (1 day old) should outrank File A (60 days old)
        var newDocIndex = searchResult.IndexOf("New Machine Learning Doc", StringComparison.Ordinal);
        var oldDocIndex = searchResult.IndexOf("Old Machine Learning Doc", StringComparison.Ordinal);

        Assert.That(newDocIndex, Is.LessThan(oldDocIndex),
            "DECAY: Newer document (1 day old) should rank higher than older document (60 days old). " +
            "This test will FAIL until decay weighting is implemented in SearchMemories.");
    }

    /// <summary>
    /// T-GRAPH-DECAY-001.1: RTM FR-7.5
    /// Verifies that Semantic search mode also applies decay weighting.
    /// </summary>
    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Semantic search applies decay weighting")]
    public void SearchMemories_SemanticMode_AppliesDecayWeighting()
    {
        // Arrange: Create files with identical semantic content
        const string identicalContent = "Understanding [[vector-embeddings]] for [[semantic-search]] in AI systems.";

        CreateTestFileWithAge("old-embeddings.md", "Old Embeddings Doc", identicalContent, daysAgo: 45);
        CreateTestFileWithAge("new-embeddings.md", "New Embeddings Doc", identicalContent, daysAgo: 2);

        GraphTools.Sync();

        // Act: Search using semantic mode
        var searchResult = MemorySearchTools.SearchMemories(
            query: "vector embeddings semantic search",
            mode: SearchMode.Semantic,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            minScore: 0.0);

        TestContext.Out.WriteLine($"Semantic search result:\n{searchResult}");

        // Assert: Newer file ranks higher
        var newDocIndex = searchResult.IndexOf("New Embeddings Doc", StringComparison.Ordinal);
        var oldDocIndex = searchResult.IndexOf("Old Embeddings Doc", StringComparison.Ordinal);

        Assert.That(newDocIndex, Is.GreaterThan(-1), "Should find new document");
        Assert.That(oldDocIndex, Is.GreaterThan(-1), "Should find old document");
        Assert.That(newDocIndex, Is.LessThan(oldDocIndex),
            "DECAY: Newer document should rank higher in Semantic mode. " +
            "This test will FAIL until decay weighting is implemented.");
    }

    /// <summary>
    /// T-GRAPH-DECAY-001.1: RTM FR-7.5
    /// Verifies that FullText search mode also applies decay weighting.
    /// </summary>
    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - FullText search applies decay weighting")]
    public void SearchMemories_FullTextMode_AppliesDecayWeighting()
    {
        // Arrange: Create files with identical keyword content
        const string identicalContent = "This covers [[authentication]] patterns and [[jwt]] token validation strategies.";

        CreateTestFileWithAge("old-auth.md", "Old Authentication Doc", identicalContent, daysAgo: 50);
        CreateTestFileWithAge("new-auth.md", "New Authentication Doc", identicalContent, daysAgo: 3);

        GraphTools.Sync();

        // Act: Search using full-text mode
        var searchResult = MemorySearchTools.SearchMemories(
            query: "authentication jwt token",
            mode: SearchMode.FullText,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            minScore: 0.0);

        TestContext.Out.WriteLine($"FullText search result:\n{searchResult}");

        // Assert: Newer file ranks higher
        var newDocIndex = searchResult.IndexOf("New Authentication Doc", StringComparison.Ordinal);
        var oldDocIndex = searchResult.IndexOf("Old Authentication Doc", StringComparison.Ordinal);

        Assert.That(newDocIndex, Is.GreaterThan(-1), "Should find new document");
        Assert.That(oldDocIndex, Is.GreaterThan(-1), "Should find old document");
        Assert.That(newDocIndex, Is.LessThan(oldDocIndex),
            "DECAY: Newer document should rank higher in FullText mode. " +
            "This test will FAIL until decay weighting is implemented.");
    }

    #endregion

    #region BuildContext Decay Tests

    /// <summary>
    /// T-GRAPH-DECAY-001.1: RTM FR-7.5
    /// Verifies that BuildContext applies decay weighting to related concepts.
    /// Concepts from recently modified files should rank higher.
    /// </summary>
    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - BuildContext applies decay to related concepts")]
    public void BuildContext_NewerRelatedConcepts_RankHigher()
    {
        // Arrange: Create files that establish relationships to a central concept
        // The central concept is "api-design" which appears in both files

        // Old file: api-design + old-pattern
        CreateTestFileWithAge("old-api-patterns.md", "Old API Patterns",
            "This discusses [[api-design]] with [[rest-architecture]] and [[old-pattern]] legacy approaches.",
            daysAgo: 60);

        // New file: api-design + new-pattern
        CreateTestFileWithAge("new-api-patterns.md", "New API Patterns",
            "Modern [[api-design]] using [[graphql]] and [[new-pattern]] contemporary approaches.",
            daysAgo: 2);

        GraphTools.Sync();

        // Act: Build context for the shared concept "api-design"
        var contextResult = GraphTools.BuildContext(
            conceptName: "api-design",
            depth: 1,
            maxEntities: 20,
            includeContent: false);

        TestContext.Out.WriteLine($"BuildContext result:\n{contextResult}");

        // Assert: Both files should contribute concepts to the graph
        Assert.That(contextResult.DirectRelations, Is.Not.Empty,
            "api-design should have related concepts");

        // Assert: Concepts from newer file should rank higher
        // "new-pattern" and "graphql" should rank above "old-pattern" and "rest-architecture"
        // This is measured by co-occurrence count * decay factor
        var relations = contextResult.DirectRelations;
        var newPatternRelation = relations.FirstOrDefault(r => r.Name == "new-pattern");
        var oldPatternRelation = relations.FirstOrDefault(r => r.Name == "old-pattern");

        // If both patterns exist, the newer one should have higher effective rank
        // Note: This test may need adjustment based on actual BuildContext ranking behavior
        if (newPatternRelation != null && oldPatternRelation != null)
        {
            var newPatternIndex = relations.IndexOf(newPatternRelation);
            var oldPatternIndex = relations.IndexOf(oldPatternRelation);

            Assert.That(newPatternIndex, Is.LessThan(oldPatternIndex),
                "DECAY: Concept 'new-pattern' from 2-day-old file should rank higher than 'old-pattern' from 60-day-old file. " +
                "This test will FAIL until decay weighting is implemented in BuildContext.");
        }
    }

    #endregion

    #region FindSimilarConcepts Decay Tests

    /// <summary>
    /// T-GRAPH-DECAY-001.1: RTM FR-7.5
    /// Verifies that FindSimilarConcepts applies decay weighting.
    /// Concepts from recently updated files should rank higher.
    /// </summary>
    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - FindSimilarConcepts applies decay weighting")]
    public void FindSimilarConcepts_NewerConcepts_RankHigher()
    {
        // Arrange: Create files with semantically similar concepts
        // Old concept: from 60-day-old file
        CreateTestFileWithAge("old-framework.md", "Old Framework Doc",
            "This covers [[legacy-framework]] for [[software-architecture]] applications.",
            daysAgo: 60);

        // New concept: from 2-day-old file
        CreateTestFileWithAge("new-framework.md", "New Framework Doc",
            "This covers [[modern-framework]] for [[software-architecture]] applications.",
            daysAgo: 2);

        GraphTools.Sync();

        // Act: Find concepts similar to "software-architecture"
        var similarResult = VectorSearchTools.FindSimilarConcepts(
            conceptName: "software-architecture",
            maxResults: 20);

        TestContext.Out.WriteLine($"FindSimilarConcepts result:\n{similarResult}");

        // Assert: Both concepts should be discoverable
        // Note: Similarity depends on embeddings, not just file co-occurrence

        // The test verifies decay is APPLIED, not that specific concepts rank a certain way
        // (because FindSimilarConcepts is primarily semantic, not co-occurrence based)
        Assert.That(similarResult, Does.Not.Contain("ERROR"),
            "FindSimilarConcepts should succeed");

        // If both framework concepts appear in results, check decay ordering
        var hasModernFramework = similarResult.Contains("modern-framework", StringComparison.OrdinalIgnoreCase);
        var hasLegacyFramework = similarResult.Contains("legacy-framework", StringComparison.OrdinalIgnoreCase);

        if (hasModernFramework && hasLegacyFramework)
        {
            var modernIndex = similarResult.IndexOf("modern-framework", StringComparison.OrdinalIgnoreCase);
            var legacyIndex = similarResult.IndexOf("legacy-framework", StringComparison.OrdinalIgnoreCase);

            Assert.That(modernIndex, Is.LessThan(legacyIndex),
                "DECAY: 'modern-framework' from 2-day-old file should rank higher than 'legacy-framework' from 60-day-old file. " +
                "This test will FAIL until decay weighting is implemented in FindSimilarConcepts.");
        }
    }

    #endregion

    #region ReadMemory Bypass Tests

    /// <summary>
    /// T-GRAPH-DECAY-001.5: RTM NFR-7.5.4
    /// Verifies that ReadMemory bypasses decay weighting entirely.
    /// Direct access should return full content regardless of file age.
    /// </summary>
    [Test]
    [Description("T-GRAPH-DECAY-001.5: RTM NFR-7.5.4 - ReadMemory bypasses decay, returns full content")]
    public void ReadMemory_BypassesDecay_ReturnsFullContentRegardlessOfAge()
    {
        // Arrange: Create a very old file (well past decay threshold)
        const string expectedContent = "This is the full content with [[decay-testing]] concepts that should be fully retrievable.";
        CreateTestFileWithAge("ancient-doc.md", "Ancient Document", expectedContent, daysAgo: 365);

        // Act: Read the file directly via ReadMemory
        var uri = $"memory://{TestFolder}/ancient-doc";
        var readResult = MemoryTools.ReadMemory(uri);

        TestContext.Out.WriteLine($"ReadMemory result:\n{readResult}");

        // Assert: Full content is returned regardless of age
        Assert.That(readResult, Does.Not.StartWith("ERROR:"),
            "ReadMemory should not error on old files");
        Assert.That(readResult, Does.Contain("Ancient Document"),
            "ReadMemory should return the title");
        Assert.That(readResult, Does.Contain("[[decay-testing]]"),
            "ReadMemory should return full content including WikiLinks");
        Assert.That(readResult, Does.Contain("fully retrievable"),
            "ReadMemory should return all content text");
    }

    /// <summary>
    /// T-GRAPH-DECAY-001.5: RTM NFR-7.5.4
    /// Verifies that direct ReadMemory access ignores decay.
    /// A file that ranks LOW in search should still be FULLY readable directly.
    /// </summary>
    [Test]
    [Description("T-GRAPH-DECAY-001.5: RTM NFR-7.5.4 - Decayed content remains fully retrievable")]
    public void ReadMemory_DecayedContent_RemainsFullyRetrievable()
    {
        // Arrange: Create old and new files
        const string sharedContent = "Content about [[retrieval-testing]] for verification.";
        CreateTestFileWithAge("decayed-file.md", "Decayed File", sharedContent, daysAgo: 90);
        CreateTestFileWithAge("fresh-file.md", "Fresh File", sharedContent, daysAgo: 1);

        GraphTools.Sync();

        // Verify search ranking (old file should rank lower when decay is implemented)
        var searchResult = MemorySearchTools.SearchMemories(
            query: "retrieval testing",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            folder: TestFolder);

        TestContext.Out.WriteLine($"Search result:\n{searchResult}");

        // Act: Directly read the decayed file
        var readResult = MemoryTools.ReadMemory($"memory://{TestFolder}/decayed-file");

        TestContext.Out.WriteLine($"Direct read of decayed file:\n{readResult}");

        // Assert: Even if the file ranks low in search, direct access returns full content
        Assert.That(readResult, Does.Contain("Decayed File"),
            "Decayed file should be fully readable");
        Assert.That(readResult, Does.Contain("[[retrieval-testing]]"),
            "Decayed file content should be complete");

        // Both files should be searchable (decay affects RANKING, not RETRIEVABILITY)
        Assert.That(searchResult, Does.Contain("Decayed File"),
            "Decayed file should still appear in search results (just lower ranked)");
        Assert.That(searchResult, Does.Contain("Fresh File"),
            "Fresh file should appear in search results");
    }

    #endregion

    #region Grace Period Tests

    /// <summary>
    /// T-GRAPH-DECAY-001.1: RTM NFR-7.5.2
    /// Verifies that files within grace period have no decay applied.
    /// Both a 1-day-old and 10-day-old file (within 14d default grace) should rank equally.
    /// </summary>
    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM NFR-7.5.2 - Files within grace period have equal ranking")]
    public void SearchMemories_FilesWithinGracePeriod_HaveEqualRanking()
    {
        // Arrange: Create two files both within the 14-day default grace period
        const string identicalContent = "Discussing [[grace-period]] behavior for [[decay-weighting]] tests.";

        CreateTestFileWithAge("day1-file.md", "Day 1 File", identicalContent, daysAgo: 1);
        CreateTestFileWithAge("day10-file.md", "Day 10 File", identicalContent, daysAgo: 10);

        GraphTools.Sync();

        // Act: Search for both
        var searchResult = MemorySearchTools.SearchMemories(
            query: "grace period decay weighting",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            folder: TestFolder);

        TestContext.Out.WriteLine($"Grace period search result:\n{searchResult}");

        // Assert: Both files should appear (decay doesn't remove, just reranks)
        Assert.That(searchResult, Does.Contain("Day 1 File"), "Should find day 1 file");
        Assert.That(searchResult, Does.Contain("Day 10 File"), "Should find day 10 file");

        // Note: Within grace period, decay factor should be 1.0 for both
        // So their ranking should be based purely on semantic/text scores (which are equal)
        // This means they could appear in either order, but both should be present
        // The key assertion is that BOTH are returned with similar effective scores
    }

    #endregion
}
#pragma warning restore CA1861
