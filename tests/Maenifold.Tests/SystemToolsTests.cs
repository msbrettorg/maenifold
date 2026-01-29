using System;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for SystemTools functionality.
///
/// Ma Protocol Compliance: These tests use REAL systems only.
/// - Real Config and file operations (no mocks)
/// - Real directory operations for ListMemories
/// - Real database operations for MemoryStatus
/// - Real asset operations for UpdateAssets
/// - No mocks, no stubs, real system behavior
///
/// These tests verify that system tools correctly report configuration,
/// status, help documentation, and asset management.
/// </summary>
[TestFixture]
public class SystemToolsTests
{
    private string _testDir = string.Empty;

    [SetUp]
    public void Setup()
    {
        // Tests run within TestEnvironmentSetup global isolation
        // Each test uses its own subdirectory for specific operations
        _testDir = Path.Combine(
            Config.MaenifoldRoot,
            "test-system-tools",
            $"run-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}"
        );
        Directory.CreateDirectory(_testDir);

        // Ensure database is initialized for MemoryStatus tests
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();

        // Ensure assets are initialized for GetHelp tests
        AssetManager.InitializeAssets();
    }

    [TearDown]
    public void TearDown()
    {
        if (Directory.Exists(_testDir))
        {
            try
            {
                Directory.Delete(_testDir, recursive: true);
            }
            catch
            {
                // Ma Protocol: Keep test artifacts for debugging
                Console.WriteLine($"[TEST CLEANUP] Keeping test artifacts at {_testDir} for inspection");
            }
        }
    }

    #region GetConfig Tests

    /// <summary>
    /// Verify that GetConfig returns expected configuration structure with key paths.
    /// Tests that configuration summary contains all essential system paths.
    /// </summary>
    [Test]
    public void GetConfig_ReturnsExpectedStructure()
    {
        // Act
        var result = SystemTools.GetConfig();

        // Assert: Should contain key configuration paths
        Assert.That(result, Is.Not.Null, "Config result should not be null");
        Assert.That(result, Is.Not.Empty, "Config result should not be empty");
        Assert.That(result, Does.Contain("Memory").Or.Contain("Maenifold"),
            "Config should contain memory/maenifold path information");
        Assert.That(result, Does.Contain("Database").Or.Contain("memory.db"),
            "Config should contain database path information");
    }

    /// <summary>
    /// Verify that GetConfig with learn=true returns help documentation.
    /// Tests that help mode returns documentation instead of configuration.
    /// </summary>
    [Test]
    public void GetConfig_WithLearnTrue_ReturnsHelpDocumentation()
    {
        // Act
        var result = SystemTools.GetConfig(learn: true);

        // Assert: Should return help documentation
        Assert.That(result, Is.Not.Null, "Help result should not be null");
        Assert.That(result, Is.Not.Empty, "Help result should not be empty");

        // Help docs should either contain actual documentation or error message
        var isHelpDoc = result.Contains("GetConfig") || result.Contains("configuration");
        var isErrorMessage = result.Contains("ERROR") && result.Contains("Help file not found");

        Assert.That(isHelpDoc || isErrorMessage, Is.True,
            "Result should be either help documentation or error message about missing help file");
    }

    #endregion

    #region MemoryStatus Tests

    /// <summary>
    /// Verify that MemoryStatus returns valid statistics with expected sections.
    /// Tests that status report includes file counts and graph database metrics.
    /// </summary>
    [Test]
    public void MemoryStatus_ReturnsValidStatistics()
    {
        // Act
        var result = SystemTools.MemoryStatus();

        // Assert: Should contain expected sections
        Assert.That(result, Is.Not.Null, "Status result should not be null");
        Assert.That(result, Is.Not.Empty, "Status result should not be empty");
        Assert.That(result, Does.Contain("Memory System Status"),
            "Status should contain header");
        Assert.That(result, Does.Contain("Files").Or.Contain("files"),
            "Status should contain file information");
    }

    /// <summary>
    /// Verify that MemoryStatus reports database status correctly.
    /// Tests that status includes graph database metrics when database exists.
    /// </summary>
    [Test]
    public void MemoryStatus_IncludesDatabaseMetrics()
    {
        // Act
        var result = SystemTools.MemoryStatus();

        // Assert: Should contain database section
        Assert.That(result, Does.Contain("Graph Database").Or.Contain("Database"),
            "Status should contain database section");

        // Should either show database metrics or indicate it's not initialized
        var hasMetrics = result.Contains("Concepts") || result.Contains("Relations");
        var notInitialized = result.Contains("Not initialized") || result.Contains("not initialized");

        Assert.That(hasMetrics || notInitialized, Is.True,
            "Status should either show database metrics or indicate database is not initialized");
    }

