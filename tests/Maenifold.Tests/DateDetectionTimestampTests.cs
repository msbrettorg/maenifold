using System.Text.RegularExpressions;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-DATE-001: RTM-005, RTM-006 regression tests for timestamp formatting
// RTM-005: All human-readable timestamps SHALL include " UTC" suffix
// RTM-006: ISO 8601 timestamps in FinalizeSession SHALL use CultureInfo.InvariantCulture
public class DateDetectionTimestampTests
{
    private string _testMemoryPath = null!;

    [SetUp]
    public void SetUp()
    {
        _testMemoryPath = Config.TestMemoryPath;
        Directory.CreateDirectory(_testMemoryPath);
        Directory.CreateDirectory(Path.Combine(_testMemoryPath, "thinking", "sequential"));
        Directory.CreateDirectory(Path.Combine(_testMemoryPath, "thinking", "workflow"));
        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", Path.GetDirectoryName(_testMemoryPath));
    }

    [TearDown]
    public void TearDown()
    {
        var testOutputsDir = Path.GetDirectoryName(_testMemoryPath);
        if (testOutputsDir != null && Directory.Exists(testOutputsDir))
            Directory.Delete(testOutputsDir, true);
        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", null);
    }

    // ============ RTM-005: UTC suffix tests ============

    /// <summary>
    /// T-DATE-001 RTM-005: Sequential thought timestamp includes " UTC" suffix.
    /// Verifies BuildThoughtSection formats human-readable timestamp as "YYYY-MM-DD HH:MM:SS UTC".
    /// </summary>
    [Test]
    public void SequentialThoughtTimestamp_IncludesUtcSuffix()
    {
        // Arrange & Act — create a session with one thought
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Test thought with [[utc-timestamp-verification]]",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2);

        Assert.That(result, Does.Contain("Created session:"), "Session should be created successfully");

        var sessionId = ExtractSessionId(result);
        var (_, content, _) = MarkdownIO.ReadSession("sequential", sessionId);

        // T-DATE-001 RTM-005: human-readable timestamp MUST include UTC suffix
        var utcTimestampPattern = new Regex(@"\*\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC\*");
        Assert.That(utcTimestampPattern.IsMatch(content), Is.True,
            $"Content should contain timestamp with UTC suffix. Content:\n{content}");
    }

    /// <summary>
    /// T-DATE-001 RTM-005: Sequential thought timestamp does NOT end with bare time (regression guard).
    /// Ensures no timestamp appears in *YYYY-MM-DD HH:MM:SS* format without the UTC suffix.
    /// </summary>
    [Test]
    public void SequentialThoughtTimestamp_DoesNotContainBareTimestampWithoutUtc()
    {
        // Arrange & Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Test thought with [[bare-timestamp-regression]]",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2);

        Assert.That(result, Does.Contain("Created session:"), "Session should be created successfully");

        var sessionId = ExtractSessionId(result);
        var (_, content, _) = MarkdownIO.ReadSession("sequential", sessionId);

        // T-DATE-001 RTM-005: every italicized timestamp MUST have UTC suffix
        // Match timestamps that do NOT have " UTC" after the seconds
        var bareTimestampPattern = new Regex(@"\*\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\*");
        Assert.That(bareTimestampPattern.IsMatch(content), Is.False,
            $"Content should NOT contain timestamp without UTC suffix. Content:\n{content}");
    }

    // ============ RTM-006: InvariantCulture ISO 8601 tests ============

    /// <summary>
    /// T-DATE-001 RTM-006: Completed session has ISO 8601 "completed" field in frontmatter.
    /// Verifies FinalizeSession uses CultureInfo.InvariantCulture for the "o" format specifier.
    /// </summary>
    [Test]
    public void CompletedSession_HasIso8601CompletedFieldInFrontmatter()
    {
        // Arrange & Act — create and complete in one call
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Thought with [[iso8601-completion-test]]",
            nextThoughtNeeded: false,
            thoughtNumber: 0,
            totalThoughts: 1,
            conclusion: "Final synthesis with [[conclusion-verification]]");

