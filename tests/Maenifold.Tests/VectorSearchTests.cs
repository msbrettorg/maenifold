#pragma warning disable CA1861
using System.Collections.Generic;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-QUAL-FSC2: RTM FR-7.4
// Test isolation: each test gets its own temp directory and database to avoid
// UNIQUE constraint errors from writing to the shared production vec_concepts table.
[NonParallelizable] // Database operations need sequential execution in CI
public class VectorSearchTests
{
    private string _testRoot = string.Empty;
    private string _previousMaenifoldRootEnv = string.Empty;
    private string _previousDatabasePathEnv = string.Empty;
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        _previousMaenifoldRootEnv = Environment.GetEnvironmentVariable("MAENIFOLD_ROOT") ?? string.Empty;
        _previousDatabasePathEnv = Environment.GetEnvironmentVariable("MAENIFOLD_DATABASE_PATH") ?? string.Empty;

        var repoRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", ".."));
        _testRoot = Path.Combine(repoRoot, "test-outputs", "vector-search", $"run-{Guid.NewGuid():N}");
        Directory.CreateDirectory(_testRoot);

        var testDbPath = Path.Combine(_testRoot, "memory.db");
        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", _testRoot);
        Environment.SetEnvironmentVariable("MAENIFOLD_DATABASE_PATH", testDbPath);
        Config.OverrideRoot(_testRoot);
        Config.SetDatabasePath(testDbPath);
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();

        _testFolderPath = Path.Combine(Config.MemoryPath, "vector-search-tests");
        Directory.CreateDirectory(_testFolderPath);
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "ai"));
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "tutorials"));
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "projects"));
    }

    [TearDown]
    public void TearDown()
    {
        try
        {
            if (!string.IsNullOrWhiteSpace(_testRoot) && Directory.Exists(_testRoot))
            {
                var dir = new DirectoryInfo(_testRoot);
                foreach (var f in dir.GetFiles("*", SearchOption.AllDirectories))
                    f.Attributes = FileAttributes.Normal;
                Directory.Delete(_testRoot, recursive: true);
            }
        }
        catch { }

        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT",
            string.IsNullOrEmpty(_previousMaenifoldRootEnv) ? null : _previousMaenifoldRootEnv);
        Environment.SetEnvironmentVariable("MAENIFOLD_DATABASE_PATH",
            string.IsNullOrEmpty(_previousDatabasePathEnv) ? null : _previousDatabasePathEnv);

        if (string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv)
            && string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
        {
            Config.ResetOverrides();
        }
        else
        {
            if (!string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv))
                Config.OverrideRoot(_previousMaenifoldRootEnv);
            if (!string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
                Config.SetDatabasePath(_previousDatabasePathEnv);
        }
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
    }

    [Test]
    public void SemanticSearchHandlesEmptyQueryGracefully()
    {
        var emptyResult = MemorySearchTools.SearchMemories(
            query: string.Empty,
            mode: SearchMode.Semantic,
            pageSize: 5,
            page: 1);

        Assert.That(emptyResult, Does.Contain("ERROR"),
            "Empty query should return a helpful error message.");
    }

    [Test]
    public void SemanticSearchHandlesWhitespaceOnlyQueryGracefully()
    {
        var whitespaceResult = MemorySearchTools.SearchMemories(
            query: "     ",
            mode: SearchMode.Semantic,
            pageSize: 5,
            page: 1);

        Assert.That(whitespaceResult, Does.Contain("ERROR"),
            "Whitespace-only query should return a helpful error message.");
    }

    [Test]
    public void SemanticSearchHandlesStopwordOnlyQueryGracefully()
    {
        var stopwordResult = MemorySearchTools.SearchMemories(
            query: "the and of",
            mode: SearchMode.Semantic,
            pageSize: 5,
            page: 1);

        Assert.That(stopwordResult, Does.Contain("ERROR"),
            "Stopword-only query should return a helpful error message.");
    }

    private void CreateTestFileWithTags(string fileName, string title, string content, string[] tags)
    {
        var path = Path.Combine(_testFolderPath, fileName);
        var fullContent = $"# {title}\n\n{content}";

        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = title,
            ["tags"] = tags,
            ["type"] = "memory",
            ["status"] = "saved"
        };

        MarkdownIO.WriteMarkdown(path, frontmatter, fullContent);
    }
}
#pragma warning restore CA1861
