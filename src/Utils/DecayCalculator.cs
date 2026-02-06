// T-GRAPH-DECAY-001.1, T-GRAPH-DECAY-001.6, T-GRAPH-DECAY-004.1-3: RTM FR-7.5, FR-7.8, NFR-7.5.5
// Decay weight calculation for memory files and assumptions
// Implements power-law decay (ACT-R model, default) and exponential decay (opt-in)

using System;

namespace Maenifold.Utils;

/// <summary>
/// Calculates decay weights for memory content based on age and type.
/// Implements recency-based decay per FR-7.5 and status-based assumption decay per FR-7.8.
/// Supports exponential decay (default) and power-law decay (NFR-7.5.5).
/// Configuration values are sourced from Config.cs environment variables.
/// </summary>
public static class DecayCalculator
{
    // T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x - Configuration values from environment variables via Config.cs
    private static int DefaultGracePeriodDays => Config.DecayGraceDaysDefault;
    private static int SequentialGracePeriodDays => Config.DecayGraceDaysSequential;
    private static int WorkflowsGracePeriodDays => Config.DecayGraceDaysWorkflows;
    private static int DefaultHalfLifeDays => Config.DecayHalfLifeDays;
    private static string ConfiguredDecayFunction => Config.DecayFunction;

    // Power-law ACT-R defaults (Wixted & Ebbesen, 1991)
    private const double DefaultPowerLawA = 1.0;
    private const double DefaultPowerLawB = 0.5;

    #region Core Decay Functions

    /// <summary>
    /// Calculates exponential decay weight based on age and configuration.
    /// Formula: if (age &lt;= gracePeriod) weight = 1.0; else weight = 0.5^((age - gracePeriod) / halfLife)
    /// </summary>
    /// <param name="ageDays">Age of content in days</param>
    /// <param name="gracePeriodDays">Grace period before decay begins</param>
    /// <param name="halfLifeDays">Time for weight to halve after grace period</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double CalculateExponentialDecay(int ageDays, int gracePeriodDays, int halfLifeDays)
    {
        // T-GRAPH-DECAY-001.1: RTM FR-7.5 - Negative age treated as zero (very recent content)
        if (ageDays < 0)
        {
            return 1.0;
        }

        // T-GRAPH-DECAY-001.1: RTM NFR-7.5.1/7.5.2 - Content within grace period has weight 1.0
        // Treat negative grace period as zero
        var effectiveGrace = Math.Max(0, gracePeriodDays);
        if (ageDays <= effectiveGrace)
        {
            return 1.0;
        }

        // T-GRAPH-DECAY-001.1: RTM FR-7.5 - Zero half-life handled gracefully
        if (halfLifeDays <= 0)
        {
            // Instant decay to near-zero (but not exactly zero to maintain asymptotic behavior)
            return 0.0;
        }

        // T-GRAPH-DECAY-001.1: RTM NFR-7.5.3 - Exponential decay formula: 0.5^((age - grace) / halfLife)
        var effectiveAge = ageDays - effectiveGrace;
        var exponent = effectiveAge / (double)halfLifeDays;
        var weight = Math.Pow(0.5, exponent);

        // Ensure bounds [0.0, 1.0]
        return Math.Max(0.0, Math.Min(1.0, weight));
    }

