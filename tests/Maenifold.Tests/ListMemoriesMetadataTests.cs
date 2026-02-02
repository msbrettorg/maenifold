using System;
using System.Globalization;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for ListMemories decay metadata output (FR-7.7, NFR-7.7.1).
///
/// T-GRAPH-DECAY-003.1: ListMemories SHALL display created, last_accessed, decay_weight for each file.
/// T-GRAPH-DECAY-003.2: decay_weight SHALL use file's tier (sequential=7d, workflows=14d, other=14d grace).
///
/// Ma Protocol Compliance: These tests use REAL systems only.
/// - Real Config and file operations (no mocks)
/// - Real directory operations for ListMemories
/// - No mocks, no stubs, real system behavior
///
/// TDD Red Phase: These tests will fail until ListMemories is updated to include decay metadata.
/// </summary>
[TestFixture]
public class ListMemoriesMetadataTests
{
    private string _testDir = string.Empty;
    private string _sequentialDir = string.Empty;
    private string _workflowsDir = string.Empty;
    private string _standardDir = string.Empty;

    [SetUp]
    public void Setup()
    {
        // Tests run within TestEnvironmentSetup global isolation
        Config.EnsureDirectories();

        // Create test directories for different tiers
        _testDir = Path.Combine(Config.MemoryPath, "decay-metadata-tests");
        _sequentialDir = Path.Combine(Config.MemoryPath, "thinking", "sequential", "decay-test");
        _workflowsDir = Path.Combine(Config.MemoryPath, "thinking", "workflows", "decay-test");
        _standardDir = Path.Combine(_testDir, "standard");

        Directory.CreateDirectory(_testDir);
        Directory.CreateDirectory(_sequentialDir);
        Directory.CreateDirectory(_workflowsDir);
        Directory.CreateDirectory(_standardDir);
    }

    [TearDown]
    public void TearDown()
    {
        // Cleanup test directories
        CleanupDirectory(_testDir);
        CleanupDirectory(_sequentialDir);
        CleanupDirectory(_workflowsDir);
    }

    private static void CleanupDirectory(string path)
    {
        if (Directory.Exists(path))
        {
            try
            {
                Directory.Delete(path, recursive: true);
            }
            catch
            {
                // Ma Protocol: Keep test artifacts for debugging if cleanup fails
                Console.WriteLine($"[TEST CLEANUP] Keeping test artifacts at {path} for inspection");
            }
        }
    }

    #region T-GRAPH-DECAY-003.1: RTM FR-7.7 - ListMemories decay metadata display

    /// <summary>
    /// T-GRAPH-DECAY-003.1: ListMemories output SHALL include created timestamp.
    /// Verifies FR-7.7 requirement for created metadata display.
    /// </summary>
    [Test]
    public void ListMemories_Output_IncludesCreatedTimestamp()
    {
        // Arrange: Create a test file with explicit created timestamp in frontmatter
        var testFile = Path.Combine(_standardDir, "created-test.md");
        var createdDate = "2026-01-15T10:30:00Z";
        var content = $@"---
title: Created Test
type: memory
status: saved
created: {createdDate}
modified: {createdDate}
---

# Created Test

Test content for [[decay-metadata]] created timestamp verification.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("decay-metadata-tests/standard");

        // Assert: Output should include created timestamp
        Assert.That(result, Does.Contain("created"),
            "ListMemories output should include 'created' field label");
        Assert.That(result, Does.Contain("2026-01-15").Or.Contain(createdDate),
            "ListMemories output should include the created date value");
    }

    /// <summary>
    /// T-GRAPH-DECAY-003.1: ListMemories output SHALL include last_accessed timestamp.
    /// Verifies FR-7.7 requirement for last_accessed metadata display.
    /// </summary>
    [Test]
    public void ListMemories_Output_IncludesLastAccessedTimestamp()
    {
        // Arrange: Create a test file with last_accessed timestamp in frontmatter
        var testFile = Path.Combine(_standardDir, "accessed-test.md");
        var createdDate = "2026-01-10T08:00:00Z";
        var lastAccessedDate = "2026-01-20T14:45:00Z";
        var content = $@"---
title: Accessed Test
type: memory
status: saved
created: {createdDate}
modified: {createdDate}
last_accessed: {lastAccessedDate}
---

# Accessed Test

Test content for [[decay-metadata]] last_accessed timestamp verification.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("decay-metadata-tests/standard");

        // Assert: Output should include last_accessed timestamp
        Assert.That(result, Does.Contain("last_accessed").Or.Contain("last accessed"),
            "ListMemories output should include 'last_accessed' field label");
        Assert.That(result, Does.Contain("2026-01-20").Or.Contain(lastAccessedDate),
            "ListMemories output should include the last_accessed date value");
    }

