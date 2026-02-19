// T-COV-001.10: RTM FR-17.12
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Integration tests for ConceptAnalyzer.AnalyzeConceptCorruption.
/// Uses real filesystem — no mocks, no stubs. Per Ma Protocol.
/// </summary>
[TestFixture]
public class ConceptAnalyzerTests
{
    private const string TestFolder = "concept-analyzer-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
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
            directory.Delete(true);
    }

    private void WriteTestFile(string filename, string content)
    {
        var path = Path.Combine(_testFolderPath, filename);
        File.WriteAllText(path, content);
    }

    // ============ Basic output structure (3 tests) ============

    [Test]
    public void EmptyMemoryDirectoryReportsZeroUniqueVariants()
    {
        // Arrange — no files written to the test folder; folder exists but is empty

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("Found 0 unique variants"),
            "Empty memory directory should report 0 unique variants");
    }

    [Test]
    public void OutputContainsConceptFamilyHeader()
    {
        // Arrange
        WriteTestFile("header-test.md", "Using [[workflow]] in this note.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("=== Concept Family Analysis: 'workflow' ==="),
            "Output must contain concept family header with the searched family name");
    }

    [Test]
    public void OutputContainsSuggestedRepairsSection()
    {
        // Arrange
        WriteTestFile("repairs-test.md", "Using [[workflow]] and [[workflows]].");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("=== SUGGESTED REPAIRS ==="),
            "Output must contain the SUGGESTED REPAIRS section");
    }

    // ============ WikiLink extraction (2 tests) ============

    [Test]
    public void WikiLinksAreExtractedFromFileContent()
    {
        // Arrange
        WriteTestFile("wikilink-test.md", "---\ntags: []\n---\n# Test\n\nUsing [[workflow]] here.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("Found 1 unique variants"),
            "Should extract [[workflow]] WikiLink from file content");
    }

    [Test]
    public void MultipleWikiLinksInSameFileAreCountedCorrectly()
    {
        // Arrange — same concept repeated 3 times in one file
        WriteTestFile("multi-wikilink.md", "# Test\n\n[[workflow]] again [[workflow]] and [[workflow]].");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert — one unique variant found (count = 3, but unique = 1)
        Assert.That(result, Does.Contain("Found 1 unique variants"),
            "Multiple occurrences of the same concept in one file = 1 unique variant");
        Assert.That(result, Does.Contain("workflow (3x)"),
            "Count should reflect total occurrences across the file (3x)");
    }

    // ============ Aggregation across multiple files (2 tests) ============

    [Test]
    public void SingleFileWithMatchingConceptsFindsAndCountsThem()
    {
        // Arrange
        WriteTestFile("single-file.md", "[[workflow]] is used here and [[workflow-dispatch]] is used there.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("Found 2 unique variants"),
            "Should find both 'workflow' and 'workflow-dispatch' as unique variants");
    }

    [Test]
    public void MultipleFilesWithSameConceptFamilyAggregatesCounts()
    {
        // Arrange
        WriteTestFile("file-a.md", "# File A\n\n[[workflow]] is mentioned here.");
        WriteTestFile("file-b.md", "# File B\n\n[[workflow]] appears again.");
        WriteTestFile("file-c.md", "# File C\n\n[[workflows]] is a plural form.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert — 2 unique variants: workflow (2x), workflows (1x)
        Assert.That(result, Does.Contain("Found 2 unique variants"),
            "Two unique variants across three files: 'workflow' and 'workflows'");
        Assert.That(result, Does.Contain("workflow (2x)"),
            "'workflow' should appear 2x (once in file-a and once in file-b)");
        Assert.That(result, Does.Contain("workflows (1x)"),
            "'workflows' should appear 1x (once in file-c)");
    }

    // ============ Pattern classification (5 tests) ============

    [Test]
    public void ConceptStartingWithSlashIsClassifiedAsFilePaths()
    {
        // Arrange — concept like /src/workflow will be classified as "File Paths"
        WriteTestFile("path-concept.md", "See [[/src/workflow]] for details.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("File Paths:"),
            "Concept starting with '/' should be classified as 'File Paths'");
    }

    [Test]
    public void ConceptContainingDotIsClassifiedAsFilePaths()
    {
        // Arrange — concept like workflow.json will be classified as "File Paths"
        WriteTestFile("dot-concept.md", "See [[workflow.json]] for the config.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("File Paths:"),
            "Concept containing '.' should be classified as 'File Paths'");
    }

    [Test]
    public void ConceptEndingInSNotSsIsClassifiedAsSingularPlural()
    {
        // Arrange — "workflows" ends in "s" but not "ss" → Singular/Plural
        WriteTestFile("plural-concept.md", "Using [[workflows]] in the system.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("Singular/Plural:"),
            "Concept ending in 's' (not 'ss') should be classified as 'Singular/Plural'");
    }

    [Test]
    public void ConceptContainingHyphenIsClassifiedAsCompound()
    {
        // Arrange — "workflow-dispatch" contains a hyphen
        WriteTestFile("compound-concept.md", "The [[workflow-dispatch]] event triggers builds.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("Compound (with -):"),
            "Concept containing '-' should be classified as 'Compound (with -)'");
    }

    [Test]
    public void ConceptContainingSystemKeywordIsClassifiedAsWithSuffix()
    {
        // Arrange — "workflow-system" contains "system"
        WriteTestFile("suffix-concept.md", "The [[workflow-system]] manages all pipelines.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert — "workflow-system" contains '-' so hits compound first (priority order)
        // Use a non-hyphenated concept for pure suffix test
        WriteTestFile("pure-suffix.md", "The [[workflowsystem]] architecture is solid.");
        var result2 = ConceptAnalyzer.AnalyzeConceptCorruption("workflowsystem");

        Assert.That(result2, Does.Contain("With Suffix:"),
            "Concept containing 'system' should be classified as 'With Suffix'");
    }

    [Test]
    public void ConceptMatchingNoPatternsIsClassifiedAsOther()
    {
        // Arrange — "workflow" itself: not a path, not plural (doesn't end in s),
        // not compound (no hyphen), not a suffix keyword
        WriteTestFile("other-concept.md", "The [[workflow]] definition.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("Other:"),
            "Concept matching no specific pattern should be classified as 'Other'");
    }

    // ============ Repair suggestions (2 tests) ============

    [Test]
    public void PluralVariantsProducePluralRepairSuggestion()
    {
        // Arrange — "workflows" is a plural variant of "workflow"
        WriteTestFile("plural-repair.md", "Using [[workflows]] in the pipeline.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("Fix plural forms:"),
            "Plural variants should trigger 'Fix plural forms' repair suggestion");
        Assert.That(result, Does.Contain("RepairConcepts"),
            "Repair suggestion must include RepairConcepts command");
        Assert.That(result, Does.Contain("workflows"),
            "Repair suggestion should list the plural variant to replace");
    }

    [Test]
    public void CompoundVariantsProduceCompoundRepairSuggestion()
    {
        // Arrange — "workflow-dispatch" contains "workflow-" prefix
        WriteTestFile("compound-repair.md", "The [[workflow-dispatch]] event is fired.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert
        Assert.That(result, Does.Contain("Fix compound forms:"),
            "Compound variants matching the family should trigger compound repair suggestion");
        Assert.That(result, Does.Contain("workflow-dispatch"),
            "Repair suggestion should list the compound variant to replace");
    }

    // ============ maxResults parameter (1 test) ============

    [Test]
    public void MaxResultsLimitsVariantsShownInOutput()
    {
        // Arrange — create 5 distinct variants
        WriteTestFile("max-results.md",
            "[[workflow]] [[workflow-a]] [[workflow-b]] [[workflow-c]] [[workflow-d]]");

        // Act — limit to 2 results
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow", maxResults: 2);

        // Assert — header reports all unique variants found (5), but classification
        // lists only up to maxResults (2). Verify at most 2 bullet points appear
        // by checking that the output header still says total found correctly,
        // while less common variants are cut off by Take(maxResults).
        Assert.That(result, Does.Contain("Found 5 unique variants"),
            "Found count reflects total unique variants, not maxResults cap");

        // Count bullet points in output
        var bulletCount = result.Split('\n').Count(line => line.Contains("  •", StringComparison.Ordinal));
        Assert.That(bulletCount, Is.LessThanOrEqualTo(2),
            "maxResults=2 should limit bullet points in classification output to at most 2");
    }

    // ============ Git directory exclusion (1 test) ============

    [Test]
    public void FilesInGitDirectoryAreExcluded()
    {
        // Arrange — create a .git subdirectory inside the memory path with a test file
        var gitPath = Path.Combine(_testFolderPath, ".git");
        Directory.CreateDirectory(gitPath);
        File.WriteAllText(Path.Combine(gitPath, "git-tracked.md"),
            "[[workflow]] should NOT be counted — this is inside .git");

        // Also create a legitimate file outside .git
        WriteTestFile("legitimate.md", "No workflow WikiLinks here.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert — the .git file's [[workflow]] must not appear in count
        Assert.That(result, Does.Contain("Found 0 unique variants"),
            "Files inside .git directories must be excluded from analysis");
    }

    // ============ Non-matching concept family (1 test) ============

    [Test]
    public void NonMatchingConceptFamilyReturnsZeroVariants()
    {
        // Arrange — file has [[workflow]] but we search for "agent"
        WriteTestFile("no-match.md", "Using [[workflow]] for orchestration.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("agent");

        // Assert
        Assert.That(result, Does.Contain("Found 0 unique variants"),
            "No concepts matching 'agent' should yield 0 unique variants");
        Assert.That(result, Does.Contain("=== Concept Family Analysis: 'agent' ==="),
            "Header should still contain the searched family name even with no matches");
    }

    // ============ Pattern priority — path takes precedence (1 test) ============

    [Test]
    public void PathPatternTakesPriorityOverPluralPattern()
    {
        // Arrange — "/workflows" starts with '/' AND ends in 's'
        // Path pattern has highest priority and should win
        WriteTestFile("priority-test.md", "See [[/workflows]] for the path.");

        // Act
        var result = ConceptAnalyzer.AnalyzeConceptCorruption("workflow");

        // Assert — classified as File Paths, not Singular/Plural
        Assert.That(result, Does.Contain("File Paths:"),
            "Path pattern takes priority over plural pattern");
    }
}