    /// <summary>
    /// Calculates exponential decay weight using DateTime for age calculation.
    /// </summary>
    /// <param name="createdDate">Creation timestamp of content</param>
    /// <param name="gracePeriodDays">Grace period before decay begins</param>
    /// <param name="halfLifeDays">Time for weight to halve after grace period</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double CalculateExponentialDecay(DateTime createdDate, int gracePeriodDays, int halfLifeDays)
    {
        var ageDays = (int)(DateTime.UtcNow - createdDate).TotalDays;
        return CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);
    }

    /// <summary>
    /// Calculates power-law decay weight per ACT-R cognitive architecture.
    /// Formula: weight = min(1.0, a * t^(-b)) where t is age in days
    /// "Memory halves when time quadruples" (Wixted &amp; Ebbesen, 1991)
    /// </summary>
    /// <param name="ageDays">Age of content in days</param>
    /// <param name="a">Scale parameter (default 1.0)</param>
    /// <param name="b">Decay rate parameter (default 0.5 per ACT-R)</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double CalculatePowerLawDecay(double ageDays, double a, double b)
    {
        // T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Handle edge cases
        // Negative age or zero age: treat as very recent content (weight = 1.0)
        if (ageDays <= 0)
        {
            return 1.0;
        }

        // T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Power-law formula: R = a * t^(-b)
        // At t=1: weight = a * 1^(-b) = a
        // At t=4 with b=0.5: weight = 1.0 * 4^(-0.5) = 0.5
        var weight = a * Math.Pow(ageDays, -b);

        // T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Cap at 1.0 (for t < 1 and high 'a' values)
        // Ensure bounds [0.0, 1.0]
        return Math.Max(0.0, Math.Min(1.0, weight));
    }

    /// <summary>
    /// Calculates power-law decay weight using DateTime for age calculation.
    /// </summary>
    /// <param name="createdDate">Creation timestamp of content</param>
    /// <param name="a">Scale parameter (default 1.0)</param>
    /// <param name="b">Decay rate parameter (default 0.5 per ACT-R)</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double CalculatePowerLawDecay(DateTime createdDate, double a, double b)
    {
        var ageDays = (DateTime.UtcNow - createdDate).TotalDays;
        return CalculatePowerLawDecay(ageDays, a, b);
    }

    #endregion

    #region High-Level API

    /// <summary>
    /// Gets the decay weight for content with configurable decay function.
    /// </summary>
    /// <param name="ageDays">Age of content in days</param>
    /// <param name="gracePeriodDays">Grace period before decay begins</param>
    /// <param name="halfLifeDays">Time for weight to halve after grace period (exponential only)</param>
    /// <param name="function">Decay function: "exponential" (default) or "power-law"</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double GetDecayWeight(int ageDays, int gracePeriodDays, int halfLifeDays, string function = "exponential")
    {
        // T-GRAPH-DECAY-001.1, T-GRAPH-DECAY-001.6: RTM FR-7.5, NFR-7.5.5
        if (string.Equals(function, "power-law", StringComparison.OrdinalIgnoreCase))
        {
            // For power-law, apply grace period manually then calculate decay
            if (ageDays <= gracePeriodDays)
            {
                return 1.0;
            }
            var effectiveAge = ageDays - gracePeriodDays;
            return CalculatePowerLawDecay(effectiveAge, DefaultPowerLawA, DefaultPowerLawB);
        }

        // Default: exponential decay
        return CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);
    }

    /// <summary>
    /// Gets the decay weight for a memory file based on its path and timestamps.
    /// Determines grace period from file path:
    /// - thinking/sequential/: 7 days (NFR-7.5.1)
    /// - thinking/workflows/: 14 days (NFR-7.5.1a)
    /// - all other memory: 14 days (NFR-7.5.2)
    /// </summary>
    /// <param name="created">Creation timestamp</param>
    /// <param name="lastAccessed">Last access timestamp (used if set, else created)</param>
    /// <param name="filePath">Path to the memory file</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double GetDecayWeight(DateTime created, DateTime? lastAccessed, string filePath)
    {
        // T-GRAPH-DECAY-001.1: RTM NFR-7.5.1/7.5.1a/7.5.2 - Tier-based grace periods
        var gracePeriodDays = GetGracePeriodForPath(filePath);

        // Use lastAccessed if available (access boosting), otherwise use created
        var referenceDate = lastAccessed ?? created;
        var ageDays = (int)(DateTime.UtcNow - referenceDate).TotalDays;

        // T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Use configured decay function
        return GetDecayWeight(ageDays, gracePeriodDays, DefaultHalfLifeDays, ConfiguredDecayFunction);
    }

    /// <summary>
    /// Gets the decay weight for a memory file (backward-compatible signature).
    /// </summary>
    /// <param name="filePath">Path to the memory file</param>
    /// <param name="created">Creation timestamp</param>
    /// <param name="lastAccessed">Last access timestamp (for access boosting)</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double GetMemoryDecayWeight(string filePath, DateTime created, DateTime? lastAccessed = null)
    {
        return GetDecayWeight(created, lastAccessed, filePath);
    }

    /// <summary>
    /// Gets the decay weight for an assumption based on its epistemic status.
    ///
    /// Status-specific decay rules (NFR-7.8.x):
    /// - validated: No decay (weight = 1.0 always) - exempt
    /// - active/refined: 14d grace period, 30d half-life
    /// - invalidated: 7d grace period, 14d half-life (aggressive decay)
    /// - unknown: Defaults to active decay parameters
    /// </summary>
    /// <param name="created">The creation timestamp of the assumption</param>
    /// <param name="status">The assumption status (validated, active, refined, invalidated)</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double GetAssumptionDecayWeight(DateTime created, string status)
    {
        // T-GRAPH-DECAY-004.1-3: RTM FR-7.8 - Status-based decay
        var normalizedStatus = (status ?? string.Empty).ToLowerInvariant().Trim();

        // T-GRAPH-DECAY-004.1: Validated assumptions are exempt from decay
        if (normalizedStatus == "validated")
        {
            return 1.0;
        }

        int gracePeriodDays;
        int halfLifeDays;

        // T-GRAPH-DECAY-004.2-3: Status-specific decay parameters
        switch (normalizedStatus)
        {
            case "invalidated":
                // Aggressive decay for invalidated assumptions
                gracePeriodDays = 7;
                halfLifeDays = 14;
                break;
            case "active":
            case "refined":
            default:
                // Standard decay for active/refined/unknown
                gracePeriodDays = 14;
                halfLifeDays = 30;
                break;
        }

        var ageDays = (int)(DateTime.UtcNow - created).TotalDays;
        return CalculateExponentialDecay(ageDays, gracePeriodDays, halfLifeDays);
    }

    /// <summary>
    /// Gets the decay weight for an assumption (backward-compatible signature).
    /// </summary>
    /// <param name="status">The assumption status (validated, active, refined, invalidated)</param>
    /// <param name="created">The creation timestamp of the assumption</param>
    /// <returns>Decay weight between 0.0 and 1.0</returns>
    public static double GetAssumptionDecayWeight(string status, DateTime created)
    {
        return GetAssumptionDecayWeight(created, status);
    }

    #endregion

    #region Helper Methods

    /// <summary>
    /// Determines the appropriate grace period based on file path.
    /// </summary>
    /// <param name="filePath">Path to the memory file</param>
    /// <returns>Grace period in days</returns>
    private static int GetGracePeriodForPath(string filePath)
    {
        if (string.IsNullOrEmpty(filePath))
        {
            return DefaultGracePeriodDays;
        }

        // Normalize path separators for consistent matching
        var normalizedPath = filePath.Replace('\\', '/').ToLowerInvariant();

        // T-GRAPH-DECAY-001.1: RTM NFR-7.5.1 - Sequential thinking uses 7-day grace
        if (normalizedPath.Contains("thinking/sequential") ||
            normalizedPath.Contains("/sequential/"))
        {
            return SequentialGracePeriodDays;
        }

        // T-GRAPH-DECAY-001.1: RTM NFR-7.5.1a - Workflows use 14-day grace
        if (normalizedPath.Contains("thinking/workflows") ||
            normalizedPath.Contains("/workflows/"))
        {
            return WorkflowsGracePeriodDays;
        }

        // T-GRAPH-DECAY-001.1: RTM NFR-7.5.2 - Default memory uses 14-day grace
        return DefaultGracePeriodDays;
    }

    #endregion
}
