// T-COV-001.DC: RTM FR-7.8, NFR-7.5.1, NFR-7.5.1a, NFR-7.5.2, NFR-7.5.5
// Targeted branch coverage tests for DecayCalculator.cs
// Covers: GetAssumptionDecayWeight (both overloads), GetDecayWeight(DateTime, DateTime?, string),
//         GetGracePeriodForPath edge cases, and GetDecayWeight(int, int, int, string) power-law branch.

using System;
using NUnit.Framework;
using Maenifold.Utils;

namespace Maenifold.Tests;

/// <summary>
/// Targeted coverage tests for DecayCalculator branches not exercised by existing test files.
/// Covers assumption status-based decay, access-boosting via lastAccessed, grace-period path
/// routing edge cases, and the GetDecayWeight power-law branch.
///
/// T-COV-001.DC: RTM FR-7.8, NFR-7.5.1, NFR-7.5.1a, NFR-7.5.2, NFR-7.5.5
/// </summary>
public class DecayCalculatorCoverageTests
{
    #region GetAssumptionDecayWeight — validated status

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - validated assumption always returns 1.0 regardless of age")]
    public void GetAssumptionDecayWeight_ValidatedStatus_ReturnsOne_WhenNew()
    {
        // Arrange: fresh validated assumption
        var created = DateTime.UtcNow.AddDays(-1);

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "validated");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Validated assumptions (new) must always return 1.0 — they are exempt from decay.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - validated assumption always returns 1.0 even when very old")]
    public void GetAssumptionDecayWeight_ValidatedStatus_ReturnsOne_WhenVeryOld()
    {
        // Arrange: year-old validated assumption
        var created = DateTime.UtcNow.AddDays(-365);

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "validated");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Validated assumptions (old) must still return 1.0 — decay exemption applies unconditionally.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - validated status is case-insensitive (VALIDATED)")]
    public void GetAssumptionDecayWeight_ValidatedStatus_CaseInsensitive()
    {
        // Arrange
        var created = DateTime.UtcNow.AddDays(-90);

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "VALIDATED");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Status matching must be case-insensitive; 'VALIDATED' should behave same as 'validated'.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - validated status with mixed case (Validated)")]
    public void GetAssumptionDecayWeight_ValidatedStatus_MixedCase()
    {
        // Arrange
        var created = DateTime.UtcNow.AddDays(-60);

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "Validated");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Mixed-case 'Validated' must be treated as validated (decay-exempt).");
    }

    #endregion

    #region GetAssumptionDecayWeight — invalidated status

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - invalidated assumption within 7-day grace returns 1.0")]
    public void GetAssumptionDecayWeight_InvalidatedStatus_WithinGracePeriod_ReturnsOne()
    {
        // Arrange: 3 days old — within 7-day invalidated grace period
        var created = DateTime.UtcNow.AddDays(-3);

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "invalidated");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Invalidated assumption within 7-day grace period must return 1.0.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - invalidated assumption past grace period decays aggressively")]
    public void GetAssumptionDecayWeight_InvalidatedStatus_PastGracePeriod_Decays()
    {
        // Arrange: 7 + 14 = 21 days old — exactly one half-life past the 7-day grace
        // Expected: exponential decay 0.5^1 = 0.5
        var created = DateTime.UtcNow.AddDays(-(7 + 14));

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "invalidated");

        // Assert
        Assert.That(weight, Is.EqualTo(0.5).Within(0.01),
            "Invalidated assumption at exactly one half-life (14d) past grace (7d) must return ~0.5.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - invalidated assumption is case-insensitive (INVALIDATED)")]
    public void GetAssumptionDecayWeight_InvalidatedStatus_CaseInsensitive()
    {
        // Arrange: past grace
        var created = DateTime.UtcNow.AddDays(-30);

        // Act
        var weightLower = DecayCalculator.GetAssumptionDecayWeight(created, "invalidated");
        var weightUpper = DecayCalculator.GetAssumptionDecayWeight(created, "INVALIDATED");

        // Assert
        Assert.That(weightUpper, Is.EqualTo(weightLower).Within(0.0001),
            "'INVALIDATED' must produce the same weight as 'invalidated'.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - invalidated decays faster than active (smaller grace + smaller half-life)")]
    public void GetAssumptionDecayWeight_Invalidated_DecaysFasterThanActive()
    {
        // Arrange: 30 days old — past both invalidated grace (7d) and active grace (14d)
        var created = DateTime.UtcNow.AddDays(-30);

        // Act
        var invalidatedWeight = DecayCalculator.GetAssumptionDecayWeight(created, "invalidated");
        var activeWeight = DecayCalculator.GetAssumptionDecayWeight(created, "active");

        // Assert
        Assert.That(invalidatedWeight, Is.LessThan(activeWeight),
            "Invalidated assumptions (7d grace, 14d half-life) must decay faster than active ones (14d grace, 30d half-life).");
    }

    #endregion

    #region GetAssumptionDecayWeight — active status

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - active assumption within 14-day grace returns 1.0")]
    public void GetAssumptionDecayWeight_ActiveStatus_WithinGracePeriod_ReturnsOne()
    {
        // Arrange: 10 days old — within 14-day active grace period
        var created = DateTime.UtcNow.AddDays(-10);

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "active");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Active assumption within 14-day grace period must return 1.0.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - active assumption past grace period decays with 30-day half-life")]
    public void GetAssumptionDecayWeight_ActiveStatus_PastGracePeriod_Decays()
    {
        // Arrange: 14 + 30 = 44 days old — exactly one half-life past 14-day grace
        // Expected: 0.5^1 = 0.5
        var created = DateTime.UtcNow.AddDays(-(14 + 30));

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "active");

        // Assert
        Assert.That(weight, Is.EqualTo(0.5).Within(0.01),
            "Active assumption exactly one half-life (30d) past grace (14d) must return ~0.5.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - active status is case-insensitive")]
    public void GetAssumptionDecayWeight_ActiveStatus_CaseInsensitive()
    {
        // Arrange
        var created = DateTime.UtcNow.AddDays(-50);

        // Act
        var weightLower = DecayCalculator.GetAssumptionDecayWeight(created, "active");
        var weightUpper = DecayCalculator.GetAssumptionDecayWeight(created, "ACTIVE");

        // Assert
        Assert.That(weightUpper, Is.EqualTo(weightLower).Within(0.0001),
            "'ACTIVE' must produce the same decay weight as 'active'.");
    }

    #endregion

    #region GetAssumptionDecayWeight — refined status

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - refined assumption within 14-day grace returns 1.0")]
    public void GetAssumptionDecayWeight_RefinedStatus_WithinGracePeriod_ReturnsOne()
    {
        // Arrange: 5 days old — within 14-day refined grace period
        var created = DateTime.UtcNow.AddDays(-5);

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight(created, "refined");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Refined assumption within 14-day grace period must return 1.0.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - refined uses same parameters as active (14d grace, 30d half-life)")]
    public void GetAssumptionDecayWeight_RefinedStatus_SameParametersAsActive()
    {
        // Arrange: old enough to have decayed past grace
        var created = DateTime.UtcNow.AddDays(-60);

        // Act
        var refinedWeight = DecayCalculator.GetAssumptionDecayWeight(created, "refined");
        var activeWeight = DecayCalculator.GetAssumptionDecayWeight(created, "active");

        // Assert
        Assert.That(refinedWeight, Is.EqualTo(activeWeight).Within(0.0001),
            "'refined' and 'active' must use identical decay parameters (14d grace, 30d half-life).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - refined status is case-insensitive")]
    public void GetAssumptionDecayWeight_RefinedStatus_CaseInsensitive()
    {
        // Arrange
        var created = DateTime.UtcNow.AddDays(-50);

        // Act
        var weightLower = DecayCalculator.GetAssumptionDecayWeight(created, "refined");
        var weightUpper = DecayCalculator.GetAssumptionDecayWeight(created, "REFINED");

        // Assert
        Assert.That(weightUpper, Is.EqualTo(weightLower).Within(0.0001),
            "'REFINED' must produce the same decay weight as 'refined'.");
    }

    #endregion

    #region GetAssumptionDecayWeight — default/unknown status

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - unknown status falls to default (active) parameters")]
    public void GetAssumptionDecayWeight_UnknownStatus_UsesActiveParameters()
    {
        // Arrange: 60 days old with an unrecognized status
        var created = DateTime.UtcNow.AddDays(-60);

        // Act
        var unknownWeight = DecayCalculator.GetAssumptionDecayWeight(created, "unknown-garbage");
        var activeWeight = DecayCalculator.GetAssumptionDecayWeight(created, "active");

        // Assert
        Assert.That(unknownWeight, Is.EqualTo(activeWeight).Within(0.0001),
            "Unknown status must fall through to default case, applying active parameters (14d grace, 30d half-life).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - empty string status normalizes and uses active parameters")]
    public void GetAssumptionDecayWeight_EmptyStringStatus_UsesActiveParameters()
    {
        // Arrange: normalizes via (status ?? string.Empty).ToLowerInvariant().Trim()
        var created = DateTime.UtcNow.AddDays(-60);

        // Act
        var emptyWeight = DecayCalculator.GetAssumptionDecayWeight(created, string.Empty);
        var activeWeight = DecayCalculator.GetAssumptionDecayWeight(created, "active");

        // Assert
        Assert.That(emptyWeight, Is.EqualTo(activeWeight).Within(0.0001),
            "Empty string status must normalize and fall through to default/active parameters.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - null status normalizes via null-coalesce and uses active parameters")]
    public void GetAssumptionDecayWeight_NullStatus_UsesActiveParameters()
    {
        // Arrange: null status is handled via (status ?? string.Empty)
        var created = DateTime.UtcNow.AddDays(-60);

        // Act
        var nullWeight = DecayCalculator.GetAssumptionDecayWeight(created, null!);
        var activeWeight = DecayCalculator.GetAssumptionDecayWeight(created, "active");

        // Assert
        Assert.That(nullWeight, Is.EqualTo(activeWeight).Within(0.0001),
            "null status must be coerced to empty string and fall through to active/default parameters.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - whitespace-only status normalizes via Trim and uses active parameters")]
    public void GetAssumptionDecayWeight_WhitespaceStatus_UsesActiveParameters()
    {
        // Arrange
        var created = DateTime.UtcNow.AddDays(-60);

        // Act
        var whitespaceWeight = DecayCalculator.GetAssumptionDecayWeight(created, "   ");
        var activeWeight = DecayCalculator.GetAssumptionDecayWeight(created, "active");

        // Assert
        Assert.That(whitespaceWeight, Is.EqualTo(activeWeight).Within(0.0001),
            "Whitespace-only status must trim to empty and fall through to active/default parameters.");
    }

    #endregion

    #region GetAssumptionDecayWeight — backward-compatible overload (string, DateTime)

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - backward-compat overload (string, DateTime) delegates to (DateTime, string)")]
    public void GetAssumptionDecayWeight_BackwardCompatOverload_DelegatesToPrimaryOverload()
    {
        // Arrange
        var created = DateTime.UtcNow.AddDays(-50);
        const string status = "active";

        // Act — call both overloads; results must be identical
        var primaryWeight = DecayCalculator.GetAssumptionDecayWeight(created, status);
        var compatWeight = DecayCalculator.GetAssumptionDecayWeight(status, created);

        // Assert
        Assert.That(compatWeight, Is.EqualTo(primaryWeight).Within(0.0001),
            "Backward-compatible overload GetAssumptionDecayWeight(string, DateTime) must delegate to " +
            "GetAssumptionDecayWeight(DateTime, string) with swapped arguments.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - backward-compat overload works for validated status")]
    public void GetAssumptionDecayWeight_BackwardCompatOverload_ValidatedReturnsOne()
    {
        // Arrange: old validated assumption
        var created = DateTime.UtcNow.AddDays(-180);

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight("validated", created);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Backward-compat overload with 'validated' must still return 1.0.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM FR-7.8 - backward-compat overload works for invalidated status")]
    public void GetAssumptionDecayWeight_BackwardCompatOverload_InvalidatedDecays()
    {
        // Arrange: 7 + 14 days = one half-life past invalidated grace
        var created = DateTime.UtcNow.AddDays(-(7 + 14));

        // Act
        var weight = DecayCalculator.GetAssumptionDecayWeight("invalidated", created);

        // Assert
        Assert.That(weight, Is.EqualTo(0.5).Within(0.01),
            "Backward-compat overload with 'invalidated' must apply aggressive decay.");
    }

    #endregion

    #region GetDecayWeight(DateTime, DateTime?, string) — lastAccessed branching

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.6.1 - null lastAccessed uses created as reference date")]
    public void GetDecayWeight_DateTime_NullLastAccessed_UsesCreatedAsReference()
    {
        // Arrange: created 60 days ago, no lastAccessed
        var created = DateTime.UtcNow.AddDays(-60);
        DateTime? lastAccessed = null;
        var filePath = "/home/user/maenifold/memory/notes/test.md";

        // Act
        var weight = DecayCalculator.GetDecayWeight(created, lastAccessed, filePath);

        // Assert: weight should reflect 60-day-old content decaying from created date
        Assert.That(weight, Is.LessThan(1.0),
            "With null lastAccessed, the reference date is 'created'. " +
            "A 60-day-old file with 28-day default grace must have a weight below 1.0.");
        Assert.That(weight, Is.GreaterThan(0.0),
            "Decay is asymptotic — weight should remain above 0.0.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.6.1 - non-null lastAccessed uses lastAccessed as reference (access boosting)")]
    public void GetDecayWeight_DateTime_NonNullLastAccessed_UsesLastAccessedAsReference()
    {
        // Arrange: created 60 days ago, but last accessed 1 day ago (access boosting)
        var created = DateTime.UtcNow.AddDays(-60);
        DateTime? lastAccessed = DateTime.UtcNow.AddDays(-1);
        var filePath = "/home/user/maenifold/memory/notes/test.md";

        // Act
        var weight = DecayCalculator.GetDecayWeight(created, lastAccessed, filePath);

        // Assert: weight should reflect 1-day-old content (within default 28-day grace)
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "With lastAccessed = 1 day ago, reference date is lastAccessed. " +
            "A 1-day-old reference within the 28-day default grace must return 1.0 (access boosting).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.6.1 - non-null lastAccessed produces higher weight than null lastAccessed")]
    public void GetDecayWeight_DateTime_NonNullLastAccessed_HigherWeightThanNullLastAccessed()
    {
        // Arrange: same created date, but one has a recent lastAccessed
        var created = DateTime.UtcNow.AddDays(-60);
        var filePath = "/home/user/maenifold/memory/notes/test.md";

        // Act
        var weightWithoutBoost = DecayCalculator.GetDecayWeight(created, null, filePath);
        var weightWithBoost = DecayCalculator.GetDecayWeight(created, DateTime.UtcNow.AddDays(-1), filePath);

        // Assert
        Assert.That(weightWithBoost, Is.GreaterThan(weightWithoutBoost),
            "Access boosting (recent lastAccessed) must yield higher weight than using created date alone.");
    }

    #endregion

    #region GetGracePeriodForPath edge cases (via GetMemoryDecayWeight)

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.2 - null filePath returns DefaultGracePeriodDays")]
    public void GetMemoryDecayWeight_NullPath_UsesDefaultGracePeriod()
    {
        // Arrange: created exactly at the default grace period boundary
        // DefaultGracePeriodDays = 28 — content at day 28 must return 1.0
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysDefault);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(null!, created);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "null filePath must fall back to DefaultGracePeriodDays (28). " +
            "Content at exactly day 28 must still be within grace (weight = 1.0).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.2 - empty string filePath returns DefaultGracePeriodDays")]
    public void GetMemoryDecayWeight_EmptyPath_UsesDefaultGracePeriod()
    {
        // Arrange: content at the default grace boundary
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysDefault);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(string.Empty, created);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Empty string filePath must fall back to DefaultGracePeriodDays (28).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.1 - Windows backslash path with sequential segment uses SequentialGracePeriodDays")]
    public void GetMemoryDecayWeight_WindowsBackslashPath_Sequential_UsesSequentialGracePeriod()
    {
        // Arrange: Windows-style path containing thinking\sequential
        // After normalization (Replace('\\', '/')) → "thinking/sequential"
        var windowsStylePath = @"C:\Users\user\maenifold\memory\thinking\sequential\session.md";

        // Content at the sequential grace boundary (7 days) must return 1.0
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysSequential);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(windowsStylePath, created);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Windows-style backslash path must be normalized to forward slashes. " +
            "thinking\\sequential must be treated as thinking/sequential (7-day grace).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.1 - path with /sequential/ substring (not thinking/sequential) uses SequentialGracePeriodDays")]
    public void GetMemoryDecayWeight_PathWithSlashSequentialSlash_UsesSequentialGracePeriod()
    {
        // Arrange: path that matches "/sequential/" but not "thinking/sequential"
        var path = "/home/user/maenifold/memory/archives/sequential/old-session.md";
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysSequential);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(path, created);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Path containing '/sequential/' must use SequentialGracePeriodDays (7d).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.1a - path with /workflows/ substring (not thinking/workflows) uses WorkflowsGracePeriodDays")]
    public void GetMemoryDecayWeight_PathWithSlashWorkflowsSlash_UsesWorkflowsGracePeriod()
    {
        // Arrange: path that matches "/workflows/" but not "thinking/workflows"
        var path = "/home/user/maenifold/memory/archives/workflows/old-workflow.md";
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysWorkflows);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(path, created);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Path containing '/workflows/' must use WorkflowsGracePeriodDays (14d).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.2 - regular (non-sequential, non-workflow) path uses DefaultGracePeriodDays")]
    public void GetMemoryDecayWeight_RegularPath_UsesDefaultGracePeriod()
    {
        // Arrange: plain notes path — should use 28-day default grace
        var path = "/home/user/maenifold/memory/notes/my-note.md";
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysDefault);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(path, created);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Regular non-sequential, non-workflow path must use DefaultGracePeriodDays (28d).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.1 - sequential path gives shorter effective grace than default path")]
    public void GetMemoryDecayWeight_SequentialPath_ShorterGraceThanDefault()
    {
        // Arrange: content at 10 days old
        // Sequential grace = 7d → already decaying at day 10
        // Default grace = 28d → still in grace at day 10
        var created = DateTime.UtcNow.AddDays(-10);
        var sequentialPath = "/home/user/maenifold/memory/thinking/sequential/session.md";
        var regularPath = "/home/user/maenifold/memory/notes/note.md";

        // Act
        var sequentialWeight = DecayCalculator.GetMemoryDecayWeight(sequentialPath, created);
        var defaultWeight = DecayCalculator.GetMemoryDecayWeight(regularPath, created);

        // Assert
        Assert.That(sequentialWeight, Is.LessThan(defaultWeight),
            "Sequential path (7d grace) must decay faster than default path (28d grace) at day 10.");
        Assert.That(defaultWeight, Is.EqualTo(1.0).Within(0.0001),
            "Default path (28d grace) must still be at full weight at day 10.");
    }

    #endregion

    #region GetDecayWeight(int, int, int, string) — power-law branch

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.5 - power-law within grace period returns 1.0")]
    public void GetDecayWeight_PowerLaw_WithinGracePeriod_ReturnsOne()
    {
        // Arrange: age within grace period
        var ageDays = 5;
        var gracePeriodDays = 14;
        var halfLifeDays = 30;

        // Act
        var weight = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "power-law");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Power-law within grace period must return 1.0 (grace period is applied before the formula).");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.5 - power-law at grace boundary returns 1.0")]
    public void GetDecayWeight_PowerLaw_AtGraceBoundary_ReturnsOne()
    {
        // Arrange: age exactly at the grace period boundary
        var ageDays = 14;
        var gracePeriodDays = 14;
        var halfLifeDays = 30;

        // Act
        var weight = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "power-law");

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Power-law at exactly the grace period boundary must return 1.0.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.5 - power-law past grace period calculates effective age decay")]
    public void GetDecayWeight_PowerLaw_PastGracePeriod_CalculatesDecay()
    {
        // Arrange: 14 + 4 = 18 days old; effective age = 4 days
        // Power-law: weight = 1.0 * 4^(-0.5) = 0.5
        var ageDays = 14 + 4; // 18
        var gracePeriodDays = 14;
        var halfLifeDays = 30;

        // Act
        var weight = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "power-law");

        // Assert
        Assert.That(weight, Is.EqualTo(0.5).Within(0.0001),
            "Power-law with effectiveAge=4 must return 1.0 * 4^(-0.5) = 0.5.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.5 - power-law produces different result from exponential past grace")]
    public void GetDecayWeight_PowerLaw_ProducesDifferentResultFromExponential()
    {
        // Arrange: well past grace period to produce measurable divergence
        var ageDays = 14 + 60; // 60 days of effective age
        var gracePeriodDays = 14;
        var halfLifeDays = 30;

        // Act
        var powerLawWeight = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "power-law");
        var exponentialWeight = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "exponential");

        // Assert
        Assert.That(powerLawWeight, Is.Not.EqualTo(exponentialWeight).Within(0.01),
            "Power-law and exponential must produce meaningfully different weights past the grace period.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.5 - power-law function string is case-insensitive")]
    public void GetDecayWeight_PowerLaw_FunctionStringIsCaseInsensitive()
    {
        // Arrange
        var ageDays = 18;
        var gracePeriodDays = 14;
        var halfLifeDays = 30;

        // Act
        var weightLower = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "power-law");
        var weightUpper = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "POWER-LAW");
        var weightMixed = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "Power-Law");

        // Assert
        Assert.That(weightUpper, Is.EqualTo(weightLower).Within(0.0001),
            "'POWER-LAW' must produce the same result as 'power-law'.");
        Assert.That(weightMixed, Is.EqualTo(weightLower).Within(0.0001),
            "'Power-Law' must produce the same result as 'power-law'.");
    }

    [Test]
    [Description("T-COV-001.DC: RTM NFR-7.5.5 - power-law weight is bounded [0, 1]")]
    public void GetDecayWeight_PowerLaw_WeightIsBounded()
    {
        // Arrange: test extreme ages
        var testAges = new[] { 0, 1, 14, 15, 30, 100, 365, 3650 };
        var gracePeriodDays = 14;
        var halfLifeDays = 30;

        // Act & Assert
        foreach (var ageDays in testAges)
        {
            var weight = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "power-law");
            Assert.That(weight, Is.InRange(0.0, 1.0),
                $"Power-law weight must remain in [0.0, 1.0] for ageDays={ageDays}.");
        }
    }

    #endregion
}
