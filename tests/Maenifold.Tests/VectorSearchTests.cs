#pragma warning disable CA1861
using System.Collections.Generic;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

[NonParallelizable] // Database operations need sequential execution in CI
public class VectorSearchTests
{
    private const string TestFolder = "vector-search-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);

        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase(); // Ensure database and vector tables exist
        Directory.CreateDirectory(_testFolderPath);
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "ai"));
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "tutorials"));
        Directory.CreateDirectory(Path.Combine(_testFolderPath, "projects"));

        // NO database isolation - use shared database which already has vector tables
        // Vector extension requires special loading that doesn't work in isolated test databases
    }

    [TearDown]
    public void TearDown()
    {
        // Clean up test folder
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
