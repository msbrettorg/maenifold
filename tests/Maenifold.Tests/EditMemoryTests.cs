using System;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

public class EditMemoryTests
{
    private const string TestFolder = "edit-memory-tests";
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
        {
            directory.Delete(true);
        }
    }

    [Test]
    public void FindReplace_WithCorrectExpectedCount_Succeeds()
    {
        // Arrange: Create a test memory file
        var testTitle = "Find Replace Test";
        var testContent = "This is a [[test]] with foo appearing twice: foo and foo.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        // Extract URI
        var uri = ExtractUri(writeResult);

        // Act: Replace "foo" with "[[bar]]" expecting 3 matches (note: "foo" appears 3 times in original)
        var editResult = MemoryTools.EditMemory(
            uri,
            "find_replace",
            "[[bar]]",  // Replacement text must contain [[concepts]]
            findText: "foo",
            expectedCount: 3
        );

        // Assert: Edit should succeed
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        // Verify: Content should be updated
        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("[[bar]] appearing twice: [[bar]] and [[bar]]"));
        Assert.That(readResult, Does.Not.Contain("foo"));
    }

    [Test]
    public void FindReplace_WithIncorrectExpectedCount_ThrowsArgumentException()
    {
        // Arrange: Create a test memory file
        var testTitle = "Find Replace Mismatch Test";
        var testContent = "This is a [[test]] with foo appearing twice: foo and foo.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        var uri = ExtractUri(writeResult);

        // Act & Assert: Expecting 1 match but actually has 3 should throw
        var ex = Assert.Throws<ArgumentException>(() =>
            MemoryTools.EditMemory(
                uri,
                "find_replace",
                "replacement with [[test]]",
                findText: "foo",
                expectedCount: 1
            )
        );

        // Verify error message is clear
        Assert.That(ex.Message, Does.Contain("Expected 1 matches but found 3"));
        Assert.That(ex.Message, Does.Contain("Find text: 'foo'"));
        Assert.That(ex.ParamName, Is.EqualTo("expectedCount"));
    }

    [Test]
    public void FindReplace_WithExpectedCountZero_ThrowsWhenMatchesFound()
    {
        // Arrange
        var testTitle = "Zero Expected Test";
        var testContent = "This is a [[test]] with foo in it.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act & Assert: Expecting 0 matches but has 1
        var ex = Assert.Throws<ArgumentException>(() =>
            MemoryTools.EditMemory(
                uri,
                "find_replace",
                "replacement with [[test]]",
                findText: "foo",
                expectedCount: 0
            )
        );

        Assert.That(ex.Message, Does.Contain("Expected 0 matches but found 1"));
    }

    [Test]
    public void FindReplace_WithNullExpectedCount_Succeeds()
    {
        // Arrange
        var testTitle = "No Count Validation Test";
        var testContent = "This is a [[test]] with foo appearing multiple times: foo, foo, foo.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace without expectedCount validation
        var editResult = MemoryTools.EditMemory(
            uri,
            "find_replace",
            "This is a [[test]] with bar appearing multiple times: bar, bar, bar.",
            findText: "foo",
            expectedCount: null
        );

        // Assert: Should succeed regardless of match count
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("bar appearing multiple times: bar, bar, bar"));
    }

    [Test]
    public void FindReplace_WithNoMatches_ThrowsWhenExpectingMatches()
    {
        // Arrange
        var testTitle = "No Matches Test";
        var testContent = "This is a [[test]] without the target text.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act & Assert: Expecting 2 matches but has 0
        var ex = Assert.Throws<ArgumentException>(() =>
            MemoryTools.EditMemory(
                uri,
                "find_replace",
                "replacement with [[test]]",
                findText: "nonexistent",
                expectedCount: 2
            )
        );

        Assert.That(ex.Message, Does.Contain("Expected 2 matches but found 0"));
        Assert.That(ex.Message, Does.Contain("Find text: 'nonexistent'"));
    }

    [Test]
    public void FindReplace_WithMultilinePattern_WorksCorrectly()
    {
        // Arrange
        var testTitle = "Multiline Replace Test";
        var testContent = @"This is a [[test]] file.

First paragraph with [[content]].

Second paragraph with [[more-content]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace "paragraph" appearing twice
        var editResult = MemoryTools.EditMemory(
            uri,
            "find_replace",
            testContent.Replace("paragraph", "section"),
            findText: "paragraph",
            expectedCount: 2
        );

        // Assert
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("First section with [[content]]"));
        Assert.That(readResult, Does.Contain("Second section with [[more-content]]"));
    }

    [Test]
    public void Append_WithConcepts_Succeeds()
    {
        // Arrange
        var testTitle = "Append Test";
        var testContent = "Original content with [[test]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act
        var editResult = MemoryTools.EditMemory(
            uri,
            "append",
            "Appended content with [[more-concepts]]."
        );

        // Assert
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("Original content with [[test]]"));
        Assert.That(readResult, Does.Contain("Appended content with [[more-concepts]]"));
    }

    [Test]
    public void Prepend_WithConcepts_Succeeds()
    {
        // Arrange
        var testTitle = "Prepend Test";
        var testContent = "Original content with [[test]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act
        var editResult = MemoryTools.EditMemory(
            uri,
            "prepend",
            "Prepended content with [[new-concepts]]."
        );

        // Assert
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("Prepended content with [[new-concepts]]"));
        Assert.That(readResult, Does.Contain("Original content with [[test]]"));
    }

    [Test]
    public void EditMemory_WithoutConcepts_ReturnsError()
    {
        // Arrange
        var testTitle = "No Concepts Test";
        var testContent = "Original content with [[test]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act
        var editResult = MemoryTools.EditMemory(
            uri,
            "append",
            "Content without any concepts."
        );

        // Assert
        Assert.That(editResult, Does.StartWith("ERROR: Edited content must contain at least one [[concept]]"));
    }

    private static string ExtractUri(string writeResult)
    {
        var lines = writeResult.Split('\n');
        var uriLine = lines.FirstOrDefault(line => line.Contains("memory://")) ?? string.Empty;
        var uriStart = uriLine.IndexOf("memory://", StringComparison.Ordinal);
        Assert.That(uriStart, Is.GreaterThanOrEqualTo(0), "Expected memory:// URI in write result");
        return uriLine.Substring(uriStart).Trim();
    }
    // =============================================================================
    // FIX #2: replace_section Tests
    // =============================================================================

    [Test]
    public void ReplaceSection_WhenSectionExists_ReplacesContent()
    {
        // Arrange: Create a memory with multiple sections
        var testTitle = "Replace Section Test";
        var testContent = @"# Introduction

This is the introduction section with [[concept]].

## Methodology

This is the OLD methodology content that will be replaced.

## Results

These are the results section with [[data]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace the Methodology section
        var newMethodology = @"## Methodology

This is the NEW methodology content with [[improved-approach]] and [[better-techniques]].";

        var editResult = MemoryTools.EditMemory(
            uri,
            "replace_section",
            newMethodology,
            sectionName: "Methodology"
        );

        // Assert: Edit should succeed
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        // Verify: Old content is GONE, new content is PRESENT
        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("NEW methodology content"));
        Assert.That(readResult, Does.Contain("[[improved-approach]]"));
        Assert.That(readResult, Does.Contain("[[better-techniques]]"));
        Assert.That(readResult, Does.Not.Contain("OLD methodology content"));

        // Verify: Other sections are untouched
        Assert.That(readResult, Does.Contain("introduction section with [[concept]]"));
        Assert.That(readResult, Does.Contain("results section with [[data]]"));
    }

    [Test]
    public void ReplaceSection_WhenSectionDoesNotExist_ThrowsInvalidOperationException()
    {
        // Arrange
        var testTitle = "Missing Section Test";
        var testContent = @"# Introduction

This is the introduction with [[concept]].

## Results

These are the results with [[data]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act & Assert: Trying to replace non-existent section should throw
        var ex = Assert.Throws<InvalidOperationException>(() =>
            MemoryTools.EditMemory(
                uri,
                "replace_section",
                "## Methodology\n\nNew content with [[test]].",
                sectionName: "Methodology"
            )
        );

        // Verify error message is clear
        Assert.That(ex.Message, Does.Contain("Methodology"));
    }

    [Test]
    public void ReplaceSection_WithH1Heading_ReplacesCorrectly()
    {
        // Arrange: Test with top-level H1 section
        var testTitle = "H1 Replace Test";
        var testContent = @"# First Section

Content of first section with [[alpha]].

# Second Section

Content of second section with [[beta]].

# Third Section

Content of third section with [[gamma]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace the Second Section (H1)
        var newSection = @"# Second Section

REPLACED content with [[delta]] and [[epsilon]].";

        var editResult = MemoryTools.EditMemory(
            uri,
            "replace_section",
            newSection,
            sectionName: "Second Section"
        );

        // Assert
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("REPLACED content"));
        Assert.That(readResult, Does.Contain("[[delta]]"));
        Assert.That(readResult, Does.Not.Contain("[[beta]]"));
        Assert.That(readResult, Does.Contain("[[alpha]]"));  // First section untouched
        Assert.That(readResult, Does.Contain("[[gamma]]"));  // Third section untouched
    }

    [Test]
    public void ReplaceSection_WithH3Heading_ReplacesCorrectly()
    {
        // Arrange: Test with nested H3 section
        var testTitle = "H3 Replace Test";
        var testContent = @"## Main Section

Main content with [[primary]].

### Subsection A

Old subsection A content with [[old-concept]].

### Subsection B

Subsection B content with [[secondary]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace Subsection A (H3)
        var newSection = @"### Subsection A

New subsection A content with [[new-concept]] and [[innovative-idea]].";

        var editResult = MemoryTools.EditMemory(
            uri,
            "replace_section",
            newSection,
            sectionName: "Subsection A"
        );

        // Assert
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("New subsection A content"));
        Assert.That(readResult, Does.Contain("[[new-concept]]"));
        Assert.That(readResult, Does.Not.Contain("[[old-concept]]"));
        Assert.That(readResult, Does.Contain("[[primary]]"));    // Main section untouched
        Assert.That(readResult, Does.Contain("[[secondary]]"));  // Subsection B untouched
    }

    [Test]
    public void ReplaceSection_OldContentCompletelyRemoved_NotAppended()
    {
        // Arrange: This test specifically verifies old content is removed, not appended
        var testTitle = "Complete Removal Test";
        var testContent = @"# Test

Initial content with [[initial]].

## Target Section

Old paragraph one with [[old-one]].
Old paragraph two with [[old-two]].
Old paragraph three with [[old-three]].

## Next Section

Next section content with [[next]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace Target Section with completely different content
        var newSection = @"## Target Section

Brand new single paragraph with [[brand-new]].";

        var editResult = MemoryTools.EditMemory(
            uri,
            "replace_section",
            newSection,
            sectionName: "Target Section"
        );

        // Assert: Verify NONE of the old content remains
        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("Brand new single paragraph"));
        Assert.That(readResult, Does.Contain("[[brand-new]]"));

        // Critical: OLD content must be completely gone
        Assert.That(readResult, Does.Not.Contain("Old paragraph one"));
        Assert.That(readResult, Does.Not.Contain("Old paragraph two"));
        Assert.That(readResult, Does.Not.Contain("Old paragraph three"));
        Assert.That(readResult, Does.Not.Contain("[[old-one]]"));
        Assert.That(readResult, Does.Not.Contain("[[old-two]]"));
        Assert.That(readResult, Does.Not.Contain("[[old-three]]"));

        // Verify other sections are intact
        Assert.That(readResult, Does.Contain("[[initial]]"));
        Assert.That(readResult, Does.Contain("[[next]]"));
    }

    // =============================================================================
    // FIX #3: WikiLink Edge Case Tests
    // =============================================================================

    [Test]
    public void FindReplace_ReplacingTextInsideWikiLink_CreatesNestedBrackets()
    {
        // Arrange: Test the edge case of replacing text INSIDE a WikiLink
        var testTitle = "WikiLink Nested Brackets Test";
        var testContent = "This file has a [[Machine Learning]] concept.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace "Learning" (which is INSIDE the WikiLink) with "[[Deep Learning]]"
        // This creates nested brackets: [[Machine [[Deep Learning]]]]
        var editResult = MemoryTools.EditMemory(
            uri,
            "find_replace",
            testContent.Replace("Learning", "[[Deep Learning]]"),
            findText: "Learning",
            expectedCount: 1
        );

        // Assert: This should succeed (edge case is now documented)
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        // Verify the nested bracket structure exists
        Assert.That(readResult, Does.Contain("[[Machine [[Deep Learning]]]]"));
    }

    [Test]
    public void FindReplace_ReplacingEntireWikiLink_WorksCorrectly()
    {
        // Arrange: Test replacing the ENTIRE WikiLink including brackets
        var testTitle = "Replace Entire WikiLink Test";
        var testContent = "This file references [[Machine Learning]] and [[Deep Learning]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace the entire "[[Machine Learning]]" with "[[Artificial Intelligence]]"
        var editResult = MemoryTools.EditMemory(
            uri,
            "find_replace",
            testContent.Replace("[[Machine Learning]]", "[[Artificial Intelligence]]"),
            findText: "[[Machine Learning]]",
            expectedCount: 1
        );

        // Assert: This should work fine
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("[[Artificial Intelligence]]"));
        Assert.That(readResult, Does.Not.Contain("[[Machine Learning]]"));
        Assert.That(readResult, Does.Contain("[[Deep Learning]]")); // Other WikiLink intact
    }

    [Test]
    public void FindReplace_MultipleWikiLinksWithSameInnerText_ReplacesAll()
    {
        // Arrange: Test edge case where inner text appears in multiple WikiLinks
        var testTitle = "Multiple WikiLinks Same Text Test";
        var testContent = @"We study [[Neural Networks]] and [[Convolutional Neural Networks]].
Also see [[Recurrent Neural Networks]] and [[Neural Networks Architecture]].";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace "Neural Networks" (appears in 4 WikiLinks)
        var editResult = MemoryTools.EditMemory(
            uri,
            "find_replace",
            testContent.Replace("Neural Networks", "NN"),
            findText: "Neural Networks",
            expectedCount: 4
        );

        // Assert
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("[[NN]]"));
        Assert.That(readResult, Does.Contain("[[Convolutional NN]]"));
        Assert.That(readResult, Does.Contain("[[Recurrent NN]]"));
        Assert.That(readResult, Does.Contain("[[NN Architecture]]"));
        Assert.That(readResult, Does.Not.Contain("Neural Networks"));
    }

    [Test]
    public void FindReplace_ReplacingWithMultipleWikiLinks_WorksCorrectly()
    {
        // Arrange: Test replacing text with content containing multiple WikiLinks
        var testTitle = "Multiple WikiLinks Replace Test";
        var testContent = "This topic covers [[AI]] fundamentals.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        var uri = ExtractUri(writeResult);

        // Act: Replace "fundamentals" with multiple WikiLinks
        var editResult = MemoryTools.EditMemory(
            uri,
            "find_replace",
            testContent.Replace("fundamentals", "[[Machine Learning]], [[Deep Learning]], and [[Neural Networks]]"),
            findText: "fundamentals",
            expectedCount: 1
        );

        // Assert
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("[[Machine Learning]]"));
        Assert.That(readResult, Does.Contain("[[Deep Learning]]"));
        Assert.That(readResult, Does.Contain("[[Neural Networks]]"));
    }
}