    /// <summary>
    /// T-GRAPH-DECAY-003.1: ListMemories output SHALL include decay_weight in range [0.0, 1.0].
    /// Verifies FR-7.7 requirement for decay_weight metadata display.
    /// </summary>
    [Test]
    public void ListMemories_Output_IncludesDecayWeight()
    {
        // Arrange: Create a test file
        var testFile = Path.Combine(_standardDir, "decay-weight-test.md");
        var createdDate = DateTime.UtcNow.AddDays(-7).ToString("o", CultureInfo.InvariantCulture);
        var content = $@"---
title: Decay Weight Test
type: memory
status: saved
created: {createdDate}
modified: {createdDate}
---

# Decay Weight Test

Test content for [[decay-metadata]] decay_weight verification.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("decay-metadata-tests/standard");

        // Assert: Output should include decay_weight
        Assert.That(result, Does.Contain("decay_weight").Or.Contain("decay weight"),
            "ListMemories output should include 'decay_weight' field label");

        // Verify decay_weight is a valid number between 0.0 and 1.0
        // Pattern matches decimal numbers like "0.85", "1.0", "0.0"
        Assert.That(result, Does.Match(@"decay[_\s]?weight[:\s]+([01]\.?\d*)"),
            "ListMemories output should include decay_weight as a decimal value");
    }

    #endregion

    #region T-GRAPH-DECAY-003.2: RTM NFR-7.7.1 - Tier-based decay calculation

    /// <summary>
    /// T-GRAPH-DECAY-003.2: Sequential file SHALL use 7d grace period for decay calculation.
    /// Verifies NFR-7.7.1 requirement for thinking/sequential/ tier.
    ///
    /// A file that is exactly 7 days old in the sequential tier should have decay_weight = 1.0 (within grace).
    /// A file that is 8+ days old should have decay_weight < 1.0 (grace period exceeded).
    /// </summary>
    [Test]
    public void ListMemories_SequentialFile_Uses7DayGracePeriod()
    {
        // Arrange: Create a sequential file that is exactly 6 days old (within 7d grace)
        var testFile = Path.Combine(_sequentialDir, "session-grace-test.md");
        var createdDate = DateTime.UtcNow.AddDays(-6).ToString("o", CultureInfo.InvariantCulture);
        var content = $@"---
title: Sequential Grace Test
type: sequential
status: saved
created: {createdDate}
modified: {createdDate}
---

# Sequential Grace Test

Test content for [[decay-metadata]] sequential tier 7d grace verification.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("thinking/sequential/decay-test");

        // Assert: Within 7d grace, decay_weight should be 1.0 (full weight, no decay)
        Assert.That(result, Does.Contain("decay_weight").Or.Contain("decay weight"),
            "ListMemories output should include decay_weight for sequential file");

        // A 6-day-old file should have decay_weight of 1.0 (within 7d grace)
        // Pattern: decay_weight: 1.0 or decay_weight: 1
        Assert.That(result, Does.Match(@"decay[_\s]?weight[:\s]+1(\.0+)?"),
            "Sequential file within 7d grace should have decay_weight = 1.0");
    }

    /// <summary>
    /// T-GRAPH-DECAY-003.2: Workflow file SHALL use 14d grace period for decay calculation.
    /// Verifies NFR-7.7.1 requirement for thinking/workflows/ tier.
    ///
    /// A file that is 10 days old in the workflows tier should have decay_weight = 1.0 (within 14d grace).
    /// </summary>
    [Test]
    public void ListMemories_WorkflowFile_Uses14DayGracePeriod()
    {
        // Arrange: Create a workflow file that is 10 days old (within 14d grace)
        var testFile = Path.Combine(_workflowsDir, "workflow-grace-test.md");
        var createdDate = DateTime.UtcNow.AddDays(-10).ToString("o", CultureInfo.InvariantCulture);
        var content = $@"---
title: Workflow Grace Test
type: workflow
status: saved
created: {createdDate}
modified: {createdDate}
---

# Workflow Grace Test

Test content for [[decay-metadata]] workflow tier 14d grace verification.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("thinking/workflows/decay-test");

        // Assert: Within 14d grace, decay_weight should be 1.0 (full weight, no decay)
        Assert.That(result, Does.Contain("decay_weight").Or.Contain("decay weight"),
            "ListMemories output should include decay_weight for workflow file");

        // A 10-day-old file should have decay_weight of 1.0 (within 14d grace)
        Assert.That(result, Does.Match(@"decay[_\s]?weight[:\s]+1(\.0+)?"),
            "Workflow file within 14d grace should have decay_weight = 1.0");
    }