        Assert.That(result, Does.Contain("Created session:"), "Session should be created successfully");
        Assert.That(result, Does.Contain("Thinking complete"), "Session should complete");

        var sessionId = ExtractSessionId(result);
        var (frontmatter, _, _) = MarkdownIO.ReadSession("sequential", sessionId);

        // T-DATE-001 RTM-006: status must be "completed"
        Assert.That(frontmatter, Is.Not.Null, "Frontmatter should exist");
        Assert.That(frontmatter!["status"], Is.EqualTo("completed"),
            "Status should be 'completed'");

        // T-DATE-001 RTM-006: "completed" field must be ISO 8601 round-trip format ("o")
        Assert.That(frontmatter.ContainsKey("completed"), Is.True,
            "Frontmatter should contain 'completed' key");

        var completedValue = frontmatter["completed"]?.ToString();
        Assert.That(completedValue, Is.Not.Null.And.Not.Empty,
            "Completed value should not be null or empty");

        // ISO 8601 round-trip format contains 'T' separator and time zone info
        var iso8601Pattern = new Regex(@"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}");
        Assert.That(iso8601Pattern.IsMatch(completedValue!), Is.True,
            $"Completed timestamp should be ISO 8601 format (contains 'T' separator). Got: {completedValue}");
    }

    /// <summary>
    /// T-DATE-001 RTM-006: Cancelled session has ISO 8601 "cancelled" field in frontmatter.
    /// Verifies FinalizeSession cancel path uses CultureInfo.InvariantCulture for the "o" format specifier.
    /// </summary>
    [Test]
    public void CancelledSession_HasIso8601CancelledFieldInFrontmatter()
    {
        // Arrange — create session first
        var result1 = SequentialThinkingTools.SequentialThinking(
            response: "Initial thought with [[cancel-iso8601-test]]",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 3);

        Assert.That(result1, Does.Contain("Created session:"), "Session should be created successfully");

        var sessionId = ExtractSessionId(result1);

        // Act — cancel session
        var result2 = SequentialThinkingTools.SequentialThinking(
            sessionId: sessionId,
            cancel: true);

        Assert.That(result2, Does.Contain("Thinking cancelled"), "Session should be cancelled");

        var (frontmatter, _, _) = MarkdownIO.ReadSession("sequential", sessionId);

        // T-DATE-001 RTM-006: status must be "cancelled"
        Assert.That(frontmatter, Is.Not.Null, "Frontmatter should exist");
        Assert.That(frontmatter!["status"], Is.EqualTo("cancelled"),
            "Status should be 'cancelled'");

        // T-DATE-001 RTM-006: "cancelled" field must be ISO 8601 round-trip format ("o")
        Assert.That(frontmatter.ContainsKey("cancelled"), Is.True,
            "Frontmatter should contain 'cancelled' key");

        var cancelledValue = frontmatter["cancelled"]?.ToString();
        Assert.That(cancelledValue, Is.Not.Null.And.Not.Empty,
            "Cancelled value should not be null or empty");

        // ISO 8601 round-trip format contains 'T' separator and time zone info
        var iso8601Pattern = new Regex(@"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}");
        Assert.That(iso8601Pattern.IsMatch(cancelledValue!), Is.True,
            $"Cancelled timestamp should be ISO 8601 format (contains 'T' separator). Got: {cancelledValue}");
    }

    // ============ Helper ============

    private static string ExtractSessionId(string responseMessage)
    {
        var lines = responseMessage.Split('\n');
        foreach (var line in lines)
        {
            if (line.Contains("Created session:") || line.Contains("to session:"))
            {
                var parts = line.Split(':');
                if (parts.Length > 1)
                    return parts[^1].Trim();
            }
        }
        throw new InvalidOperationException($"Could not extract session ID from: {responseMessage}");
    }
}
