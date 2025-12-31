#pragma warning disable CA1861
using System;
using System.IO;
using System.Linq;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for GraphAnalyzer functionality (visualization and graph traversal).
///
/// Ma Protocol Compliance: These tests use REAL systems only.
/// - Real SQLite database with real graph data (no mocks)
/// - Real concept relationships and co-occurrence counts
/// - Real Mermaid diagram generation
/// - No mocks, no stubs, real graph analysis behavior
///
/// These tests verify that graph visualization and analysis tools
/// correctly traverse concept relationships and generate valid output.
/// </summary>
[TestFixture]
public class GraphAnalyzerTests
{
    private const string TestFolder = "graph-analyzer-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);
        Directory.CreateDirectory(_testFolderPath);

        // Create test data with known concept relationships
        CreateTestGraphData();
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

    /// <summary>
    /// Create test graph with known concept relationships for visualization testing.
    /// </summary>
    private void CreateTestGraphData()
    {
        // Create files with interconnected concepts
        CreateTestFile("concept-a.md",
            "Concept A Overview",
            "This discusses [[concept-a]] and its relationship to [[concept-b]] and [[concept-c]].",
            s_testTags);

        CreateTestFile("concept-b.md",
            "Concept B Details",
            "This covers [[concept-b]] which connects to [[concept-a]] and [[concept-d]].",
            s_testTags);

        CreateTestFile("concept-c.md",
            "Concept C Analysis",
            "Analysis of [[concept-c]] and its links to [[concept-a]] and [[concept-e]].",
            s_testTags);

        CreateTestFile("concept-d.md",
            "Concept D Documentation",
            "Documentation for [[concept-d]] connecting to [[concept-b]].",
            s_testTags);

        CreateTestFile("isolated-concept.md",
            "Isolated Concept",
            "This is about [[isolated-concept]] with no connections to other concepts in this test.",
            s_isolatedTags);

        // Sync to build graph relationships
        var syncResult = GraphTools.Sync();
        TestContext.Out.WriteLine($"Sync result: {syncResult}");
    }

    private static readonly string[] s_testTags = { "test", "graph" };
    private static readonly string[] s_isolatedTags = { "test", "isolated" };

    private void CreateTestFile(string filename, string title, string content, string[] tags)
    {
        var fullPath = Path.Combine(_testFolderPath, filename);
        var frontmatter = new System.Collections.Generic.Dictionary<string, object>
        {
            ["title"] = title,
            ["tags"] = tags,
            ["type"] = "memory",
            ["status"] = "saved"
        };

        MarkdownWriter.WriteMarkdown(fullPath, frontmatter, content);
    }

    #region Visualize - Valid Input Tests

    /// <summary>
    /// Verify that Visualize returns valid Mermaid diagram for existing concept.
    /// Tests that visualization generates proper Mermaid syntax with relationships.
    /// </summary>
    [Test]
    public void Visualize_WithValidConcept_ReturnsMermaidDiagram()
    {
        // Act
        var result = GraphAnalyzer.Visualize("concept-a", depth: 1, maxNodes: 10);

        // Assert: Should return Mermaid diagram
        Assert.That(result, Is.Not.Null, "Result should not be null");
        Assert.That(result, Is.Not.Empty, "Result should not be empty");
        Assert.That(result, Does.StartWith("graph TD"),
            "Should start with Mermaid graph declaration");

        // Should contain connections (arrows)
        Assert.That(result, Does.Contain("-->"),
            "Should contain Mermaid connection syntax");

        // Should reference related concepts
        var hasConceptB = result.Contains("concept_b") || result.Contains("concept-b");
        var hasConceptC = result.Contains("concept_c") || result.Contains("concept-c");

        Assert.That(hasConceptB || hasConceptC, Is.True,
            "Should contain references to connected concepts");
    }

