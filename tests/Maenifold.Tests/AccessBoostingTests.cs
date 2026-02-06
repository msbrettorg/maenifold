// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1, NFR-7.6.2, NFR-7.6.3
// Tests verifying which tools update last_accessed timestamp for access-boosting decay behavior

using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for access-boosting behavior (NFR-7.6.x).
/// Verifies that:
/// - ReadMemory SHALL update last_accessed timestamp (NFR-7.6.1)
/// - SearchMemories SHALL NOT update last_accessed (NFR-7.6.2)
/// - BuildContext SHALL NOT update last_accessed (NFR-7.6.3)
/// </summary>
[TestFixture]
[NonParallelizable] // Database operations need sequential execution
public class AccessBoostingTests
{
    private const string TestFolder = "access-boosting-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);
        Directory.CreateDirectory(_testFolderPath);
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
            directory.Delete(true);
        }
    }

    /// <summary>
    /// Helper to get the last_accessed timestamp from the database for a given memory:// URI.
    /// Returns null if the file is not found or last_accessed column doesn't exist.
    /// </summary>
    private static DateTime? GetLastAccessedFromDb(string memoryUri)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenReadOnly();

        // T-GRAPH-DECAY-002.1: file_content stores memory:// URIs, not filesystem paths
        // Use raw command to handle NULL values properly
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT last_accessed FROM file_content WHERE file_path = @path";
        cmd.Parameters.AddWithValue("@path", memoryUri);

        using var reader = cmd.ExecuteReader();
        if (!reader.Read() || reader.IsDBNull(0))
            return null;

        var result = reader.GetString(0);
        return DateTime.Parse(result, null, System.Globalization.DateTimeStyles.RoundtripKind);
    }

    /// <summary>
    /// Helper to get the file path from a memory:// URI.
    /// </summary>
    private static string UriToFilePath(string memoryUri)
    {
        return MarkdownWriter.UriToPath(memoryUri, Config.MemoryPath);
    }

    /// <summary>
    /// Creates a test memory file and syncs to populate the database.
    /// Returns the memory:// URI and the file path.
    /// </summary>
    private static (string uri, string filePath) CreateAndSyncTestFile(string title, string content, string folder)
    {
        var writeResult = MemoryTools.WriteMemory(title, content, folder: folder);
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"), "WriteMemory should succeed");

        // Extract URI from result
        var uriLine = writeResult.Split('\n')[0];
        var uriStart = uriLine.IndexOf("memory://", StringComparison.Ordinal);
        Assert.That(uriStart, Is.GreaterThanOrEqualTo(0), "WriteMemory should return a memory:// URI");
        var uri = uriLine.Substring(uriStart).Split('\n')[0].Trim();

        // Sync to populate database
        GraphTools.Sync();

        var filePath = UriToFilePath(uri);
        return (uri, filePath);
    }

    #region NFR-7.6.1: ReadMemory SHALL update last_accessed

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
    [Test]
    [Description("ReadMemory SHALL update last_accessed timestamp on every read (NFR-7.6.1)")]
    public void ReadMemory_UpdatesLastAccessed_OnEveryRead()
    {
        // Arrange: Create a test file and sync
        var (uri, _) = CreateAndSyncTestFile(
            "Access Boosting Read Test",
            "Testing [[access-boosting]] behavior for [[ReadMemory]].",
            TestFolder);

        // Get initial last_accessed (should be null or set to indexed time)
        var initialLastAccessed = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"Initial last_accessed: {initialLastAccessed}");

        // Wait a small amount to ensure timestamp difference
        Thread.Sleep(100);

        // Act: Read the memory file
        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Not.StartWith("ERROR:"), "ReadMemory should succeed");

        // Assert: last_accessed should be updated
        var afterReadLastAccessed = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"After read last_accessed: {afterReadLastAccessed}");

        Assert.That(afterReadLastAccessed, Is.Not.Null,
            "last_accessed should be set after ReadMemory (NFR-7.6.1)");

        if (initialLastAccessed.HasValue)
        {
            Assert.That(afterReadLastAccessed, Is.GreaterThan(initialLastAccessed),
                "last_accessed should be updated to a more recent time after ReadMemory");
        }
    }

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
    [Test]
    [Description("ReadMemory updates last_accessed on each subsequent read (NFR-7.6.1)")]
    public void ReadMemory_UpdatesLastAccessed_OnSubsequentReads()
    {
        // Arrange: Create a test file and sync
        var (uri, _) = CreateAndSyncTestFile(
            "Multiple Reads Test",
            "Testing [[access-boosting]] with multiple reads for [[decay-weighting]].",
            TestFolder);

        // First read
        var readResult1 = MemoryTools.ReadMemory(uri);
        Assert.That(readResult1, Does.Not.StartWith("ERROR:"));
        var firstReadTimestamp = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"After first read: {firstReadTimestamp}");

        // Wait to ensure timestamp difference
        Thread.Sleep(100);

        // Second read
        var readResult2 = MemoryTools.ReadMemory(uri);
        Assert.That(readResult2, Does.Not.StartWith("ERROR:"));
        var secondReadTimestamp = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"After second read: {secondReadTimestamp}");

        // Assert: Second read should have later timestamp than first
        Assert.That(secondReadTimestamp, Is.Not.Null,
            "last_accessed should be set after second ReadMemory");
        Assert.That(firstReadTimestamp, Is.Not.Null,
            "last_accessed should be set after first ReadMemory");
        Assert.That(secondReadTimestamp, Is.GreaterThan(firstReadTimestamp),
            "Each ReadMemory call should update last_accessed to current time (NFR-7.6.1)");
    }

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
    [Test]
    [Description("ReadMemory by title also updates last_accessed (NFR-7.6.1)")]
    public void ReadMemory_ByTitle_UpdatesLastAccessed()
    {
        // Arrange: Create a test file and sync
        var title = "Title Based Access Test";
        var (uri, _) = CreateAndSyncTestFile(
            title,
            "Testing [[access-boosting]] when reading by [[title]].",
            TestFolder);

        // Get initial state
        var initialLastAccessed = GetLastAccessedFromDb(uri);
        Thread.Sleep(100);

        // Act: Read by title (not URI)
        var readResult = MemoryTools.ReadMemory(title);
        Assert.That(readResult, Does.Not.StartWith("ERROR:"), "ReadMemory by title should succeed");

        // Assert: last_accessed should be updated
        var afterReadLastAccessed = GetLastAccessedFromDb(uri);
        Assert.That(afterReadLastAccessed, Is.Not.Null,
            "last_accessed should be set after ReadMemory by title (NFR-7.6.1)");

        if (initialLastAccessed.HasValue)
        {
            Assert.That(afterReadLastAccessed, Is.GreaterThan(initialLastAccessed),
                "last_accessed should be updated when reading by title");
        }
    }

    #endregion

    #region NFR-7.6.2: SearchMemories SHALL NOT update last_accessed

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
    [Test]
    [Description("SearchMemories SHALL NOT update last_accessed for files in results (NFR-7.6.2)")]
    public void SearchMemories_DoesNotUpdateLastAccessed()
    {
        // Arrange: Create a test file with searchable content
        var (uri, _) = CreateAndSyncTestFile(
            "Search Results Test",
            "Testing [[search-behavior]] for [[access-boosting]]. This contains machine learning keywords.",
            TestFolder);

        // Read once to ensure last_accessed is set
        MemoryTools.ReadMemory(uri);
        var initialLastAccessed = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"Initial last_accessed (after read): {initialLastAccessed}");

        Assert.That(initialLastAccessed, Is.Not.Null,
            "Precondition: last_accessed should be set before search test");

        // Wait to ensure any timestamp update would be detectable
        Thread.Sleep(200);

        // Act: Search that returns this file in results
        var searchResult = MemorySearchTools.SearchMemories(
            query: "search-behavior access-boosting",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: 0.0);

        TestContext.Out.WriteLine($"Search result: {searchResult}");
        Assert.That(searchResult, Does.Contain("Search Results Test"),
            "Search should find the test file");

        // Assert: last_accessed should NOT be updated by search
        var afterSearchLastAccessed = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"After search last_accessed: {afterSearchLastAccessed}");

        Assert.That(afterSearchLastAccessed, Is.EqualTo(initialLastAccessed),
            "SearchMemories SHALL NOT update last_accessed for files in results (NFR-7.6.2)");
    }

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
    [Test]
    [Description("SearchMemories in Semantic mode does not update last_accessed (NFR-7.6.2)")]
    public void SearchMemories_SemanticMode_DoesNotUpdateLastAccessed()
    {
        // Arrange
        var (uri, _) = CreateAndSyncTestFile(
            "Semantic Search Test",
            "Testing [[semantic-search]] and [[vector-embeddings]] behavior.",
            TestFolder);

        MemoryTools.ReadMemory(uri);
        var initialLastAccessed = GetLastAccessedFromDb(uri);
        Thread.Sleep(200);

        // Act: Semantic search
        var searchResult = MemorySearchTools.SearchMemories(
            query: "semantic search vector",
            mode: SearchMode.Semantic,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: 0.0);

        // Assert
        var afterSearchLastAccessed = GetLastAccessedFromDb(uri);
        Assert.That(afterSearchLastAccessed, Is.EqualTo(initialLastAccessed),
            "Semantic SearchMemories SHALL NOT update last_accessed (NFR-7.6.2)");
    }

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2
    [Test]
    [Description("SearchMemories in FullText mode does not update last_accessed (NFR-7.6.2)")]
    public void SearchMemories_FullTextMode_DoesNotUpdateLastAccessed()
    {
        // Arrange
        var (uri, _) = CreateAndSyncTestFile(
            "FullText Search Test",
            "Testing [[fulltext-search]] and [[keyword-matching]] behavior.",
            TestFolder);

        MemoryTools.ReadMemory(uri);
        var initialLastAccessed = GetLastAccessedFromDb(uri);
        Thread.Sleep(200);

        // Act: FullText search
        var searchResult = MemorySearchTools.SearchMemories(
            query: "fulltext keyword matching",
            mode: SearchMode.FullText,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: 0.0);

        // Assert
        var afterSearchLastAccessed = GetLastAccessedFromDb(uri);
        Assert.That(afterSearchLastAccessed, Is.EqualTo(initialLastAccessed),
            "FullText SearchMemories SHALL NOT update last_accessed (NFR-7.6.2)");
    }

    #endregion

    #region NFR-7.6.3: BuildContext SHALL NOT update last_accessed

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3
    [Test]
    [Description("BuildContext SHALL NOT update last_accessed for referenced files (NFR-7.6.3)")]
    public void BuildContext_DoesNotUpdateLastAccessed()
    {
        // Arrange: Create test files that will be linked via concepts
        var (uri1, filePath1) = CreateAndSyncTestFile(
            "BuildContext Test File 1",
            "Testing [[build-context]] and [[graph-traversal]] concepts.",
            TestFolder);

        var (uri2, filePath2) = CreateAndSyncTestFile(
            "BuildContext Test File 2",
            "More content about [[build-context]] and [[concept-relationships]].",
            TestFolder);

        // Read both files to ensure last_accessed is set
        MemoryTools.ReadMemory(uri1);
        MemoryTools.ReadMemory(uri2);

        var initialLastAccessed1 = GetLastAccessedFromDb(uri1);
        var initialLastAccessed2 = GetLastAccessedFromDb(uri2);
        TestContext.Out.WriteLine($"File1 initial last_accessed: {initialLastAccessed1}");
        TestContext.Out.WriteLine($"File2 initial last_accessed: {initialLastAccessed2}");

        Assert.That(initialLastAccessed1, Is.Not.Null, "Precondition: File1 last_accessed should be set");
        Assert.That(initialLastAccessed2, Is.Not.Null, "Precondition: File2 last_accessed should be set");

        // Wait to ensure any timestamp update would be detectable
        Thread.Sleep(200);

        // Act: Build context around a concept that links to these files
        var contextResult = GraphTools.BuildContext(
            conceptName: "build-context",
            depth: 2,
            maxEntities: 20,
            includeContent: true);

        TestContext.Out.WriteLine($"BuildContext result: {contextResult.ConceptName}, " +
            $"DirectRelations: {contextResult.DirectRelations.Count}, " +
            $"ExpandedRelations: {contextResult.ExpandedRelations.Count}");

        // Assert: last_accessed should NOT be updated by BuildContext
        var afterBuildContextLastAccessed1 = GetLastAccessedFromDb(uri1);
        var afterBuildContextLastAccessed2 = GetLastAccessedFromDb(uri2);

        TestContext.Out.WriteLine($"File1 after BuildContext: {afterBuildContextLastAccessed1}");
        TestContext.Out.WriteLine($"File2 after BuildContext: {afterBuildContextLastAccessed2}");

        Assert.That(afterBuildContextLastAccessed1, Is.EqualTo(initialLastAccessed1),
            "BuildContext SHALL NOT update last_accessed for referenced files (NFR-7.6.3)");
        Assert.That(afterBuildContextLastAccessed2, Is.EqualTo(initialLastAccessed2),
            "BuildContext SHALL NOT update last_accessed for referenced files (NFR-7.6.3)");
    }

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3
    [Test]
    [Description("BuildContext with includeContent=true does not update last_accessed (NFR-7.6.3)")]
    public void BuildContext_WithContentPreview_DoesNotUpdateLastAccessed()
    {
        // Arrange
        var (uri, _) = CreateAndSyncTestFile(
            "Content Preview Test",
            "Testing [[content-preview]] in [[BuildContext]] with [[access-boosting]].",
            TestFolder);

        MemoryTools.ReadMemory(uri);
        var initialLastAccessed = GetLastAccessedFromDb(uri);
        Thread.Sleep(200);

        // Act: BuildContext with content preview enabled
        var contextResult = GraphTools.BuildContext(
            conceptName: "content-preview",
            depth: 1,
            maxEntities: 10,
            includeContent: true);

        // Assert
        var afterContextLastAccessed = GetLastAccessedFromDb(uri);
        Assert.That(afterContextLastAccessed, Is.EqualTo(initialLastAccessed),
            "BuildContext with includeContent=true SHALL NOT update last_accessed (NFR-7.6.3)");
    }

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3
    [Test]
    [Description("BuildContext at various depths does not update last_accessed (NFR-7.6.3)")]
    public void BuildContext_AtVariousDepths_DoesNotUpdateLastAccessed()
    {
        // Arrange: Create a chain of related files
        var (uri, _) = CreateAndSyncTestFile(
            "Depth Test File",
            "Testing [[depth-traversal]] with [[graph-depth]] concepts.",
            TestFolder);

        MemoryTools.ReadMemory(uri);
        var initialLastAccessed = GetLastAccessedFromDb(uri);
        Thread.Sleep(200);

        // Act: BuildContext at depth=3
        var contextResult = GraphTools.BuildContext(
            conceptName: "depth-traversal",
            depth: 3,
            maxEntities: 50,
            includeContent: false);

        // Assert
        var afterContextLastAccessed = GetLastAccessedFromDb(uri);
        Assert.That(afterContextLastAccessed, Is.EqualTo(initialLastAccessed),
            "BuildContext at depth=3 SHALL NOT update last_accessed (NFR-7.6.3)");
    }

    #endregion

    #region Combined Behavior Tests

    // T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1, NFR-7.6.2, NFR-7.6.3
    [Test]
    [Description("Only ReadMemory updates last_accessed, not Search or BuildContext")]
    public void OnlyReadMemory_UpdatesLastAccessed_NotSearchOrBuildContext()
    {
        // Arrange: Create a test file
        var (uri, _) = CreateAndSyncTestFile(
            "Combined Behavior Test",
            "Testing [[combined-access]] patterns for [[decay-weighting]] and [[boosting]].",
            TestFolder);

        // Step 1: Initial read to set last_accessed
        MemoryTools.ReadMemory(uri);
        var afterFirstRead = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"After first read: {afterFirstRead}");
        Assert.That(afterFirstRead, Is.Not.Null, "ReadMemory should set last_accessed");

        Thread.Sleep(200);

        // Step 2: Search (should NOT update)
        MemorySearchTools.SearchMemories(
            query: "combined-access decay",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            tags: null,
            minScore: 0.0);
        var afterSearch = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"After search: {afterSearch}");
        Assert.That(afterSearch, Is.EqualTo(afterFirstRead), "Search should NOT update last_accessed");

        Thread.Sleep(200);

        // Step 3: BuildContext (should NOT update)
        GraphTools.BuildContext(
            conceptName: "combined-access",
            depth: 2,
            maxEntities: 20,
            includeContent: true);
        var afterBuildContext = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"After BuildContext: {afterBuildContext}");
        Assert.That(afterBuildContext, Is.EqualTo(afterFirstRead), "BuildContext should NOT update last_accessed");

        Thread.Sleep(200);

        // Step 4: Another read (SHOULD update)
        MemoryTools.ReadMemory(uri);
        var afterSecondRead = GetLastAccessedFromDb(uri);
        TestContext.Out.WriteLine($"After second read: {afterSecondRead}");
        Assert.That(afterSecondRead, Is.GreaterThan(afterFirstRead),
            "Second ReadMemory SHOULD update last_accessed to a newer timestamp");
    }

    #endregion
}