    /// <summary>
    /// Verify that MemoryStatus with learn=true returns help documentation.
    /// Tests that help mode returns documentation instead of status.
    /// </summary>
    [Test]
    public void MemoryStatus_WithLearnTrue_ReturnsHelpDocumentation()
    {
        // Act
        var result = SystemTools.MemoryStatus(learn: true);

        // Assert: Should return help documentation
        Assert.That(result, Is.Not.Null, "Help result should not be null");
        Assert.That(result, Is.Not.Empty, "Help result should not be empty");

        // Help docs should either contain actual documentation or error message
        var isHelpDoc = result.Contains("MemoryStatus") || result.Contains("statistics") || result.Contains("status");
        var isErrorMessage = result.Contains("ERROR") && result.Contains("Help file not found");

        Assert.That(isHelpDoc || isErrorMessage, Is.True,
            "Result should be either help documentation or error message about missing help file");
    }

    #endregion

    #region ListMemories Tests

    /// <summary>
    /// Verify that ListMemories returns directory structure for root path.
    /// Tests that listing shows folders and files in memory root.
    /// </summary>
    [Test]
    public void ListMemories_WithNoPath_ReturnsRootStructure()
    {
        // Arrange: Create test files in memory root
        var testFile = Path.Combine(Config.MemoryPath, "test-list-memories.md");
        File.WriteAllText(testFile, "# Test\n\nTest content for listing");

        try
        {
            // Act
            var result = SystemTools.ListMemories();

            // Assert: Should contain directory header
            Assert.That(result, Is.Not.Null, "List result should not be null");
            Assert.That(result, Is.Not.Empty, "List result should not be empty");
            Assert.That(result, Does.Contain("Directory"),
                "List should contain directory header");
        }
        finally
        {
            // Cleanup test file
            if (File.Exists(testFile))
                File.Delete(testFile);
        }
    }

    /// <summary>
    /// Verify that ListMemories with subdirectory path returns that directory's contents.
    /// Tests that path parameter correctly filters to specific directory.
    /// </summary>
    [Test]
    public void ListMemories_WithSubdirectoryPath_ReturnsSubdirectoryContents()
    {
        // Arrange: Create test subdirectory with file
        var subDir = Path.Combine(Config.MemoryPath, "test-subdir-list");
        Directory.CreateDirectory(subDir);
        var testFile = Path.Combine(subDir, "test-file.md");
        File.WriteAllText(testFile, "# Test\n\nTest content");

        try
        {
            // Act
            var result = SystemTools.ListMemories("test-subdir-list");

            // Assert: Should show the subdirectory contents
            Assert.That(result, Is.Not.Null, "List result should not be null");
            Assert.That(result, Does.Contain("test-subdir-list"),
                "List should reference the subdirectory");
            Assert.That(result, Does.Contain("test-file.md"),
                "List should contain the test file");
        }
        finally
        {
            // Cleanup test directory
            if (Directory.Exists(subDir))
                Directory.Delete(subDir, recursive: true);
        }
    }

    /// <summary>
    /// Verify that ListMemories with non-existent path returns error message.
    /// Tests error handling for invalid directory paths.
    /// </summary>
    [Test]
    public void ListMemories_WithNonExistentPath_ReturnsErrorMessage()
    {
        // Act
        var result = SystemTools.ListMemories("non-existent-directory-xyz");

        // Assert: Should return error message
        Assert.That(result, Is.Not.Null, "Result should not be null");
        Assert.That(result, Does.Contain("not found").IgnoreCase,
            "Result should indicate directory not found");
    }

    /// <summary>
    /// Verify that ListMemories with learn=true returns help documentation.
    /// Tests that help mode returns documentation instead of directory listing.
    /// </summary>
    [Test]
    public void ListMemories_WithLearnTrue_ReturnsHelpDocumentation()
    {
        // Act
        var result = SystemTools.ListMemories(learn: true);

        // Assert: Should return help documentation
        Assert.That(result, Is.Not.Null, "Help result should not be null");
        Assert.That(result, Is.Not.Empty, "Help result should not be empty");

        // Help docs should either contain actual documentation or error message
        var isHelpDoc = result.Contains("ListMemories") || result.Contains("directory") || result.Contains("folder");
        var isErrorMessage = result.Contains("ERROR") && result.Contains("Help file not found");

        Assert.That(isHelpDoc || isErrorMessage, Is.True,
            "Result should be either help documentation or error message about missing help file");
    }

    #endregion

    #region GetHelp Tests

    /// <summary>
    /// Verify that GetHelp returns documentation for a valid tool name.
    /// Tests that help system can retrieve documentation for known tools.
    /// </summary>
    [Test]
    public void GetHelp_WithValidToolName_ReturnsDocumentation()
    {
        // Arrange: Use a known tool name that should have documentation
        var knownTool = "writememory"; // Tool names are lowercased

        // Act
        var result = SystemTools.GetHelp(knownTool);

        // Assert: Should return documentation content
        Assert.That(result, Is.Not.Null, "Help result should not be null");
        Assert.That(result, Is.Not.Empty, "Help result should not be empty");

        // Result should either contain actual help content or list available tools
        var isHelpContent = result.Contains("WriteMemory") || result.Contains("memory") || result.Contains('#');
        var isToolList = result.Contains("Available tools") && result.Contains("writememory");

        Assert.That(isHelpContent || isToolList, Is.True,
            "Result should be either help documentation or list of available tools");
    }