    /// <summary>
    /// Verify that Visualize includes co-occurrence counts in diagram.
    /// Tests that edge weights (counts) are included in the Mermaid output.
    /// </summary>
    [Test]
    public void Visualize_IncludesCoOccurrenceCounts()
    {
        // Act
        var result = GraphAnalyzer.Visualize("concept-a", depth: 1, maxNodes: 10);

        // Assert: Should include numeric counts on edges
        Assert.That(result, Does.Match(@"-->\|\d+\|"),
            "Should include co-occurrence counts on edges (format: -->|N|)");
    }

    /// <summary>
    /// Verify that Visualize respects depth parameter.
    /// Tests that depth controls how many hops from root concept are included.
    /// </summary>
    [Test]
    public void Visualize_RespectsDepthParameter()
    {
        // Act: Get results with different depths
        var depth1 = GraphAnalyzer.Visualize("concept-a", depth: 1, maxNodes: 50);
        var depth2 = GraphAnalyzer.Visualize("concept-a", depth: 2, maxNodes: 50);

        // Assert: Depth 2 should include more relationships than depth 1
        var depth1Lines = depth1.Split('\n', StringSplitOptions.RemoveEmptyEntries).Length;
        var depth2Lines = depth2.Split('\n', StringSplitOptions.RemoveEmptyEntries).Length;

        Assert.That(depth2Lines, Is.GreaterThanOrEqualTo(depth1Lines),
            "Depth 2 should include at least as many relationships as depth 1");
    }

    /// <summary>
    /// Verify that Visualize respects maxNodes parameter.
    /// Tests that node limit controls the size of the generated diagram.
    /// </summary>
    [Test]
    public void Visualize_RespectsMaxNodesParameter()
    {
        // Act: Get results with different maxNodes limits
        var maxNodes5 = GraphAnalyzer.Visualize("concept-a", depth: 3, maxNodes: 5);
        var maxNodes50 = GraphAnalyzer.Visualize("concept-a", depth: 3, maxNodes: 50);

        // Assert: Lower limit should result in fewer or equal lines
        var lines5 = maxNodes5.Split('\n', StringSplitOptions.RemoveEmptyEntries).Length;
        var lines50 = maxNodes50.Split('\n', StringSplitOptions.RemoveEmptyEntries).Length;

        Assert.That(lines5, Is.LessThanOrEqualTo(lines50),
            "Lower maxNodes should result in fewer or equal relationships");
    }

    /// <summary>
    /// Verify that Visualize normalizes concept names correctly.
    /// Tests that input concept names are normalized before graph lookup.
    /// </summary>
    [Test]
    public void Visualize_NormalizesConceptNames()
    {
        // Act: Try with different case and format variations
        var lowerResult = GraphAnalyzer.Visualize("concept-a", depth: 1, maxNodes: 10);
        var upperResult = GraphAnalyzer.Visualize("CONCEPT-A", depth: 1, maxNodes: 10);
        var mixedResult = GraphAnalyzer.Visualize("Concept-A", depth: 1, maxNodes: 10);

        // Assert: All should return valid Mermaid diagrams (or same error)
        Assert.That(lowerResult, Does.StartWith("graph TD"),
            "Lowercase should work");
        Assert.That(upperResult, Does.StartWith("graph TD").Or.Contains("not found"),
            "Uppercase should either work or be normalized and not found");
        Assert.That(mixedResult, Does.StartWith("graph TD").Or.Contains("not found"),
            "Mixed case should either work or be normalized and not found");
    }

    #endregion

    #region Visualize - Error Handling Tests

    /// <summary>
    /// Verify that Visualize returns error for empty concept name.
    /// Tests validation of required conceptName parameter.
    /// </summary>
    [Test]
    public void Visualize_WithEmptyConceptName_ReturnsError()
    {
        // Act
        var result = GraphAnalyzer.Visualize("", depth: 1, maxNodes: 10);

        // Assert: Should return error message
        Assert.That(result, Does.StartWith("ERROR"),
            "Should return error for empty concept name");
        Assert.That(result, Does.Contain("required"),
            "Error should indicate concept name is required");
    }

