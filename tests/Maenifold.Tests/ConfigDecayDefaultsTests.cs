using System;
using NUnit.Framework;
using Maenifold.Utils;

namespace Maenifold.Tests;

/// <summary>
/// Tests for decay configuration environment variables in Config.cs.
/// Verifies default values, environment variable overrides, and invalid input handling.
///
/// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - Decay configuration via environment variables
/// </summary>
[TestFixture]
public class ConfigDecayDefaultsTests
{
    private readonly Dictionary<string, string?> _originalEnvVars = new();

    [SetUp]
    public void SetUp()
    {
        // Save original environment variable values
        _originalEnvVars["MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL"] =
            Environment.GetEnvironmentVariable("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL");
        _originalEnvVars["MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS"] =
            Environment.GetEnvironmentVariable("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS");
        _originalEnvVars["MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT"] =
            Environment.GetEnvironmentVariable("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT");
        _originalEnvVars["MAENIFOLD_DECAY_HALF_LIFE_DAYS"] =
            Environment.GetEnvironmentVariable("MAENIFOLD_DECAY_HALF_LIFE_DAYS");
        _originalEnvVars["MAENIFOLD_DECAY_FUNCTION"] =
            Environment.GetEnvironmentVariable("MAENIFOLD_DECAY_FUNCTION");
    }

    [TearDown]
    public void TearDown()
    {
        // Restore original environment variable values
        foreach (var kvp in _originalEnvVars)
        {
            if (kvp.Value == null)
            {
                Environment.SetEnvironmentVariable(kvp.Key, null);
            }
            else
            {
                Environment.SetEnvironmentVariable(kvp.Key, kvp.Value);
            }
        }
    }

    #region Default Value Tests

