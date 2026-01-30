using System;
using System.IO;
using System.Linq;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

public class AssumptionLedgerTests
{
    private const string TestFolder = "assumptions";
    private string _testFolderPath = "";
    private string _testRunId = "";

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();

        // Create unique test directory for this specific test run
        // Following TESTING_PHILOSOPHY: Use real directories but ensure test isolation
        _testRunId = Guid.NewGuid().ToString("N").Substring(0, 8);
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder, "test-runs", _testRunId);

        // Create isolated test directory
        Directory.CreateDirectory(_testFolderPath);
    }

    [TearDown]
    public void TearDown()
    {
        // Keep artifacts for debugging per TESTING_PHILOSOPHY
        // Clean up is handled by SetUp creating unique test directories
        // This prevents test pollution while preserving evidence for debugging
    }

    [Test]
    [Description("Test creating a new assumption with required parameters")]
    public void AppendAssumptionCreatesFileWithCorrectStructure()
    {
        // Arrange
        var assumption = "The dialogue tool will remain MCP-only";
        var concepts = new[] { "dialogue", "workflow-dispatch" };
        var confidence = "medium";
        var validationPlan = "Validate once dialogue MCP hooks are reintroduced";
        var context = "workflow://thinking/session-1756610546730";

        // Get current date for file location
        var now = DateTime.UtcNow;
        var expectedYear = now.Year.ToString(System.Globalization.CultureInfo.InvariantCulture);
        var expectedMonth = now.Month.ToString("00", System.Globalization.CultureInfo.InvariantCulture);
        var datePath = Path.Combine(Config.MemoryPath, "assumptions", expectedYear, expectedMonth);

        // Count files BEFORE the test to ensure we're testing isolation
        var filesBefore = Directory.Exists(datePath)
            ? Directory.GetFiles(datePath, "assumption-*.md").Length
            : 0;

        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: assumption,
            context: context,
            validationPlan: validationPlan,
            confidence: confidence,
            concepts: concepts);

        // Assert - check the response
        Assert.That(result, Does.StartWith("✅ Assumption recorded at memory://assumptions/"));
        Assert.That(result, Does.Contain($"**Statement**: {assumption}"));
        Assert.That(result, Does.Contain($"**Confidence**: {confidence}"));
        Assert.That(result, Does.Contain("**Status**: active"));
        Assert.That(result, Does.Contain($"**Context**: {context}"));
        Assert.That(result, Does.Contain("💡 Next: Consider running Sync()"));

        // Verify EXACTLY ONE NEW file was created
        Assert.That(Directory.Exists(datePath), Is.True, "Date-based directory should exist");

        var filesAfter = Directory.GetFiles(datePath, "assumption-*.md");
        Assert.That(filesAfter.Length, Is.EqualTo(filesBefore + 1),
            "Should create exactly one new assumption file");

        // Find the newest file by creation time (added after filesBefore count)
        var newestFile = filesAfter
            .OrderByDescending(f => File.GetCreationTimeUtc(f))
            .First();

        // Verify the newly created file has correct structure
        var content = File.ReadAllText(newestFile);
        Assert.That(content, Does.Contain("status: active"));
        Assert.That(content, Does.Contain($"confidence: {confidence}"));
        Assert.That(content, Does.Contain($"context: {context}"));
        Assert.That(content, Does.Contain($"validation_plan: {validationPlan}"));
        Assert.That(content, Does.Contain("[[dialogue]]"));
        Assert.That(content, Does.Contain("[[workflow-dispatch]]"));
    }

    [Test]
    [Description("Test that concepts parameter is required")]
    public void AppendAssumptionRequiresConcepts()
    {
        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: "Test assumption",
            concepts: Array.Empty<string>());

        // Assert
        Assert.That(result, Does.StartWith("ERROR: At least one [[WikiLink]] tag must be provided"));
    }

    [Test]
    [Description("Test that concepts should not include brackets")]
    public void AppendAssumptionRejectsConceptsWithBrackets()
    {
        // Arrange
        string[] conceptsWithBrackets = ["[[dialogue]]"];

        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: "Test assumption",
            concepts: conceptsWithBrackets);

        // Assert
        Assert.That(result, Does.StartWith("ERROR: Concept tags should not include brackets"));
    }

    [Test]
    [Description("Test reading an existing assumption")]
    public void ReadAssumptionReturnsCorrectContent()
    {
        // Arrange: Create an assumption first
        var assumption = "Test assumption for reading";
        var concepts = new[] { "test-concept" };
        var createResult = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: assumption,
            concepts: concepts);

        // Extract URI from create result
        var uriStart = createResult.IndexOf("memory://", StringComparison.Ordinal);
        var uriEnd = createResult.IndexOf('\n', uriStart);
        var uri = createResult.Substring(uriStart, uriEnd - uriStart);

        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "read",
            uri: uri);

        // Assert
        Assert.That(result, Does.StartWith("📋 **Assumption**:"));
        Assert.That(result, Does.Contain("**Status**: active"));
        Assert.That(result, Does.Contain(assumption));
        Assert.That(result, Does.Contain("[[test-concept]]"));
    }

    [Test]
    [Description("Test updating an assumption's status")]
    public void UpdateAssumptionChangesStatus()
    {
        // Arrange: Create an assumption first
        var assumption = "Test assumption for updating";
        var concepts = new[] { "test-update" };
        var createResult = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: assumption,
            concepts: concepts);

        // Extract URI from create result
        var uriStart = createResult.IndexOf("memory://", StringComparison.Ordinal);
        var uriEnd = createResult.IndexOf('\n', uriStart);
        var uri = createResult.Substring(uriStart, uriEnd - uriStart);

        // Act
        var updateResult = AssumptionLedgerTools.AssumptionLedger(
            action: "update",
            uri: uri,
            status: "validated",
            notes: "Test validation complete");

        // Assert
        Assert.That(updateResult, Does.StartWith("✅ Assumption updated at"));
        Assert.That(updateResult, Does.Contain("**New Status**: validated"));

        // Verify the file was updated
        var readResult = AssumptionLedgerTools.AssumptionLedger(
            action: "read",
            uri: uri);

        Assert.That(readResult, Does.Contain("**Status**: validated"));
        Assert.That(readResult, Does.Contain("Test validation complete"));
    }

    [Test]
    [Description("Test updating with invalid status is rejected")]
    public void UpdateAssumptionRejectsInvalidStatus()
    {
        // Arrange: Create an assumption first
        var assumption = "Test assumption";
        var concepts = new[] { "test" };
        var createResult = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: assumption,
            concepts: concepts);

        var uriStart = createResult.IndexOf("memory://", StringComparison.Ordinal);
        var uriEnd = createResult.IndexOf('\n', uriStart);
        var uri = createResult.Substring(uriStart, uriEnd - uriStart);

        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "update",
            uri: uri,
            status: "invalid-status");

        // Assert
        Assert.That(result, Does.StartWith("ERROR: Invalid status"));
        Assert.That(result, Does.Contain("active, validated, invalidated, refined"));
    }

    [Test]
    [Description("Test that updating adds timestamp to notes")]
    public void UpdateAssumptionAddsTimestampedNotes()
    {
        // Arrange: Create an assumption first
        var assumption = "Test assumption for notes";
        var concepts = new[] { "test-notes" };
        var createResult = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: assumption,
            concepts: concepts);

        var uriStart = createResult.IndexOf("memory://", StringComparison.Ordinal);
        var uriEnd = createResult.IndexOf('\n', uriStart);
        var uri = createResult.Substring(uriStart, uriEnd - uriStart);

        // Act
        var updateResult = AssumptionLedgerTools.AssumptionLedger(
            action: "update",
            uri: uri,
            notes: "Additional context added");

        // Assert
        Assert.That(updateResult, Does.StartWith("✅ Assumption updated at"));

        // Verify notes appear in file with timestamp
        var readResult = AssumptionLedgerTools.AssumptionLedger(
            action: "read",
            uri: uri);

        Assert.That(readResult, Does.Contain("## Update:"));
        Assert.That(readResult, Does.Contain("Additional context added"));
    }

    [Test]
    [Description("Test reading non-existent assumption returns error")]
    public void ReadNonExistentAssumptionReturnsError()
    {
        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "read",
            uri: "memory://assumptions/2025/01/assumption-nonexistent");

        // Assert
        Assert.That(result, Does.StartWith("ERROR: Assumption not found"));
    }

    [Test]
    [Description("Test updating non-existent assumption returns error")]
    public void UpdateNonExistentAssumptionReturnsError()
    {
        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "update",
            uri: "memory://assumptions/2025/01/assumption-nonexistent",
            status: "validated");

        // Assert
        Assert.That(result, Does.StartWith("ERROR: Assumption not found"));
    }

    [Test]
    [Description("Test invalid action returns error")]
    public void InvalidActionReturnsError()
    {
        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "invalid-action");

        // Assert
        Assert.That(result, Does.StartWith("ERROR: Invalid action"));
        Assert.That(result, Does.Contain("'append', 'update', or 'read'"));
    }

    [Test]
    [Description("Test assumption file is stored with correct date-based path")]
    public void AssumptionFileUsesCorrectDateBasedPath()
    {
        // Arrange
        var now = DateTime.UtcNow;
        var expectedYear = now.Year.ToString(System.Globalization.CultureInfo.InvariantCulture);
        var expectedMonth = now.Month.ToString("00", System.Globalization.CultureInfo.InvariantCulture);
        string[] testConcepts = ["test"];

        // Count files before test
        var datePath = Path.Combine(Config.MemoryPath, "assumptions", expectedYear, expectedMonth);
        var filesBefore = Directory.Exists(datePath)
            ? Directory.GetFiles(datePath, "assumption-*.md").Length
            : 0;

        // Act
        var result = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: "Test date path",
            concepts: testConcepts);

        // Assert
        Assert.That(result, Does.Contain($"memory://assumptions/{expectedYear}/{expectedMonth}/assumption-"));

        // Verify physical file exists in correct directory
        Assert.That(Directory.Exists(datePath), Is.True, "Month directory should exist");

        var files = Directory.GetFiles(datePath, "assumption-*.md");
        Assert.That(files.Length, Is.EqualTo(filesBefore + 1),
            "Should create exactly one new file in correct date directory");
    }

    [Test]
    [Description("Test updating confidence level")]
    public void UpdateAssumptionChangesConfidence()
    {
        // Arrange: Create an assumption
        var assumption = "Test confidence update";
        var concepts = new[] { "test" };
        var createResult = AssumptionLedgerTools.AssumptionLedger(
            action: "append",
            assumption: assumption,
            concepts: concepts,
            confidence: "low");

        var uriStart = createResult.IndexOf("memory://", StringComparison.Ordinal);
        var uriEnd = createResult.IndexOf('\n', uriStart);
        var uri = createResult.Substring(uriStart, uriEnd - uriStart);

        // Act
        var updateResult = AssumptionLedgerTools.AssumptionLedger(
            action: "update",
            uri: uri,
            confidence: "high");

        // Assert
        Assert.That(updateResult, Does.Contain("**New Confidence**: high"));

        // Verify the update
        var readResult = AssumptionLedgerTools.AssumptionLedger(
            action: "read",
            uri: uri);

        Assert.That(readResult, Does.Contain("**Confidence**: high"));
    }
}
