using Maenifold.Tools;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// T-DATE-001: Tests for session ID validation (RTM-003) and extraction (RTM-004).
/// Pure function tests — no mocks, no setup/teardown, no file I/O.
/// </summary>
public class DateDetectionValidationTests
{
    // ---------------------------------------------------------------
    // RTM-003: IsValidSessionIdFormat SHALL validate the timestamp
    //          segment (not random suffix) is a valid long.
    // ---------------------------------------------------------------

    // T-DATE-001: RTM FR-2 — valid sequential session ID with timestamp and random suffix
    [Test]
    public void IsValidSessionIdFormat_ValidSequentialSessionId_ReturnsTrue()
    {
        // Arrange
        var sessionId = "session-1770910549160-79869";

        // Act
        var result = SequentialThinkingTools.IsValidSessionIdFormat(sessionId);

        // Assert
        Assert.That(result, Is.True,
            "A standard sequential session ID with numeric timestamp should be valid");
    }

    // T-DATE-001: RTM FR-2 — valid workflow session ID (no random suffix)
    [Test]
    public void IsValidSessionIdFormat_ValidWorkflowSessionId_ReturnsTrue()
    {
        // Arrange
        var sessionId = "workflow-1770910549160";

        // Act
        var result = SequentialThinkingTools.IsValidSessionIdFormat(sessionId);

        // Assert
        Assert.That(result, Is.True,
            "A workflow session ID with numeric timestamp and no suffix should be valid");
    }

    // T-DATE-001: RTM FR-2 — non-numeric timestamp segment must fail
    [Test]
    public void IsValidSessionIdFormat_NonNumericTimestamp_ReturnsFalse()
    {
        // Arrange
        var sessionId = "session-abc-12345";

        // Act
        var result = SequentialThinkingTools.IsValidSessionIdFormat(sessionId);

        // Assert
        Assert.That(result, Is.False,
            "A session ID with non-numeric timestamp segment should be invalid");
    }

    // T-DATE-001: RTM FR-2 — no dash separator at all
    [Test]
    public void IsValidSessionIdFormat_NoDash_ReturnsFalse()
    {
        // Arrange
        var sessionId = "noseparator";

        // Act
        var result = SequentialThinkingTools.IsValidSessionIdFormat(sessionId);

        // Assert
        Assert.That(result, Is.False,
            "A session ID with no dash separator should be invalid");
    }

    // T-DATE-001: RTM FR-2 — trailing dash with nothing after it
    [Test]
    public void IsValidSessionIdFormat_TrailingDashOnly_ReturnsFalse()
    {
        // Arrange
        var sessionId = "session-";

        // Act
        var result = SequentialThinkingTools.IsValidSessionIdFormat(sessionId);

        // Assert
        Assert.That(result, Is.False,
            "A session ID with only a trailing dash should be invalid");
    }

    // T-DATE-001: RTM FR-2 — any prefix works as long as timestamp segment is numeric
    [Test]
    public void IsValidSessionIdFormat_CustomPrefix_ReturnsTrue()
    {
        // Arrange
        var sessionId = "custom-1234567890";

        // Act
        var result = SequentialThinkingTools.IsValidSessionIdFormat(sessionId);

        // Assert
        Assert.That(result, Is.True,
            "Any prefix with a numeric timestamp segment should be valid");
    }

    // T-DATE-001: RTM FR-2 — zero is a valid long value
    [Test]
    public void IsValidSessionIdFormat_ZeroTimestamp_ReturnsTrue()
    {
        // Arrange
        var sessionId = "session-0-12345";

        // Act
        var result = SequentialThinkingTools.IsValidSessionIdFormat(sessionId);

        // Assert
        Assert.That(result, Is.True,
            "Zero is a valid long value and should pass timestamp validation");
    }

    // ---------------------------------------------------------------
    // RTM-004: ExtractSessionId SHALL return session ID from last
    //          path segment, not segments[1].
    // ---------------------------------------------------------------

    // T-DATE-001: RTM FR-3 — sequential path with date directories
    [Test]
    public void ExtractSessionId_SequentialPathWithDateDirs_ReturnsSessionId()
    {
        // Arrange
        var filePath = "memory://thinking/sequential/2026/02/12/session-1770910549160-79869.md";

        // Act
        var result = RecentActivityReader.ExtractSessionId(filePath);

        // Assert
        Assert.That(result, Is.EqualTo("session-1770910549160-79869"),
            "Should extract session ID from the last path segment, stripping .md extension");
    }

    // T-DATE-001: RTM FR-3 — workflow path with date directories
    [Test]
    public void ExtractSessionId_WorkflowPathWithDateDirs_ReturnsSessionId()
    {
        // Arrange
        var filePath = "memory://thinking/workflow/2026/02/12/workflow-1770910549160.md";

        // Act
        var result = RecentActivityReader.ExtractSessionId(filePath);

        // Assert
        Assert.That(result, Is.EqualTo("workflow-1770910549160"),
            "Should extract workflow session ID from the last path segment");
    }

    // T-DATE-001: RTM FR-3 — path without memory:// prefix
    [Test]
    public void ExtractSessionId_PathWithoutMemoryPrefix_ReturnsSessionId()
    {
        // Arrange
        var filePath = "sequential/2026/02/12/session-123-456.md";

        // Act
        var result = RecentActivityReader.ExtractSessionId(filePath);

        // Assert
        Assert.That(result, Is.EqualTo("session-123-456"),
            "Should handle paths without memory:// prefix");
    }

    // T-DATE-001: RTM FR-3 — regression: must return filename, not year directory
    [Test]
    public void ExtractSessionId_PathWithDateDirs_DoesNotReturnYearAsSessionId()
    {
        // Arrange
        var filePath = "memory://thinking/sequential/2026/02/12/session-123-456.md";

        // Act
        var result = RecentActivityReader.ExtractSessionId(filePath);

        // Assert
        Assert.That(result, Is.Not.EqualTo("2026"),
            "Regression: must return the filename-based session ID, not the year directory segment");
        Assert.That(result, Is.EqualTo("session-123-456"),
            "Should return the session ID from the last path segment");
    }
}
