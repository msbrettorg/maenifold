using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-DATE-001: RTM-001, RTM-002 regression tests for GetSessionPath date detection
public class DateDetectionPathTests
{
    // T-DATE-001 RTM-001: Sequential session ID parses timestamp correctly
    [Test]
    public void GetSessionPath_SequentialSessionId_ParsesTimestampToCorrectDate()
    {
        // 1770910549160 ms = 2026-02-12 UTC
        var path = MarkdownWriter.GetSessionPath("sequential", "session-1770910549160-79869");

        // T-RTM-001: Must parse the timestamp after stripping the "session-" prefix
        Assert.That(path, Does.Contain(Path.Combine("2026", "02", "12")));
        Assert.That(path, Does.EndWith($"session-1770910549160-79869.md"));
    }

    // T-DATE-001 RTM-001: Workflow session ID (no random suffix) parses timestamp correctly
    [Test]
    public void GetSessionPath_WorkflowSessionId_ParsesTimestampToCorrectDate()
    {
        // 1770910549160 ms = 2026-02-12 UTC
        var path = MarkdownWriter.GetSessionPath("workflow", "workflow-1770910549160");

        // T-RTM-001: Must handle session IDs without a trailing random suffix
        Assert.That(path, Does.Contain(Path.Combine("2026", "02", "12")));
        Assert.That(path, Does.EndWith($"workflow-1770910549160.md"));
    }

    // T-DATE-001 RTM-002: Path does NOT contain 1970 (regression guard)
    [Test]
    public void GetSessionPath_ModernTimestamp_DoesNotProduceEpochDate()
    {
        var path = MarkdownWriter.GetSessionPath("sequential", "session-1770910549160-79869");

        // T-RTM-002: The old bug parsed "session" as the timestamp, yielding epoch 0 -> 1970/01/01
        Assert.That(path, Does.Not.Contain("1970"));
    }

    // T-DATE-001 RTM-002: Path structure matches year/month/day directories
    [Test]
    public void GetSessionPath_ReturnsCorrectDirectoryStructure()
    {
        var path = MarkdownWriter.GetSessionPath("sequential", "session-1770910549160-79869");

        // T-RTM-002: Full expected path suffix
        var expectedSuffix = Path.Combine("thinking", "sequential", "2026", "02", "12", "session-1770910549160-79869.md");
        Assert.That(path, Does.EndWith(expectedSuffix));
    }

    // T-DATE-001 RTM-001: Different timestamps produce different date directories
    [Test]
    public void GetSessionPath_DifferentTimestamps_ProduceDifferentDateDirectories()
    {
        // 1609459200000 ms = 2021-01-01 00:00:00 UTC
        var pathA = MarkdownWriter.GetSessionPath("sequential", "session-1609459200000-12345");

        // 1672531200000 ms = 2023-01-01 00:00:00 UTC
        var pathB = MarkdownWriter.GetSessionPath("sequential", "session-1672531200000-12345");

        // T-RTM-001: The parsed timestamps must drive the date directory
        Assert.That(pathA, Does.Contain(Path.Combine("2021", "01", "01")));
        Assert.That(pathB, Does.Contain(Path.Combine("2023", "01", "01")));
        Assert.That(pathA, Is.Not.EqualTo(pathB));
    }

    // T-DATE-001 RTM-002: ThinkingType parameter propagates to path
    [Test]
    public void GetSessionPath_ThinkingType_PropagatesCorrectly()
    {
        var path = MarkdownWriter.GetSessionPath("workflow", "workflow-1770910549160");

        // T-RTM-002: thinkingType must appear in the path under /thinking/
        Assert.That(path, Does.Contain(Path.Combine("thinking", "workflow")));
        Assert.That(path, Does.Not.Contain(Path.Combine("thinking", "sequential")));
    }
}