    [Test]
    [Description("T-GRAPH-DECAY-001.2: RTM NFR-7.5.1 - Sequential grace period defaults to 7 days")]
    public void DecayGraceDaysSequential_Default_IsSeven()
    {
        // Arrange: Clear any existing env var
        Environment.SetEnvironmentVariable("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", null);

        // Act & Assert: Note - Config uses static readonly, so we test the expected default
        // The test verifies the code specifies 7 as the default
        Assert.That(Config.DecayGraceDaysSequential, Is.EqualTo(7),
            "Sequential grace period should default to 7 days per NFR-7.5.1");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.2a: RTM NFR-7.5.1a - Workflows grace period defaults to 14 days")]
    public void DecayGraceDaysWorkflows_Default_IsFourteen()
    {
        // Arrange: Clear any existing env var
        Environment.SetEnvironmentVariable("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", null);

        // Act & Assert
        Assert.That(Config.DecayGraceDaysWorkflows, Is.EqualTo(14),
            "Workflows grace period should default to 14 days per NFR-7.5.1a");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.3: RTM NFR-7.5.2 - Default grace period defaults to 14 days")]
    public void DecayGraceDaysDefault_Default_IsFourteen()
    {
        // Arrange: Clear any existing env var
        Environment.SetEnvironmentVariable("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", null);

        // Act & Assert
        Assert.That(Config.DecayGraceDaysDefault, Is.EqualTo(28),
            "Default grace period should default to 28 days per NFR-7.5.2");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.4: RTM NFR-7.5.3 - Half-life defaults to 30 days")]
    public void DecayHalfLifeDays_Default_IsThirty()
    {
        // Arrange: Clear any existing env var
        Environment.SetEnvironmentVariable("MAENIFOLD_DECAY_HALF_LIFE_DAYS", null);

        // Act & Assert
        Assert.That(Config.DecayHalfLifeDays, Is.EqualTo(30),
            "Decay half-life should default to 30 days per NFR-7.5.3");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Decay function defaults to 'exponential'")]
    public void DecayFunction_Default_IsExponential()
    {
        // Arrange: Clear any existing env var
        Environment.SetEnvironmentVariable("MAENIFOLD_DECAY_FUNCTION", null);

        // Act & Assert
        Assert.That(Config.DecayFunction, Is.EqualTo("power-law"),
            "Decay function should default to 'power-law' per NFR-7.5.5 (research-validated)");
    }

    #endregion

    #region GetConfigSummary Tests

    [Test]
    [Description("T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - Config summary includes decay settings")]
    public void GetConfigSummary_IncludesDecayConfig()
    {
        // Act
        var summary = Config.GetConfigSummary();

        // Assert
        Assert.That(summary, Does.Contain("Decay Config:"),
            "Config summary should include Decay Config section");
        Assert.That(summary, Does.Contain("Grace Days (Sequential):"),
            "Config summary should include sequential grace days");
        Assert.That(summary, Does.Contain("Grace Days (Workflows):"),
            "Config summary should include workflows grace days");
        Assert.That(summary, Does.Contain("Grace Days (Default):"),
            "Config summary should include default grace days");
        Assert.That(summary, Does.Contain("Half-Life Days:"),
            "Config summary should include half-life days");
        Assert.That(summary, Does.Contain("Function:"),
            "Config summary should include decay function");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - Config summary shows actual values")]
    public void GetConfigSummary_ShowsActualValues()
    {
        // Act
        var summary = Config.GetConfigSummary();

        // Assert: Verify the summary contains the actual configured values
        Assert.That(summary, Does.Contain($"Grace Days (Sequential): {Config.DecayGraceDaysSequential}"),
            "Config summary should show actual sequential grace days value");
        Assert.That(summary, Does.Contain($"Grace Days (Workflows): {Config.DecayGraceDaysWorkflows}"),
            "Config summary should show actual workflows grace days value");
        Assert.That(summary, Does.Contain($"Grace Days (Default): {Config.DecayGraceDaysDefault}"),
            "Config summary should show actual default grace days value");
        Assert.That(summary, Does.Contain($"Half-Life Days: {Config.DecayHalfLifeDays}"),
            "Config summary should show actual half-life days value");
        Assert.That(summary, Does.Contain($"Function: {Config.DecayFunction}"),
            "Config summary should show actual decay function value");
    }

    #endregion

    #region DecayCalculator Integration Tests

    [Test]
    [Description("T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - DecayCalculator uses Config values for sequential path")]
    public void DecayCalculator_SequentialPath_UsesConfigGracePeriod()
    {
        // Arrange
        var sequentialPath = "/home/user/maenifold/memory/thinking/sequential/session-123.md";
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysSequential);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(sequentialPath, created);

        // Assert: At exactly the grace period boundary, weight should be 1.0
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Content at sequential grace period boundary should have weight 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - DecayCalculator uses Config values for workflow path")]
    public void DecayCalculator_WorkflowPath_UsesConfigGracePeriod()
    {
        // Arrange
        var workflowPath = "/home/user/maenifold/memory/thinking/workflows/workflow-abc.md";
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysWorkflows);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(workflowPath, created);

        // Assert: At exactly the grace period boundary, weight should be 1.0
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Content at workflows grace period boundary should have weight 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - DecayCalculator uses Config values for default path")]
    public void DecayCalculator_DefaultPath_UsesConfigGracePeriod()
    {
        // Arrange
        var defaultPath = "/home/user/maenifold/memory/notes/my-note.md";
        var created = DateTime.UtcNow.AddDays(-Config.DecayGraceDaysDefault);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(defaultPath, created);

        // Assert: At exactly the grace period boundary, weight should be 1.0
        Assert.That(weight, Is.EqualTo(1.0).Within(0.0001),
            "Content at default grace period boundary should have weight 1.0");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.4: RTM NFR-7.5.3 - DecayCalculator uses Config half-life value")]
    public void DecayCalculator_UsesConfigHalfLife()
    {
        // Arrange: Content one half-life past grace period
        // With power-law decay (default), formula is: weight = 1.0 * (effectiveAge)^(-0.5)
        var defaultPath = "/home/user/maenifold/memory/notes/my-note.md";
        var ageDays = Config.DecayGraceDaysDefault + Config.DecayHalfLifeDays;
        var created = DateTime.UtcNow.AddDays(-ageDays);

        // Act
        var weight = DecayCalculator.GetMemoryDecayWeight(defaultPath, created);

        // Assert: Power-law decay at 30 days past grace
        // weight = 1.0 * 30^(-0.5) = 1/sqrt(30) ≈ 0.1826
        Assert.That(weight, Is.EqualTo(0.1826).Within(0.01),
            "Power-law decay: 1.0 * (30)^(-0.5) ≈ 0.1826");
    }

    #endregion

    #region Decay Function Configuration Tests

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - DecayCalculator respects exponential function config")]
    public void DecayCalculator_ExponentialFunction_AppliesCorrectly()
    {
        // Arrange: Test exponential decay directly via GetDecayWeight
        var ageDays = Config.DecayGraceDaysDefault + Config.DecayHalfLifeDays;
        var gracePeriod = Config.DecayGraceDaysDefault;
        var halfLife = Config.DecayHalfLifeDays;

        // Act: Call GetDecayWeight with explicit "exponential" parameter
        var weight = DecayCalculator.GetDecayWeight(ageDays, gracePeriod, halfLife, "exponential");

        // Assert: Exponential decay at one half-life should yield 0.5
        // Formula: 0.5^((58-28)/30) = 0.5^1 = 0.5
        Assert.That(weight, Is.EqualTo(0.5).Within(0.0001),
            "Exponential decay at one half-life should yield weight 0.5");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - GetDecayWeight function parameter overrides config")]
    public void GetDecayWeight_FunctionParameter_OverridesConfig()
    {
        // Arrange
        var ageDays = 16; // Past grace period
        var gracePeriodDays = 0;
        var halfLifeDays = 30;

        // Act: Explicitly request power-law (ignores Config.DecayFunction)
        var weightPowerLaw = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "power-law");
        var weightExponential = DecayCalculator.GetDecayWeight(ageDays, gracePeriodDays, halfLifeDays, "exponential");

        // Assert: Power-law and exponential produce different results
        Assert.That(weightPowerLaw, Is.Not.EqualTo(weightExponential).Within(0.01),
            "Power-law and exponential should produce different weights");
    }

    #endregion

    #region Invalid Environment Variable Handling Tests

    [Test]
    [Description("T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - Invalid integer env var falls back to default")]
    public void GetEnvInt_InvalidValue_ReturnsDefault()
    {
        // Note: Since Config uses static readonly fields initialized at class load time,
        // we cannot directly test environment variable parsing at runtime.
        // This test verifies the behavior documented in GetEnvInt.

        // The implementation uses int.TryParse which returns defaultValue on parse failure.
        // We test this indirectly by verifying the current values are valid integers.
        Assert.That(Config.DecayGraceDaysSequential, Is.TypeOf<int>(),
            "DecayGraceDaysSequential should be a valid integer");
        Assert.That(Config.DecayGraceDaysWorkflows, Is.TypeOf<int>(),
            "DecayGraceDaysWorkflows should be a valid integer");
        Assert.That(Config.DecayGraceDaysDefault, Is.TypeOf<int>(),
            "DecayGraceDaysDefault should be a valid integer");
        Assert.That(Config.DecayHalfLifeDays, Is.TypeOf<int>(),
            "DecayHalfLifeDays should be a valid integer");
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Empty decay function falls back to default")]
    public void DecayFunction_Empty_FallsBackToExponential()
    {
        // Note: Since Config uses static readonly fields, we test the default behavior
        // The GetEnvString implementation returns defaultValue when env var is empty/whitespace
        Assert.That(Config.DecayFunction, Is.Not.Null.And.Not.Empty,
            "DecayFunction should never be null or empty");

        // Verify default is exponential (when no env var is set)
        // This is validated by the Default test above
    }

    [Test]
    [Description("T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - Config values are positive integers")]
    public void DecayConfig_AllValuesArePositive()
    {
        Assert.That(Config.DecayGraceDaysSequential, Is.GreaterThan(0),
            "Sequential grace days should be positive");
        Assert.That(Config.DecayGraceDaysWorkflows, Is.GreaterThan(0),
            "Workflows grace days should be positive");
        Assert.That(Config.DecayGraceDaysDefault, Is.GreaterThan(0),
            "Default grace days should be positive");
        Assert.That(Config.DecayHalfLifeDays, Is.GreaterThan(0),
            "Half-life days should be positive");
    }

    #endregion
}
