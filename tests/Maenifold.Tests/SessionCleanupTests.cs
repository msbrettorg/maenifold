// T-COV-001.5: RTM FR-17.7
// Goal: Unit tests for SessionCleanup covering abandonment detection, threshold logic,
// and the DB metadata pre-pass (TryMarkAbandonedFromLastIndexed).

using System.Globalization;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

[TestFixture]
[Category("T-COV-001.5")]
public class SessionCleanupTests
{
    private string _testRoot = string.Empty;

    [SetUp]
    public void SetUp()
    {
        _testRoot = Path.Combine(Path.GetTempPath(), "maenifold-test-root", $"session-cleanup-{Guid.NewGuid():N}");
        Directory.CreateDirectory(_testRoot);
        Config.OverrideRoot(_testRoot);
        Config.EnsureDirectories();
    }

    [TearDown]
    public void TearDown()
    {
        Config.ResetOverrides();
        if (Directory.Exists(_testRoot))
        {
            try { Directory.Delete(_testRoot, true); } catch { /* ignore cleanup errors */ }
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // HandleSessionCleanup — stale workflow session
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void HandleSessionCleanup_StaleWorkflowSession_MarksStatusAbandoned()
    {
        // T-COV-001.5: RTM FR-17.7
        var staleModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "workflow",
            ["status"] = "active",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var filePath = CreateTestFile("workflow", "stale-workflow-test", frontmatter!, "# Test Session\n\nContent");

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Test Session\n\nContent");

        var (readFrontmatter, _, _) = MarkdownIO.ReadMarkdown(filePath);
        Assert.That(readFrontmatter, Is.Not.Null);
        Assert.That(readFrontmatter!["status"], Is.EqualTo("abandoned"),
            "FR-17.7: stale workflow session must be marked abandoned.");
    }

    [Test]
    public void HandleSessionCleanup_StaleWorkflowSession_AppendsAbandonedSection()
    {
        // T-COV-001.5: RTM FR-17.7
        var staleModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "workflow",
            ["status"] = "active",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var filePath = CreateTestFile("workflow", "stale-workflow-section-test", frontmatter!, "# Test Session\n\nContent");

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Test Session\n\nContent");

        var fileText = File.ReadAllText(filePath);
        Assert.That(fileText, Does.Contain("Session Abandoned"),
            "FR-17.7: abandoned workflow session must have Session Abandoned section appended.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // HandleSessionCleanup — stale sequential session
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void HandleSessionCleanup_StaleSequentialSession_MarksStatusAbandoned()
    {
        // T-COV-001.5: RTM FR-17.7
        var staleModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "sequential",
            ["status"] = "active",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var filePath = CreateTestFile("sequential", "stale-sequential-test", frontmatter!, "# Sequential Session\n\nContent");

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Sequential Session\n\nContent");

        var (readFrontmatter, _, _) = MarkdownIO.ReadMarkdown(filePath);
        Assert.That(readFrontmatter, Is.Not.Null);
        Assert.That(readFrontmatter!["status"], Is.EqualTo("abandoned"),
            "FR-17.7: stale sequential session must be marked abandoned.");
    }

    [Test]
    public void HandleSessionCleanup_StaleSequentialSession_AppendsAbandonedSection()
    {
        // T-COV-001.5: RTM FR-17.7
        var staleModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "sequential",
            ["status"] = "active",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var filePath = CreateTestFile("sequential", "stale-sequential-section-test", frontmatter!, "# Sequential Session\n\nContent");

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Sequential Session\n\nContent");

        var fileText = File.ReadAllText(filePath);
        Assert.That(fileText, Does.Contain("Session Abandoned"),
            "FR-17.7: abandoned sequential session must have Session Abandoned section appended.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // HandleSessionCleanup — recent active session (within threshold)
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void HandleSessionCleanup_RecentActiveSession_NoChange()
    {
        // T-COV-001.5: RTM FR-17.7
        var recentModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes - 5));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "workflow",
            ["status"] = "active",
            ["modified"] = recentModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var filePath = CreateTestFile("workflow", "recent-session-test", frontmatter!, "# Recent Session\n\nContent");
        var beforeMtime = File.GetLastWriteTimeUtc(filePath);

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Recent Session\n\nContent");

        var afterMtime = File.GetLastWriteTimeUtc(filePath);
        Assert.That(afterMtime, Is.EqualTo(beforeMtime),
            "FR-17.7: a recent active session must not be rewritten.");

        var (readFrontmatter, _, _) = MarkdownIO.ReadMarkdown(filePath);
        Assert.That(readFrontmatter!["status"], Is.EqualTo("active"),
            "FR-17.7: a recent active session must remain active.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // HandleSessionCleanup — non-session file (type != workflow/sequential)
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void HandleSessionCleanup_NullType_NoChange()
    {
        // T-COV-001.5: RTM FR-17.7
        var staleModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = null,
            ["status"] = "active",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var filePath = CreateTestFile("notes", "null-type-test", frontmatter!, "# Note\n\nContent");
        var beforeMtime = File.GetLastWriteTimeUtc(filePath);

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Note\n\nContent");

        var afterMtime = File.GetLastWriteTimeUtc(filePath);
        Assert.That(afterMtime, Is.EqualTo(beforeMtime),
            "FR-17.7: files with null type must not be abandoned.");
    }

    [Test]
    public void HandleSessionCleanup_MemoryType_NoChange()
    {
        // T-COV-001.5: RTM FR-17.7
        var staleModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "memory",
            ["status"] = "active",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var filePath = CreateTestFile("notes", "memory-type-test", frontmatter!, "# Memory Note\n\nContent");
        var beforeMtime = File.GetLastWriteTimeUtc(filePath);

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Memory Note\n\nContent");

        var afterMtime = File.GetLastWriteTimeUtc(filePath);
        Assert.That(afterMtime, Is.EqualTo(beforeMtime),
            "FR-17.7: files with type=memory must not be abandoned.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // HandleSessionCleanup — already abandoned session
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void HandleSessionCleanup_AlreadyAbandoned_NoChange()
    {
        // T-COV-001.5: RTM FR-17.7
        var staleModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "workflow",
            ["status"] = "abandoned",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var filePath = CreateTestFile("workflow", "already-abandoned-test", frontmatter!, "# Abandoned Session\n\nContent");
        var beforeMtime = File.GetLastWriteTimeUtc(filePath);

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Abandoned Session\n\nContent");

        var afterMtime = File.GetLastWriteTimeUtc(filePath);
        Assert.That(afterMtime, Is.EqualTo(beforeMtime),
            "FR-17.7: already-abandoned sessions must not be rewritten.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // HandleSessionCleanup — missing modified field
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void HandleSessionCleanup_MissingModifiedField_NoExceptionAndNoChange()
    {
        // T-COV-001.5: RTM FR-17.7
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "workflow",
            ["status"] = "active"
            // "modified" intentionally absent
        };

        var filePath = CreateTestFile("workflow", "missing-modified-test", frontmatter!, "# Session\n\nContent");
        var beforeMtime = File.GetLastWriteTimeUtc(filePath);

        Assert.DoesNotThrow(
            () => SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Session\n\nContent"),
            "FR-17.7: missing modified field must not throw an exception.");

        var afterMtime = File.GetLastWriteTimeUtc(filePath);
        Assert.That(afterMtime, Is.EqualTo(beforeMtime),
            "FR-17.7: file with missing modified field must not be rewritten.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // HandleSessionCleanup — invalid date in modified field
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void HandleSessionCleanup_InvalidDateInModified_NoExceptionAndNoChange()
    {
        // T-COV-001.5: RTM FR-17.7
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "workflow",
            ["status"] = "active",
            ["modified"] = "not-a-date"
        };

        var filePath = CreateTestFile("workflow", "invalid-date-test", frontmatter!, "# Session\n\nContent");
        var beforeMtime = File.GetLastWriteTimeUtc(filePath);

        Assert.DoesNotThrow(
            () => SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Session\n\nContent"),
            "FR-17.7: invalid modified date must not throw an exception.");

        var afterMtime = File.GetLastWriteTimeUtc(filePath);
        Assert.That(afterMtime, Is.EqualTo(beforeMtime),
            "FR-17.7: file with invalid modified date must not be rewritten.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // TryMarkAbandonedFromLastIndexed — stale (lastIndexed older than threshold)
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void TryMarkAbandonedFromLastIndexed_StaleSession_ReturnsTrue()
    {
        // T-COV-001.5: RTM FR-17.7
        var nowUtc = DateTime.UtcNow;
        var lastIndexedUtc = nowUtc.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));

        var frontmatter = new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "active"
        };
        var content = "# Stale Workflow\n\nSome content.";

        var result = SessionCleanup.TryMarkAbandonedFromLastIndexed(frontmatter, content, lastIndexedUtc, nowUtc, out _);

        Assert.That(result, Is.True,
            "FR-17.7: TryMarkAbandonedFromLastIndexed must return true for stale sessions.");
    }

    [Test]
    public void TryMarkAbandonedFromLastIndexed_StaleSession_UpdatesFrontmatterStatus()
    {
        // T-COV-001.5: RTM FR-17.7
        var nowUtc = DateTime.UtcNow;
        var lastIndexedUtc = nowUtc.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));

        var frontmatter = new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "active"
        };
        var content = "# Stale Workflow\n\nSome content.";

        SessionCleanup.TryMarkAbandonedFromLastIndexed(frontmatter, content, lastIndexedUtc, nowUtc, out _);

        Assert.That(frontmatter["status"], Is.EqualTo("abandoned"),
            "FR-17.7: frontmatter status must be set to 'abandoned' for stale sessions.");
    }

    [Test]
    public void TryMarkAbandonedFromLastIndexed_StaleSession_AppendsAbandonedSection()
    {
        // T-COV-001.5: RTM FR-17.7
        var nowUtc = DateTime.UtcNow;
        var lastIndexedUtc = nowUtc.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));

        var frontmatter = new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "active"
        };
        var content = "# Stale Workflow\n\nSome content.";

        SessionCleanup.TryMarkAbandonedFromLastIndexed(frontmatter, content, lastIndexedUtc, nowUtc, out var updatedContent);

        Assert.That(updatedContent, Does.Contain("Session Abandoned"),
            "FR-17.7: updatedContent must contain Session Abandoned section for stale sessions.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // TryMarkAbandonedFromLastIndexed — recent (within threshold)
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void TryMarkAbandonedFromLastIndexed_RecentSession_ReturnsFalse()
    {
        // T-COV-001.5: RTM FR-17.7
        var nowUtc = DateTime.UtcNow;
        var lastIndexedUtc = nowUtc.AddMinutes(-(Config.SessionAbandonmentMinutes - 5));

        var frontmatter = new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "active"
        };
        var content = "# Recent Session\n\nSome content.";

        var result = SessionCleanup.TryMarkAbandonedFromLastIndexed(frontmatter, content, lastIndexedUtc, nowUtc, out var updatedContent);

        Assert.That(result, Is.False,
            "FR-17.7: TryMarkAbandonedFromLastIndexed must return false for recent sessions.");
        Assert.That(updatedContent, Is.EqualTo(content),
            "FR-17.7: updatedContent must be unchanged for recent sessions.");
    }

    [Test]
    public void TryMarkAbandonedFromLastIndexed_RecentSession_FrontmatterUnchanged()
    {
        // T-COV-001.5: RTM FR-17.7
        var nowUtc = DateTime.UtcNow;
        var lastIndexedUtc = nowUtc.AddMinutes(-(Config.SessionAbandonmentMinutes - 5));

        var frontmatter = new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "active"
        };
        var content = "# Recent Session\n\nSome content.";

        SessionCleanup.TryMarkAbandonedFromLastIndexed(frontmatter, content, lastIndexedUtc, nowUtc, out _);

        Assert.That(frontmatter["status"], Is.EqualTo("active"),
            "FR-17.7: frontmatter status must remain 'active' for recent sessions.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // TryMarkAbandonedFromLastIndexed — exactly at threshold (boundary)
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void TryMarkAbandonedFromLastIndexed_ExactlyAtThreshold_ReturnsFalse()
    {
        // T-COV-001.5: RTM FR-17.7
        // When timeSinceIndexed == SessionAbandonmentMinutes exactly, the condition is
        // <= (not <), so it returns false — the session is NOT yet abandoned.
        var nowUtc = DateTime.UtcNow;
        var lastIndexedUtc = nowUtc.AddMinutes(-Config.SessionAbandonmentMinutes);

        var frontmatter = new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "active"
        };
        var content = "# Boundary Session\n\nContent.";

        var result = SessionCleanup.TryMarkAbandonedFromLastIndexed(frontmatter, content, lastIndexedUtc, nowUtc, out var updatedContent);

        Assert.That(result, Is.False,
            "FR-17.7: a session at exactly the threshold must not be marked abandoned (boundary: <= means not yet over).");
        Assert.That(updatedContent, Is.EqualTo(content),
            "FR-17.7: content must be unchanged at the exact threshold boundary.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // TryMarkAbandonedFromLastIndexed — content preservation
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void TryMarkAbandonedFromLastIndexed_StaleSession_PreservesOriginalContent()
    {
        // T-COV-001.5: RTM FR-17.7
        var nowUtc = DateTime.UtcNow;
        var lastIndexedUtc = nowUtc.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));

        var frontmatter = new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "active"
        };
        var originalContent = "# My Workflow\n\nImportant details about the work.\n\n## Step 1\n\nDo something.\n";

        SessionCleanup.TryMarkAbandonedFromLastIndexed(frontmatter, originalContent, lastIndexedUtc, nowUtc, out var updatedContent);

        Assert.That(updatedContent, Does.StartWith(originalContent),
            "FR-17.7: original content must be preserved as a prefix in the updated content.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Helpers
    // ────────────────────────────────────────────────────────────────────────────

    /// <summary>
    /// Creates a real markdown file under the test memory path with given frontmatter and content.
    /// The folder is created relative to the thinking area (for session types) or notes (for others).
    /// Returns the absolute file path.
    /// </summary>
    private static string CreateTestFile(string subFolder, string fileName, Dictionary<string, object> frontmatter, string content)
    {
        string dir;
        if (subFolder is "workflow" or "sequential")
        {
            dir = Path.Combine(Config.MemoryPath, "thinking", subFolder);
        }
        else
        {
            dir = Path.Combine(Config.MemoryPath, subFolder);
        }

        Directory.CreateDirectory(dir);
        var filePath = Path.Combine(dir, $"{fileName}.md");
        MarkdownIO.WriteMarkdown(filePath, frontmatter, content);
        return filePath;
    }
}