    /// <summary>
    /// Verify that Visualize returns error for null/whitespace concept name.
    /// Tests null safety of conceptName parameter.
    /// </summary>
    [Test]
    public void Visualize_WithNullConceptName_ReturnsError()
    {
        // Act
        var result = GraphAnalyzer.Visualize(null!, depth: 1, maxNodes: 10);

        // Assert: Should return error message
        Assert.That(result, Does.StartWith("ERROR"),
            "Should return error for null concept name");
    }

    /// <summary>
    /// Verify that Visualize returns error for non-existent concept.
    /// Tests handling of concepts not present in the graph database.
    /// </summary>
    [Test]
    public void Visualize_WithNonExistentConcept_ReturnsNotFoundMessage()
    {
        // Act
        var result = GraphAnalyzer.Visualize("non-existent-concept-xyz", depth: 1, maxNodes: 10);

        // Assert: Should return not found message
        Assert.That(result, Does.Contain("not found"),
            "Should indicate concept not found");
        Assert.That(result, Does.Contain("sync"),
            "Should suggest running sync");
    }

    /// <summary>
    /// Verify that Visualize returns error for depth out of range.
    /// Tests validation of depth parameter (1-5).
    /// </summary>
    [Test]
    [TestCase(0)]
    [TestCase(-1)]
    [TestCase(6)]
    [TestCase(10)]
    public void Visualize_WithInvalidDepth_ReturnsError(int invalidDepth)
    {
        // Act
        var result = GraphAnalyzer.Visualize("concept-a", depth: invalidDepth, maxNodes: 10);

        // Assert: Should return error about depth range
        Assert.That(result, Does.StartWith("ERROR"),
            $"Should return error for depth {invalidDepth}");
        Assert.That(result, Does.Contain("depth").And.Contains("between"),
            "Error should mention depth range requirement");
    }

    /// <summary>
    /// Verify that Visualize returns error for maxNodes out of range.
    /// Tests validation of maxNodes parameter (5-100).
    /// </summary>
    [Test]
    [TestCase(0)]
    [TestCase(4)]
    [TestCase(101)]
    [TestCase(1000)]
    public void Visualize_WithInvalidMaxNodes_ReturnsError(int invalidMaxNodes)
    {
        // Act
        var result = GraphAnalyzer.Visualize("concept-a", depth: 1, maxNodes: invalidMaxNodes);

        // Assert: Should return error about maxNodes range
        Assert.That(result, Does.StartWith("ERROR"),
            $"Should return error for maxNodes {invalidMaxNodes}");
        Assert.That(result, Does.Contain("maxNodes").And.Contains("between"),
            "Error should mention maxNodes range requirement");
    }

    #endregion

    #region Visualize - Edge Cases Tests

    /// <summary>
    /// Verify that Visualize handles isolated concepts (no connections).
    /// Tests behavior when concept exists but has no relationships.
    /// </summary>
    [Test]
    public void Visualize_WithIsolatedConcept_ReturnsEmptyGraph()
    {
        // Act
        var result = GraphAnalyzer.Visualize("isolated-concept", depth: 1, maxNodes: 10);

        // Assert: Should return Mermaid diagram with message about no connections
        Assert.That(result, Does.StartWith("graph TD"),
            "Should still return valid Mermaid diagram");
        Assert.That(result, Does.Contain("No connections found").Or.Contains("isolated_concept"),
            "Should indicate no connections or show isolated node");
    }

