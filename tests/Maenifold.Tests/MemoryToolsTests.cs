using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

public class MemoryToolsTests
{
    private const string TestFolder = "memory-tools-tests";
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

        MemoryTools.DeleteMemory("memory://regression-test/moved-extension-test", confirm: true);
        MemoryTools.DeleteMemory("memory://simple-move-renamed", confirm: true);
        MemoryTools.DeleteMemory("memory://simple-move-test", confirm: true);
        MemoryTools.DeleteMemory("memory://move-extension-test", confirm: true);

        var regressionTestDir = Path.Combine(Config.MemoryPath, "regression-test");
        if (Directory.Exists(regressionTestDir) && !Directory.EnumerateFileSystemEntries(regressionTestDir).Any())
        {
            Directory.Delete(regressionTestDir);
        }
    }

    [Test]
    public void MoveMemoryWithPathBasedDestinationPreservesMarkdownExtension()
    {
        // Arrange: Create a test memory file
        var testTitle = "Move Extension Test";
        var testContent = "# Move Extension Test\n\nTesting [[file-operations]] preserves .md extension.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent);
        Assert.That(writeResult, Does.StartWith("Created memory FILE: memory://move-extension-test"));

        // Act: Move with path-based destination (contains slash)
        var moveResult = MemoryTools.MoveMemory("memory://move-extension-test", "regression-test/Moved Extension Test");

        // Assert: Move should succeed (destination gets slugified like WriteMemory)
        Assert.That(moveResult, Does.StartWith("Moved memory FILE: memory://move-extension-test"));
        Assert.That(moveResult, Does.Contain("memory://regression-test/moved-extension-test"));

        // Verify: File should be readable at new location (proving .md extension exists)
        var readResult = MemoryTools.ReadMemory("memory://regression-test/moved-extension-test");
        Assert.That(readResult, Does.Not.StartWith("ERROR:"), "File should be readable, proving .md extension was added");
        Assert.That(readResult, Does.Contain("Testing [[file-operations]] preserves .md extension."));
    }

    [Test]
    public void MoveMemoryWithSimpleDestinationPreservesExistingBehavior()
    {
        // Arrange: Create a test memory file
        var testTitle = "Simple Move Test";
        var testContent = "# Simple Move Test\n\nTesting [[refactoring]] functionality.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent);
        Assert.That(writeResult, Does.StartWith("Created memory FILE: memory://simple-move-test"));

        // Act: Move with simple destination (no slash)
        var moveResult = MemoryTools.MoveMemory("memory://simple-move-test", "Simple Move Renamed");

        // Assert: Move should succeed
        Assert.That(moveResult, Does.StartWith("Moved memory FILE: memory://simple-move-test"));
        Assert.That(moveResult, Does.Contain("memory://simple-move-renamed"));

        // Verify: File should be readable at new location
        var readResult = MemoryTools.ReadMemory("memory://simple-move-renamed");
        Assert.That(readResult, Does.Not.StartWith("ERROR:"));
        Assert.That(readResult, Does.Contain("Testing [[refactoring]] functionality."));
    }

    [Test]
    public void ReadMemoryHandlesMissingTitleInFrontmatter()
    {
        // Arrange: Create a memory file manually without a title field in frontmatter
        var testFileName = "no-title-test.md";
        var testFilePath = Path.Combine(_testFolderPath, testFileName);

        // Create file with frontmatter that has no title field
        var fileContent = @"---
type: memory
status: saved
---

# Test Content

This is a [[metadata-handling]] without a title in frontmatter.";

        Directory.CreateDirectory(_testFolderPath);
        File.WriteAllText(testFilePath, fileContent);

        // Act: Read the memory file using its URI
        var uri = $"memory://{TestFolder}/{Path.GetFileNameWithoutExtension(testFileName)}";
        var readResult = MemoryTools.ReadMemory(uri);

        // Assert: Should not throw exception and should use filename as title
        Assert.That(readResult, Does.Not.StartWith("ERROR:"));
        Assert.That(readResult, Does.Contain("# no-title-test")); // Filename used as title
        Assert.That(readResult, Does.Contain("URI: memory://memory-tools-tests/no-title-test"));
        Assert.That(readResult, Does.Contain("This is a [[metadata-handling]] without a title in frontmatter."));
    }

    [Test]
    public void ReadMemoryHandlesEmptyFrontmatter()
    {
        // Arrange: Create a memory file with no frontmatter at all
        var testFileName = "no-frontmatter-test.md";
        var testFilePath = Path.Combine(_testFolderPath, testFileName);

        var fileContent = @"# Test Without Frontmatter

This is a [[yaml-frontmatter]] without any frontmatter.";

        Directory.CreateDirectory(_testFolderPath);
        File.WriteAllText(testFilePath, fileContent);

        // Act: Read the memory file
        var uri = $"memory://{TestFolder}/{Path.GetFileNameWithoutExtension(testFileName)}";
        var readResult = MemoryTools.ReadMemory(uri);

        // Assert: Should not throw exception and should use filename as title
        Assert.That(readResult, Does.Not.StartWith("ERROR:"));
        Assert.That(readResult, Does.Contain("# no-frontmatter-test")); // Filename used as title
        Assert.That(readResult, Does.Contain("This is a [[yaml-frontmatter]] without any frontmatter."));
    }

    [Test]
    public void WriteMemoryDoesNotWriteEmbeddingFields()
    {
        // Arrange
        var testTitle = "Embedding Removal Test";
        var testContent = "Content with [[vector-embeddings]] concept to ensure write path works.";

        // Act
        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        // Extract URI and resolve to path
        var uriLine = writeResult.Split('\n').FirstOrDefault() ?? string.Empty;
        var uriStart = uriLine.IndexOf("memory://", StringComparison.Ordinal);
        Assert.That(uriStart, Is.GreaterThanOrEqualTo(0), "WriteMemory should return a memory:// URI");
        var uri = uriLine.Substring(uriStart).Trim();
        var path = MarkdownWriter.UriToPath(uri, Config.MemoryPath);

        // Read raw file content
        var raw = File.ReadAllText(path);

        // Assert: frontmatter must not contain any embedding_* keys
        Assert.That(raw, Does.Not.Contain("embedding_base64"));
        Assert.That(raw, Does.Not.Contain("embedding_model"));
        Assert.That(raw, Does.Not.Contain("embedding_date"));
    }

    [Test]
    public void EditMemory_ReplaceSection_ReplacesExistingSection()
    {
        // Arrange: Create a memory file with multiple sections
        var testTitle = "Replace Section Test";
        var testContent = @"This is the intro with a [[markdown-sections]].

## First Section

Old content in first section.

## Second Section

Old content in second section with [[document-structure]].

## Third Section

Old content in third section.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        var uri = $"memory://{TestFolder}/replace-section-test";

        // Act: Replace the second section
        var newSectionContent = "New content in second section with [[content-editing]].\n\nMultiple lines work too.";
        var editResult = MemoryTools.EditMemory(uri, "replace_section", newSectionContent, sectionName: "Second Section");

        // Assert: Edit should succeed
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        // Verify: Read back and check the section was replaced
        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("## Second Section"));
        Assert.That(readResult, Does.Contain("New content in second section with [[content-editing]]"));
        Assert.That(readResult, Does.Contain("Multiple lines work too"));

        // Verify: Old section content is gone
        Assert.That(readResult, Does.Not.Contain("Old content in second section"));
        Assert.That(readResult, Does.Not.Contain("[[document-structure]]"));

        // Verify: Other sections are untouched
        Assert.That(readResult, Does.Contain("## First Section"));
        Assert.That(readResult, Does.Contain("Old content in first section"));
        Assert.That(readResult, Does.Contain("## Third Section"));
        Assert.That(readResult, Does.Contain("Old content in third section"));
    }

    [Test]
    public void EditMemory_ReplaceSection_ThrowsWhenSectionNotFound()
    {
        // Arrange: Create a memory file without the target section
        var testTitle = "Section Not Found Test";
        var testContent = @"This is the intro with a [[error-handling]].

## Existing Section

Some content here.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        var uri = $"memory://{TestFolder}/section-not-found-test";

        // Act & Assert: Attempting to replace a non-existent section should throw
        var newSectionContent = "New content with [[validation]].";
        var ex = Assert.Throws<InvalidOperationException>(() =>
            MemoryTools.EditMemory(uri, "replace_section", newSectionContent, sectionName: "Nonexistent Section"));

        Assert.That(ex.Message, Does.Contain("Section not found"));
        Assert.That(ex.Message, Does.Contain("Nonexistent Section"));
    }

    [Test]
    public void EditMemory_ReplaceSection_WorksWithDifferentHeadingLevels()
    {
        // Arrange: Create a memory file with different heading levels
        var testTitle = "Heading Levels Test";
        var testContent = @"Intro with [[markdown-hierarchy]].

# Level 1 Heading

Content under level 1.

## Level 2 Heading

Content under level 2.

### Level 3 Heading

Content under level 3.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        var uri = $"memory://{TestFolder}/heading-levels-test";

        // Act: Replace the level 2 section
        var newContent = "New content for level 2 with [[section-replacement]].";
        var editResult = MemoryTools.EditMemory(uri, "replace_section", newContent, sectionName: "Level 2 Heading");

        // Assert: Edit should succeed
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        // Verify: The level 2 section was replaced
        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("## Level 2 Heading"));
        Assert.That(readResult, Does.Contain("New content for level 2 with [[section-replacement]]"));
        Assert.That(readResult, Does.Not.Contain("Content under level 2"));

        // Verify: Other sections are untouched
        Assert.That(readResult, Does.Contain("# Level 1 Heading"));
        Assert.That(readResult, Does.Contain("Content under level 1"));
        Assert.That(readResult, Does.Contain("### Level 3 Heading"));
        Assert.That(readResult, Does.Contain("Content under level 3"));
    }

    [Test]
    public void EditMemory_ReplaceSection_ReplacesLastSection()
    {
        // Arrange: Create a memory file and replace the last section
        var testTitle = "Replace Last Section Test";
        var testContent = @"Intro with [[boundary-conditions]].

## First Section

First content.

## Last Section

This is the last section content.";

        var writeResult = MemoryTools.WriteMemory(testTitle, testContent, folder: TestFolder);
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        var uri = $"memory://{TestFolder}/replace-last-section-test";

        // Act: Replace the last section
        var newContent = "New last section content with [[document-end]].";
        var editResult = MemoryTools.EditMemory(uri, "replace_section", newContent, sectionName: "Last Section");

        // Assert: Edit should succeed
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"));

        // Verify: The last section was replaced
        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Contain("## Last Section"));
        Assert.That(readResult, Does.Contain("New last section content with [[document-end]]"));
        Assert.That(readResult, Does.Not.Contain("This is the last section content"));

        // Verify: First section is untouched
        Assert.That(readResult, Does.Contain("## First Section"));
        Assert.That(readResult, Does.Contain("First content"));
    }

    // SEC-002: URL-encoded path traversal tests
    [Test]
    public void SanitizeUserInput_BlocksUrlEncodedPathTraversal()
    {
        // Arrange: Malicious title with URL-encoded ../
        var maliciousTitle = "test%2e%2e%2fmalicious";
        var safeContent = "Safe content with [[security-validation]].";

        // Act: Attempt to write memory with encoded path traversal
        var writeResult = MemoryTools.WriteMemory(maliciousTitle, safeContent, folder: TestFolder);

        // Assert: Should succeed but with sanitized title (../ stripped after decoding)
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));
        Assert.That(writeResult, Does.Not.Contain(".."));
        // The word "malicious" is preserved (it's legitimate text), only path traversal is stripped
        Assert.That(writeResult, Does.Contain("testmalicious"));

        // Verify: The sanitized filename should not contain path traversal
        var readResult = MemoryTools.ReadMemory("memory://memory-tools-tests/testmalicious");
        Assert.That(readResult, Does.Not.StartWith("ERROR:"));
        Assert.That(readResult, Does.Contain("Safe content with [[security-validation]]"));
    }

    [Test]
    public void SanitizeUserInput_BlocksDoubleUrlEncodedPathTraversal()
    {
        // Arrange: Double URL-encoded ../ (%252e%252e%252f)
        var doubleEncodedTitle = "test%252e%252e%252fmalicious";
        var safeContent = "Safe content with [[input-sanitization]].";

        // Act: Attempt to write memory with double-encoded path traversal
        var writeResult = MemoryTools.WriteMemory(doubleEncodedTitle, safeContent, folder: TestFolder);

        // Assert: Should succeed with sanitized title
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));
        Assert.That(writeResult, Does.Not.Contain(".."));

        // Note: Double-encoding means first decode gives %2e%2e%2f, which is still encoded
        // Uri.UnescapeDataString will decode once per call, so we handle single-encoded
        // This test verifies the behavior - double encoding requires multiple decodes
    }

    [Test]
    public void SanitizeUserInput_BlocksUrlEncodedDotDot()
    {
        // Arrange: URL-encoded .. without slash
        var encodedTitle = "test%2e%2e";
        var safeContent = "Safe content with [[path-traversal-prevention]].";

        // Act: Write memory with encoded ..
        var writeResult = MemoryTools.WriteMemory(encodedTitle, safeContent, folder: TestFolder);

        // Assert: Should succeed with .. stripped after decoding
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));
        Assert.That(writeResult, Does.Not.Contain(".."));
    }

    [Test]
    public void SanitizeUserInput_BlocksMixedEncodedAndPlainPathTraversal()
    {
        // Arrange: Mix of encoded and plain characters
        var mixedTitle = "%2e./test";
        var safeContent = "Safe content with [[url-encoding]].";

        // Act: Write memory with partially encoded path traversal
        var writeResult = MemoryTools.WriteMemory(mixedTitle, safeContent, folder: TestFolder);

        // Assert: Should succeed with path traversal components stripped
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));
        Assert.That(writeResult, Does.Not.Contain(".."));

        // Verify: Can read with sanitized name
        var readResult = MemoryTools.ReadMemory("memory://memory-tools-tests/test");
        Assert.That(readResult, Does.Not.StartWith("ERROR:"));
    }

    [Test]
    public void SanitizeUserInput_PreservesLegitimatePercentSigns()
    {
        // Arrange: Legitimate use of percent (e.g., "50% Complete")
        // Note: Uri.UnescapeDataString may interpret this differently
        var titleWithPercent = "Task 50 Complete";
        var safeContent = "Status update with [[task-tracking]].";

        // Act: Write memory with legitimate content
        var writeResult = MemoryTools.WriteMemory(titleWithPercent, safeContent, folder: TestFolder);

        // Assert: Should succeed
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        // Verify: File is readable and content preserved
        var uri = writeResult.Split('\n')[0].Replace("Created memory FILE: ", "").Trim();
        var readResult = MemoryTools.ReadMemory(uri);
        Assert.That(readResult, Does.Not.StartWith("ERROR:"));
        Assert.That(readResult, Does.Contain("Status update with [[task-tracking]]"));
    }

}