    /// <summary>
    /// T-GRAPH-DECAY-003.2: Standard memory file SHALL use 14d grace period for decay calculation.
    /// Verifies NFR-7.7.1 requirement for all other memory paths.
    ///
    /// A file that is 10 days old in standard memory should have decay_weight = 1.0 (within 14d grace).
    /// </summary>
    [Test]
    public void ListMemories_StandardFile_Uses14DayGracePeriod()
    {
        // Arrange: Create a standard memory file that is 10 days old (within 14d grace)
        var testFile = Path.Combine(_standardDir, "standard-grace-test.md");
        var createdDate = DateTime.UtcNow.AddDays(-10).ToString("o", CultureInfo.InvariantCulture);
        var content = $@"---
title: Standard Grace Test
type: memory
status: saved
created: {createdDate}
modified: {createdDate}
---

# Standard Grace Test

Test content for [[decay-metadata]] standard memory 14d grace verification.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("decay-metadata-tests/standard");

        // Assert: Within 14d grace, decay_weight should be 1.0 (full weight, no decay)
        Assert.That(result, Does.Contain("decay_weight").Or.Contain("decay weight"),
            "ListMemories output should include decay_weight for standard file");

        // A 10-day-old file should have decay_weight of 1.0 (within 14d grace)
        Assert.That(result, Does.Match(@"decay[_\s]?weight[:\s]+1(\.0+)?"),
            "Standard file within 14d grace should have decay_weight = 1.0");
    }

    #endregion

    #region Edge Cases and Boundary Conditions

    /// <summary>
    /// T-GRAPH-DECAY-003.2: Sequential file past grace period should have decayed weight.
    /// Verifies decay calculation occurs after grace period expires.
    /// </summary>
    [Test]
    public void ListMemories_SequentialFile_PastGracePeriod_HasDecayedWeight()
    {
        // Arrange: Create a sequential file that is 14 days old (past 7d grace)
        var testFile = Path.Combine(_sequentialDir, "session-decayed-test.md");
        var createdDate = DateTime.UtcNow.AddDays(-14).ToString("o", CultureInfo.InvariantCulture);
        var content = $@"---
title: Sequential Decayed Test
type: sequential
status: saved
created: {createdDate}
modified: {createdDate}
---

# Sequential Decayed Test

Test content for [[decay-metadata]] sequential tier decay after grace period.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("thinking/sequential/decay-test");

        // Assert: Past 7d grace, decay_weight should be < 1.0
        Assert.That(result, Does.Contain("decay_weight").Or.Contain("decay weight"),
            "ListMemories output should include decay_weight for decayed sequential file");

        // A 14-day-old sequential file (7 days past the 7d grace) should have decay_weight < 1.0
        // Pattern: matches 0.xxx values (less than 1.0)
        Assert.That(result, Does.Match(@"decay[_\s]?weight[:\s]+0\.\d+"),
            "Sequential file past 7d grace should have decay_weight < 1.0");
    }

    /// <summary>
    /// T-GRAPH-DECAY-003.1: File without last_accessed should fall back to created/modified.
    /// Verifies graceful handling when last_accessed is not present.
    /// </summary>
    [Test]
    public void ListMemories_FileWithoutLastAccessed_UsesFallback()
    {
        // Arrange: Create a file without last_accessed in frontmatter
        var testFile = Path.Combine(_standardDir, "no-accessed-test.md");
        var createdDate = DateTime.UtcNow.AddDays(-5).ToString("o", CultureInfo.InvariantCulture);
        var content = $@"---
title: No Accessed Test
type: memory
status: saved
created: {createdDate}
modified: {createdDate}
---

# No Accessed Test

Test content for [[decay-metadata]] fallback when last_accessed is missing.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("decay-metadata-tests/standard");

        // Assert: Should still show last_accessed (using created/modified as fallback)
        Assert.That(result, Does.Contain("last_accessed").Or.Contain("last accessed"),
            "ListMemories should display last_accessed even when using fallback value");

        // decay_weight should still be calculated
        Assert.That(result, Does.Contain("decay_weight").Or.Contain("decay weight"),
            "ListMemories should calculate decay_weight even without explicit last_accessed");
    }

    /// <summary>
    /// T-GRAPH-DECAY-003.1: Newly created file should have decay_weight = 1.0.
    /// Verifies brand new files have full weight.
    /// </summary>
    [Test]
    public void ListMemories_NewFile_HasFullDecayWeight()
    {
        // Arrange: Create a brand new file (created just now)
        var testFile = Path.Combine(_standardDir, "new-file-test.md");
        var content = @"---
title: New File Test
type: memory
status: saved
---

# New File Test

Test content for [[decay-metadata]] new file full weight verification.
";
        File.WriteAllText(testFile, content);

        // Act
        var result = SystemTools.ListMemories("decay-metadata-tests/standard");

        // Assert: Brand new file should have decay_weight = 1.0
        Assert.That(result, Does.Contain("decay_weight").Or.Contain("decay weight"),
            "ListMemories should include decay_weight for new file");

        // New file should have full weight (1.0)
        Assert.That(result, Does.Match(@"decay[_\s]?weight[:\s]+1(\.0+)?"),
            "New file should have decay_weight = 1.0");
    }

    #endregion
}