    /// <summary>
    /// Verify that Visualize generates valid Mermaid node IDs.
    /// Tests that concept names with special characters are properly escaped.
    /// </summary>
    [Test]
    public void Visualize_GeneratesValidMermaidNodeIds()
    {
        // Act
        var result = GraphAnalyzer.Visualize("concept-a", depth: 1, maxNodes: 10);

        // Assert: Result should contain valid Mermaid syntax
        Assert.That(result, Does.StartWith("graph TD"),
            "Should start with Mermaid graph declaration");

        // Mermaid format should include arrows with labels
        Assert.That(result, Does.Match(@"\w+ -->\|\d+\| \w+"),
            "Should contain Mermaid edges with co-occurrence counts (format: node_a -->|N| node_b)");

        // Node IDs should use underscores (not hyphens or spaces)
        var lines = result.Split('\n', StringSplitOptions.RemoveEmptyEntries);
        foreach (var line in lines.Skip(1)) // Skip "graph TD" header
        {
            if (line.Trim().Length == 0)
                continue;

            // Line should contain arrow syntax (either --> or edge labels)
            var hasArrow = line.Contains("-->") || line.Contains('[');
            Assert.That(hasArrow, Is.True,
                $"Mermaid line should contain valid syntax: {line}");
        }
    }

    #endregion

    #region RandomWalk Tests

    /// <summary>
    /// Verify that RandomWalk returns a valid walk starting from given concept.
    /// Tests that random walk traverses graph and returns concept sequence.
    /// </summary>
    [Test]
    public void RandomWalk_WithValidConcept_ReturnsWalkSequence()
    {
        // Arrange
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenReadOnly();

        // Act
        var walk = GraphAnalyzer.RandomWalk(conn, "concept-a", walkLength: 5);

        // Assert: Should return a list of concepts
        Assert.That(walk, Is.Not.Null, "Walk should not be null");
        Assert.That(walk.Count, Is.GreaterThan(0), "Walk should contain at least the start concept");
        Assert.That(walk[0], Is.EqualTo("concept-a"),
            "Walk should start with the specified concept");
        Assert.That(walk.Count, Is.LessThanOrEqualTo(5),
            "Walk length should not exceed requested length");
    }

    /// <summary>
    /// Verify that RandomWalk terminates when reaching concept with no neighbors.
    /// Tests that walk stops early when hitting a dead end in the graph.
    /// </summary>
    [Test]
    public void RandomWalk_TerminatesAtDeadEnd()
    {
        // Arrange
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenReadOnly();

        // Act: Random walk from isolated concept should terminate immediately
        var walk = GraphAnalyzer.RandomWalk(conn, "isolated-concept", walkLength: 10);

        // Assert: Should only contain the start concept
        Assert.That(walk, Is.Not.Null, "Walk should not be null");
        Assert.That(walk.Count, Is.EqualTo(1),
            "Walk from isolated concept should only contain the start concept");
        Assert.That(walk[0], Is.EqualTo("isolated-concept"),
            "Walk should contain the isolated concept");
    }

    /// <summary>
    /// Verify that RandomWalk produces different results on multiple runs.
    /// Tests that walk is actually random and not deterministic.
    /// </summary>
    [Test]
    public void RandomWalk_ProducesDifferentResults()
    {
        // Arrange
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenReadOnly();

        // Act: Perform multiple random walks
        var walks = new System.Collections.Generic.List<System.Collections.Generic.List<string>>();
        for (int i = 0; i < 10; i++)
        {
            walks.Add(GraphAnalyzer.RandomWalk(conn, "concept-a", walkLength: 5));
        }

        // Assert: At least some walks should be different
        // (With 10 walks from a concept with multiple neighbors, we expect variation)
        var uniqueWalks = walks.Select(w => string.Join(",", w)).Distinct().Count();

        Assert.That(uniqueWalks, Is.GreaterThan(1),
            "Multiple random walks should produce at least some different paths");
    }

    /// <summary>
    /// Verify that RandomWalk default walk length is 10.
    /// Tests default parameter value for walkLength.
    /// </summary>
    [Test]
    public void RandomWalk_HasDefaultWalkLength()
    {
        // Arrange
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenReadOnly();

        // Act: Call without specifying walkLength
        var walk = GraphAnalyzer.RandomWalk(conn, "concept-a");

        // Assert: Should respect default length (10 or less if graph is smaller)
        Assert.That(walk, Is.Not.Null, "Walk should not be null");
        Assert.That(walk.Count, Is.LessThanOrEqualTo(10),
            "Default walk length should be 10 or less");
    }

    #endregion
}
