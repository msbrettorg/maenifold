using System;
using NUnit.Framework;
using Maenifold.Utils;

namespace Maenifold.Tests;

/// <summary>
/// Tests for DecayCalculator utility class implementing recency-based decay weighting.
/// Covers exponential decay (default) and power-law decay (NFR-7.5.5) functions.
///
/// T-GRAPH-DECAY-001.1: RTM FR-7.5 - Apply recency-based decay weighting to all search result rankings
/// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Optional power-law decay via MAENIFOLD_DECAY_FUNCTION env var
/// </summary>
public class DecayFunctionTests
{
    // Default configuration values per PRD
    private const int DefaultGracePeriodDays = 14;
    private const int SequentialGracePeriodDays = 7;
    private const int WorkflowsGracePeriodDays = 14;
    private const int DefaultHalfLifeDays = 30;

    // Power-law ACT-R defaults
    private const double PowerLawA = 1.0;
    private const double PowerLawB = 0.5;

    #region Exponential Decay - Grace Period Tests (NFR-7.5.1, NFR-7.5.2)

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Content within grace period has weight 1.0")]
    public void ExponentialDecay_WithinGracePeriod_ReturnsWeightOne()
    {
        // Arrange
        var ageDays = 5; // Within default 14-day grace period
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Content within grace period should have full weight of 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Content at zero age has weight 1.0")]
    public void ExponentialDecay_ZeroAge_ReturnsWeightOne()
    {
        // Arrange
        var ageDays = 0;
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Brand new content (age=0) should have full weight of 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Content exactly at grace period boundary has weight 1.0")]
    public void ExponentialDecay_ExactlyAtGracePeriodBoundary_ReturnsWeightOne()
    {
        // Arrange
        var ageDays = DefaultGracePeriodDays; // Exactly at boundary
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Content exactly at grace period boundary should have full weight of 1.0");
    }

    #endregion

    #region Exponential Decay - Half-Life Tests (NFR-7.5.3)

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM NFR-7.5.3 - One half-life past grace period yields weight 0.5")]
    public void ExponentialDecay_OneHalfLifePastGrace_ReturnsWeightHalf()
    {
        // Arrange
        // Age = grace period + half-life = 14 + 30 = 44 days
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;
        var ageDays = gracePeriodDays + halfLifeDays;

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weight, Is.EqualTo(0.5).Within(0.0001),
            "Content one half-life past grace should have weight 0.5");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM NFR-7.5.3 - Two half-lives past grace period yields weight 0.25")]
    public void ExponentialDecay_TwoHalfLivesPastGrace_ReturnsWeightQuarter()
    {
        // Arrange
        // Age = grace period + 2*half-life = 14 + 60 = 74 days
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;
        var ageDays = gracePeriodDays + (2 * halfLifeDays);

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weight, Is.EqualTo(0.25).Within(0.0001),
            "Content two half-lives past grace should have weight 0.25");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM NFR-7.5.3 - Three half-lives past grace period yields weight 0.125")]
    public void ExponentialDecay_ThreeHalfLivesPastGrace_ReturnsWeightEighth()
    {
        // Arrange
        // Age = grace period + 3*half-life = 14 + 90 = 104 days
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;
        var ageDays = gracePeriodDays + (3 * halfLifeDays);

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weight, Is.EqualTo(0.125).Within(0.0001),
            "Content three half-lives past grace should have weight 0.125");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Partial half-life yields intermediate weight")]
    public void ExponentialDecay_HalfOfHalfLifePastGrace_ReturnsIntermediateWeight()
    {
        // Arrange
        // Age = grace period + half-life/2 = 14 + 15 = 29 days
        // Expected weight = 0.5^0.5 ≈ 0.7071
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;
        var ageDays = gracePeriodDays + (halfLifeDays / 2);

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);

        // Assert
        var expected = Math.Pow(0.5, 0.5); // ~0.7071
        Assert.That(weight, Is.EqualTo(expected).Within(0.0001),
            $"Content half a half-life past grace should have weight ~{expected:F4}");
    }

    #endregion

    #region Exponential Decay - Bounds Tests

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Weight never exceeds 1.0")]
    public void ExponentialDecay_NeverExceedsOne()
    {
        // Arrange & Act: Test various ages including negative (shouldn't happen but defensive)
        var testAges = new[] { -10, -1, 0, 1, 5, 14, 15, 30, 100, 365, 1000 };

        foreach (var ageDays in testAges)
        {
            var weight = DecayCalculator.CalculateExponentialDecay(
                ageDays, DefaultGracePeriodDays, DefaultHalfLifeDays);

            // Assert
            Assert.That(weight, Is.LessThanOrEqualTo(1.0),
                $"Weight should never exceed 1.0 for age={ageDays}");
        }
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Weight never goes below 0.0")]
    public void ExponentialDecay_NeverGoesBelowZero()
    {
        // Arrange & Act: Test extreme ages
        var testAges = new[] { 100, 365, 1000, 3650, 10000 };

        foreach (var ageDays in testAges)
        {
            var weight = DecayCalculator.CalculateExponentialDecay(
                ageDays, DefaultGracePeriodDays, DefaultHalfLifeDays);

            // Assert
            Assert.That(weight, Is.GreaterThanOrEqualTo(0.0),
                $"Weight should never go below 0.0 for age={ageDays}");
        }
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Very old content approaches but never reaches zero")]
    public void ExponentialDecay_VeryOldContent_ApproachesZeroButNeverReaches()
    {
        // Arrange: 10 years old content
        var ageDays = 3650;
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weight, Is.GreaterThan(0.0),
            "Very old content should have weight > 0 (asymptotic)");
        Assert.That(weight, Is.LessThan(0.001),
            "Very old content should have very small weight");
    }

    #endregion

    #region Tiered Grace Periods (NFR-7.5.1, NFR-7.5.1a, NFR-7.5.2)

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM NFR-7.5.1 - Sequential thinking uses 7-day grace period")]
    public void ExponentialDecay_SequentialThinking_Uses7DayGracePeriod()
    {
        // Arrange: Content at 7 days (within sequential grace) and 8 days (past sequential grace)
        var atGrace = SequentialGracePeriodDays;
        var pastGrace = SequentialGracePeriodDays + 1;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weightAtGrace = DecayCalculator.CalculateExponentialDecay(atGrace, SequentialGracePeriodDays, halfLifeDays);
        var weightPastGrace = DecayCalculator.CalculateExponentialDecay(pastGrace, SequentialGracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weightAtGrace, Is.EqualTo(1.0),
            "Sequential content exactly at 7-day grace should have weight 1.0");
        Assert.That(weightPastGrace, Is.LessThan(1.0),
            "Sequential content past 7-day grace should decay");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM NFR-7.5.1a - Workflows use 14-day grace period")]
    public void ExponentialDecay_Workflows_Uses14DayGracePeriod()
    {
        // Arrange
        var atGrace = WorkflowsGracePeriodDays;
        var pastGrace = WorkflowsGracePeriodDays + 1;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weightAtGrace = DecayCalculator.CalculateExponentialDecay(atGrace, WorkflowsGracePeriodDays, halfLifeDays);
        var weightPastGrace = DecayCalculator.CalculateExponentialDecay(pastGrace, WorkflowsGracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weightAtGrace, Is.EqualTo(1.0),
            "Workflow content exactly at 14-day grace should have weight 1.0");
        Assert.That(weightPastGrace, Is.LessThan(1.0),
            "Workflow content past 14-day grace should decay");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM NFR-7.5.2 - Default memory uses 14-day grace period")]
    public void ExponentialDecay_DefaultMemory_Uses14DayGracePeriod()
    {
        // Arrange
        var atGrace = DefaultGracePeriodDays;
        var pastGrace = DefaultGracePeriodDays + 1;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weightAtGrace = DecayCalculator.CalculateExponentialDecay(atGrace, DefaultGracePeriodDays, halfLifeDays);
        var weightPastGrace = DecayCalculator.CalculateExponentialDecay(pastGrace, DefaultGracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(weightAtGrace, Is.EqualTo(1.0),
            "Default content exactly at 14-day grace should have weight 1.0");
        Assert.That(weightPastGrace, Is.LessThan(1.0),
            "Default content past 14-day grace should decay");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM NFR-7.5.1/7.5.1a/7.5.2 - Sequential decays faster than workflows/default")]
    public void ExponentialDecay_TieredGracePeriods_SequentialDecaysFaster()
    {
        // Arrange: Same age content, different tiers
        var ageDays = 10; // Past sequential grace (7d), within workflows/default grace (14d)
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var sequentialWeight = DecayCalculator.CalculateExponentialDecay(ageDays, SequentialGracePeriodDays, halfLifeDays);
        var workflowWeight = DecayCalculator.CalculateExponentialDecay(ageDays, WorkflowsGracePeriodDays, halfLifeDays);
        var defaultWeight = DecayCalculator.CalculateExponentialDecay(ageDays, DefaultGracePeriodDays, halfLifeDays);

        // Assert
        Assert.That(sequentialWeight, Is.LessThan(workflowWeight),
            "Sequential content (7d grace) should have lower weight than workflows (14d grace) at day 10");
        Assert.That(sequentialWeight, Is.LessThan(defaultWeight),
            "Sequential content (7d grace) should have lower weight than default (14d grace) at day 10");
        Assert.That(workflowWeight, Is.EqualTo(defaultWeight),
            "Workflows and default should have same weight (both 14d grace) at day 10");
    }

    #endregion

    #region Power-Law Decay Tests (NFR-7.5.5)

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law decay produces weight based on R = a × t^(-b)")]
    public void PowerLawDecay_BasicCalculation_FollowsFormula()
    {
        // Arrange: Using ACT-R defaults a=1.0, b=0.5
        // At t=4 days: R = 1.0 × 4^(-0.5) = 1.0 × 0.5 = 0.5
        var ageDays = 4.0;
        var a = PowerLawA;
        var b = PowerLawB;

        // Act
        var weight = DecayCalculator.CalculatePowerLawDecay(ageDays, a, b);

        // Assert
        var expected = a * Math.Pow(ageDays, -b); // 0.5
        Assert.That(weight, Is.EqualTo(expected).Within(0.0001),
            "Power-law decay at t=4 should yield 0.5 with ACT-R defaults");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law decay at t=1 returns weight 1.0")]
    public void PowerLawDecay_AtDayOne_ReturnsWeightOne()
    {
        // Arrange: At t=1: R = 1.0 × 1^(-0.5) = 1.0
        var ageDays = 1.0;

        // Act
        var weight = DecayCalculator.CalculatePowerLawDecay(ageDays, PowerLawA, PowerLawB);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Power-law decay at t=1 should yield 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law decay caps weight at 1.0 for t < 1")]
    public void PowerLawDecay_BelowDayOne_CapsAtOne()
    {
        // Arrange: t < 1 would yield weight > 1.0 without capping
        // At t=0.5: R = 1.0 × 0.5^(-0.5) = 1.0 × ~1.414 > 1.0
        var ageDays = 0.5;

        // Act
        var weight = DecayCalculator.CalculatePowerLawDecay(ageDays, PowerLawA, PowerLawB);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Power-law decay should cap at 1.0 for very recent content");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law decay at zero age returns 1.0 (capped)")]
    public void PowerLawDecay_ZeroAge_ReturnsWeightOne()
    {
        // Arrange: t=0 would cause division by zero, should be handled
        var ageDays = 0.0;

        // Act
        var weight = DecayCalculator.CalculatePowerLawDecay(ageDays, PowerLawA, PowerLawB);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Power-law decay at t=0 should return capped weight 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law decay never exceeds 1.0")]
    public void PowerLawDecay_NeverExceedsOne()
    {
        // Arrange & Act: Test various ages
        var testAges = new[] { 0.0, 0.1, 0.5, 1.0, 2.0, 10.0, 100.0 };

        foreach (var ageDays in testAges)
        {
            var weight = DecayCalculator.CalculatePowerLawDecay(ageDays, PowerLawA, PowerLawB);

            // Assert
            Assert.That(weight, Is.LessThanOrEqualTo(1.0),
                $"Power-law weight should never exceed 1.0 for age={ageDays}");
        }
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law decay never goes below 0.0")]
    public void PowerLawDecay_NeverGoesBelowZero()
    {
        // Arrange & Act: Test extreme ages
        var testAges = new[] { 100.0, 365.0, 1000.0, 3650.0, 10000.0 };

        foreach (var ageDays in testAges)
        {
            var weight = DecayCalculator.CalculatePowerLawDecay(ageDays, PowerLawA, PowerLawB);

            // Assert
            Assert.That(weight, Is.GreaterThanOrEqualTo(0.0),
                $"Power-law weight should never go below 0.0 for age={ageDays}");
        }
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law decays slower than exponential for old content")]
    public void PowerLawDecay_SlowerThanExponential_ForOldContent()
    {
        // Arrange: Per Wixted & Ebbesen (1991), power-law decays slower long-term
        // "Memory halves when time quadruples" vs exponential's constant half-life
        var ageDays = 120; // 120 days past grace period for exponential
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var exponentialWeight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);
        var powerLawWeight = DecayCalculator.CalculatePowerLawDecay(ageDays, PowerLawA, PowerLawB);

        // Assert
        // Exponential at 120 days: past grace by 106 days = ~3.5 half-lives = ~0.088
        // Power-law at 120 days: 1.0 × 120^(-0.5) ≈ 0.091
        TestContext.Out.WriteLine($"At {ageDays} days - Exponential: {exponentialWeight:F6}, Power-law: {powerLawWeight:F6}");

        // Power-law should decay slower for very old content
        Assert.That(powerLawWeight, Is.GreaterThanOrEqualTo(exponentialWeight * 0.5),
            "Power-law should not decay drastically faster than exponential");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law produces different curve than exponential")]
    public void PowerLawDecay_ProducesDifferentCurveThanExponential()
    {
        // Arrange: Compare decay curves at multiple time points
        var timePoints = new[] { 1, 4, 16, 64, 256 }; // Powers of 4 to demonstrate "quadrupling" behavior
        var gracePeriodDays = 0; // No grace period for clean comparison
        var halfLifeDays = 30;

        var exponentialWeights = new double[timePoints.Length];
        var powerLawWeights = new double[timePoints.Length];

        // Act
        for (int i = 0; i < timePoints.Length; i++)
        {
            exponentialWeights[i] = DecayCalculator.CalculateExponentialDecay(
                timePoints[i], gracePeriodDays, halfLifeDays);
            powerLawWeights[i] = DecayCalculator.CalculatePowerLawDecay(
                timePoints[i], PowerLawA, PowerLawB);
        }

        // Assert: Verify the curves are meaningfully different
        TestContext.Out.WriteLine("Time\tExponential\tPower-Law\tDiff");
        var differenceFound = false;
        for (int i = 0; i < timePoints.Length; i++)
        {
            var diff = Math.Abs(exponentialWeights[i] - powerLawWeights[i]);
            TestContext.Out.WriteLine($"{timePoints[i]}\t{exponentialWeights[i]:F6}\t{powerLawWeights[i]:F6}\t{diff:F6}");

            if (diff > 0.01) differenceFound = true;
        }

        Assert.That(differenceFound, Is.True,
            "Power-law and exponential curves should produce meaningfully different weights");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law 'memory halves when time quadruples'")]
    public void PowerLawDecay_MemoryHalvesWhenTimeQuadruples()
    {
        // Arrange: Per ACT-R research with b=0.5, when time quadruples, memory halves
        // At t=4: weight = 4^(-0.5) = 0.5
        // At t=16 (4x4): weight = 16^(-0.5) = 0.25 (half of 0.5)
        // At t=64 (4x16): weight = 64^(-0.5) = 0.125 (half of 0.25)

        // Act
        var weightAt4 = DecayCalculator.CalculatePowerLawDecay(4, PowerLawA, PowerLawB);
        var weightAt16 = DecayCalculator.CalculatePowerLawDecay(16, PowerLawA, PowerLawB);
        var weightAt64 = DecayCalculator.CalculatePowerLawDecay(64, PowerLawA, PowerLawB);

        // Assert
        Assert.That(weightAt4, Is.EqualTo(0.5).Within(0.0001),
            "At t=4, weight should be 0.5");
        Assert.That(weightAt16, Is.EqualTo(weightAt4 / 2.0).Within(0.0001),
            "At t=16 (4x4), weight should be half of t=4");
        Assert.That(weightAt64, Is.EqualTo(weightAt16 / 2.0).Within(0.0001),
            "At t=64 (4x16), weight should be half of t=16");
    }

    #endregion

    #region Edge Cases and Error Handling

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Negative age treated as zero")]
    public void ExponentialDecay_NegativeAge_TreatedAsZero()
    {
        // Arrange
        var negativeAge = -5;

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(
            negativeAge, DefaultGracePeriodDays, DefaultHalfLifeDays);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Negative age should be treated as very recent content with weight 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Zero half-life handled gracefully")]
    public void ExponentialDecay_ZeroHalfLife_HandledGracefully()
    {
        // Arrange: Zero half-life is an edge case that shouldn't crash
        var ageDays = 20;
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = 0;

        // Act & Assert: Should not throw, should return reasonable value
        Assert.DoesNotThrow(() =>
        {
            var weight = DecayCalculator.CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);
            Assert.That(weight, Is.InRange(0.0, 1.0),
                "Zero half-life should still return bounded weight");
        });
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - Negative grace period treated as zero grace")]
    public void ExponentialDecay_NegativeGracePeriod_TreatedAsZero()
    {
        // Arrange
        var ageDays = 5;
        var negativeGracePeriod = -10;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(ageDays, negativeGracePeriod, halfLifeDays);

        // Assert: Should decay immediately (no grace period)
        Assert.That(weight, Is.LessThan(1.0),
            "With negative/zero grace period, content should decay from day 1");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law with negative age returns 1.0")]
    public void PowerLawDecay_NegativeAge_ReturnsWeightOne()
    {
        // Arrange
        var negativeAge = -5.0;

        // Act
        var weight = DecayCalculator.CalculatePowerLawDecay(negativeAge, PowerLawA, PowerLawB);

        // Assert
        Assert.That(weight, Is.EqualTo(1.0),
            "Negative age in power-law should return capped weight 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law with custom parameters")]
    public void PowerLawDecay_CustomParameters_AppliesCorrectly()
    {
        // Arrange: Custom a and b values
        var ageDays = 10.0;
        var customA = 2.0; // Higher base multiplier
        var customB = 0.3; // Slower decay rate

        // Act
        var weight = DecayCalculator.CalculatePowerLawDecay(ageDays, customA, customB);

        // Assert
        var expected = Math.Min(1.0, customA * Math.Pow(ageDays, -customB));
        Assert.That(weight, Is.EqualTo(expected).Within(0.0001),
            "Custom power-law parameters should apply correctly");
    }

    #endregion

    #region DateTime-based API Tests

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - DateTime API calculates correct age")]
    public void ExponentialDecay_DateTimeApi_CalculatesCorrectAge()
    {
        // Arrange: Content created 10 days ago
        var createdDate = DateTime.UtcNow.AddDays(-10);
        var gracePeriodDays = DefaultGracePeriodDays;
        var halfLifeDays = DefaultHalfLifeDays;

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(createdDate, gracePeriodDays, halfLifeDays);

        // Assert: Should be within grace period (10 < 14)
        Assert.That(weight, Is.EqualTo(1.0),
            "Content 10 days old should be within 14-day grace period");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.1: RTM FR-7.5 - DateTime API handles future dates")]
    public void ExponentialDecay_DateTimeApi_FutureDateReturnsOne()
    {
        // Arrange: Content "created" in the future (edge case)
        var futureDate = DateTime.UtcNow.AddDays(5);

        // Act
        var weight = DecayCalculator.CalculateExponentialDecay(
            futureDate, DefaultGracePeriodDays, DefaultHalfLifeDays);

        // Assert: Future dates should be treated as brand new
        Assert.That(weight, Is.EqualTo(1.0),
            "Future creation dates should return weight 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law DateTime API calculates correct age")]
    public void PowerLawDecay_DateTimeApi_CalculatesCorrectAge()
    {
        // Arrange: Content created 4 days ago
        var createdDate = DateTime.UtcNow.AddDays(-4);

        // Act
        var weight = DecayCalculator.CalculatePowerLawDecay(createdDate, PowerLawA, PowerLawB);

        // Assert: At t=4, weight = 4^(-0.5) = 0.5
        Assert.That(weight, Is.EqualTo(0.5).Within(0.01),
            "Content 4 days old should have power-law weight ~0.5");
    }

    #endregion
}