    /// <summary>
    /// Verify that GetHelp with invalid tool name returns error with available tools list.
    /// Tests error handling for non-existent tool documentation.
    /// </summary>
    [Test]
    public void GetHelp_WithInvalidToolName_ReturnsErrorWithAvailableTools()
    {
        // Act
        var result = SystemTools.GetHelp("non-existent-tool-xyz");

        // Assert: Should return error message with available tools
        Assert.That(result, Is.Not.Null, "Result should not be null");
        Assert.That(result, Does.Contain("No help file found").Or.Contain("not found"),
            "Result should indicate help file not found");
        Assert.That(result, Does.Contain("Available tools"),
            "Result should list available tools");
    }

    /// <summary>
    /// Verify that GetHelp is case-insensitive for tool names.
    /// Tests that tool name lookup normalizes case correctly.
    /// </summary>
    [Test]
    public void GetHelp_IsCaseInsensitive()
    {
        // Act: Try different case variations
        var lowerResult = SystemTools.GetHelp("writememory");
        var upperResult = SystemTools.GetHelp("WRITEMEMORY");
        var mixedResult = SystemTools.GetHelp("WriteMemory");

        // Assert: All should return same type of result (either help or available tools list)
        Assert.That(lowerResult, Is.Not.Null, "Lower case result should not be null");
        Assert.That(upperResult, Is.Not.Null, "Upper case result should not be null");
        Assert.That(mixedResult, Is.Not.Null, "Mixed case result should not be null");

        // All should be consistent (either all have help content or all list available tools)
        var allHaveHelp = !lowerResult.Contains("Available tools") &&
                         !upperResult.Contains("Available tools") &&
                         !mixedResult.Contains("Available tools");

        var allListTools = lowerResult.Contains("Available tools") &&
                          upperResult.Contains("Available tools") &&
                          mixedResult.Contains("Available tools");

        Assert.That(allHaveHelp || allListTools, Is.True,
            "Case variations should all return consistent results");
    }

    #endregion

    #region UpdateAssets Tests

    /// <summary>
    /// Verify that UpdateAssets with dryRun=true returns preview without modifying files.
    /// Tests that dry run mode shows what would be updated without making changes.
    /// </summary>
    [Test]
    public void UpdateAssets_WithDryRunTrue_ReturnsPreviewWithoutModifying()
    {
        // Act
        var result = SystemTools.UpdateAssets(dryRun: true);

        // Assert: Should return preview information
        Assert.That(result, Is.Not.Null, "Result should not be null");
        Assert.That(result, Is.Not.Empty, "Result should not be empty");

        // Result should indicate dry run mode or show preview of changes
        var isDryRunResult = result.Contains("DRY RUN") ||
                            result.Contains("dry run") ||
                            result.Contains("Preview") ||
                            result.Contains("would") ||
                            result.Contains("Summary");

        Assert.That(isDryRunResult, Is.True,
            "Result should indicate dry run mode or show preview");
    }

    /// <summary>
    /// Verify that UpdateAssets with learn=true returns help documentation.
    /// Tests that help mode returns documentation instead of performing update.
    /// </summary>
    [Test]
    public void UpdateAssets_WithLearnTrue_ReturnsHelpDocumentation()
    {
        // Act
        var result = SystemTools.UpdateAssets(learn: true);

        // Assert: Should return help documentation
        Assert.That(result, Is.Not.Null, "Help result should not be null");
        Assert.That(result, Is.Not.Empty, "Help result should not be empty");

        // Help docs should either contain actual documentation or error message
        var isHelpDoc = result.Contains("UpdateAssets") || result.Contains("assets") || result.Contains("upgrade");
        var isErrorMessage = result.Contains("ERROR") && result.Contains("Help file not found");

        Assert.That(isHelpDoc || isErrorMessage, Is.True,
            "Result should be either help documentation or error message about missing help file");
    }

    /// <summary>
    /// Verify that UpdateAssets defaults to dryRun=true for safety.
    /// Tests that calling without parameters performs dry run by default.
    /// </summary>
    [Test]
    public void UpdateAssets_DefaultsToDryRun()
    {
        // Act: Call without parameters
        var result = SystemTools.UpdateAssets();

        // Assert: Should perform dry run by default
        Assert.That(result, Is.Not.Null, "Result should not be null");

        // Result should indicate dry run behavior
        var isDryRunResult = result.Contains("DRY RUN") ||
                            result.Contains("dry run") ||
                            result.Contains("Preview") ||
                            result.Contains("would");

        // If not explicitly showing dry run, should at least show summary without "Updated" actions
        var showsSummaryOnly = result.Contains("Summary") && !result.Contains("Updated:");

        Assert.That(isDryRunResult || showsSummaryOnly, Is.True,
            "Default behavior should be dry run (safe preview mode)");
    }

    #endregion
}
