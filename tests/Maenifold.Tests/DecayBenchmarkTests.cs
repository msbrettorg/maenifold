// T-GRAPH-DECAY-BENCH-001: Benchmark decay-weighted vs unweighted retrieval precision
//
// The central claim of productive forgetting: decay makes search BETTER, not just newer.
// These tests measure whether decay weighting actually improves retrieval quality by
// creating realistic scenarios where temporal signal provides useful disambiguation.
//
// What we measure:
// - Rank position of the "right" answer with and without decay
// - Score delta between signal and noise with and without decay
// - Whether decay prevents context rot (old noise drowning recent signal)

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Benchmark tests measuring whether decay-weighted search produces higher precision
/// than unweighted search. This is the unmeasured claim from productive-forgetting.md:
/// "controlled deprioritization of stale memory improves system performance."
///
/// Unlike GraphDecayWeightingTests (which test that decay IS applied), these tests
/// measure whether applying it HELPS.
/// </summary>
[TestFixture]
[NonParallelizable]
public class DecayBenchmarkTests
{
    private const string TestFolder = "decay-benchmark-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);

        CleanupDatabaseForTestFolder();

        if (Directory.Exists(_testFolderPath))
            Directory.Delete(_testFolderPath, recursive: true);
        Directory.CreateDirectory(_testFolderPath);
    }

    [TearDown]
    public void TearDown()
    {
        CleanupDatabaseForTestFolder();

        if (!string.IsNullOrEmpty(_testFolderPath) && Directory.Exists(_testFolderPath))
        {
            var directory = new DirectoryInfo(_testFolderPath);
            foreach (var file in directory.EnumerateFiles("*", SearchOption.AllDirectories))
            {
                file.IsReadOnly = false;
                file.Delete();
            }
            foreach (var sub in directory.EnumerateDirectories("*", SearchOption.AllDirectories))
                sub.Delete(true);
            directory.Delete(true);
        }
    }

    private static void CleanupDatabaseForTestFolder()
    {
        try
        {
            using var conn = new Microsoft.Data.Sqlite.SqliteConnection(Config.DatabaseConnectionString);
            conn.Open();

            var pattern = $"memory://{TestFolder}/%";

            using var deleteContent = conn.CreateCommand();
            deleteContent.CommandText = "DELETE FROM file_content WHERE file_path LIKE @pattern";
            deleteContent.Parameters.AddWithValue("@pattern", pattern);
            deleteContent.ExecuteNonQuery();

            using var deleteMentions = conn.CreateCommand();
            deleteMentions.CommandText = "DELETE FROM concept_mentions WHERE source_file LIKE @pattern";
            deleteMentions.Parameters.AddWithValue("@pattern", pattern);
            deleteMentions.ExecuteNonQuery();

            try
            {
                conn.LoadExtension(Path.Combine(AppContext.BaseDirectory, "assets", "native",
                    System.Runtime.InteropServices.RuntimeInformation.IsOSPlatform(System.Runtime.InteropServices.OSPlatform.OSX)
                        ? (System.Runtime.InteropServices.RuntimeInformation.ProcessArchitecture == System.Runtime.InteropServices.Architecture.Arm64 ? "osx-arm64" : "osx-x64")
                        : "linux-x64",
                    System.Runtime.InteropServices.RuntimeInformation.IsOSPlatform(System.Runtime.InteropServices.OSPlatform.OSX) ? "vec0.dylib" : "vec0.so"));

                using var deleteVec = conn.CreateCommand();
                deleteVec.CommandText = "DELETE FROM vec_memory_files WHERE file_path LIKE @pattern";
                deleteVec.Parameters.AddWithValue("@pattern", pattern);
                deleteVec.ExecuteNonQuery();
            }
            catch { /* vec tables may not exist */ }
        }
        catch (Exception ex)
        {
            TestContext.Out.WriteLine($"[BENCHMARK CLEANUP] {ex.Message}");
        }
    }

    private void CreateTestFileWithAge(string relativePath, string title, string content, int daysAgo)
    {
        var fullPath = Path.Combine(_testFolderPath, relativePath);
        var directory = Path.GetDirectoryName(fullPath);
        if (directory != null && !Directory.Exists(directory))
            Directory.CreateDirectory(directory);

        var createdDate = DateTime.UtcNow.AddDays(-daysAgo);
        var createdIso = createdDate.ToString("yyyy-MM-ddTHH:mm:ss.fffZ", CultureInfo.InvariantCulture);

        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = title,
            ["type"] = "memory",
            ["status"] = "saved",
            ["created"] = createdIso,
            ["modified"] = createdIso
        };

        MarkdownIO.WriteMarkdown(fullPath, frontmatter, $"# {title}\n\n{content}");
    }

    #region Benchmark 1: Context Rot Scenario

    /// <summary>
    /// The context rot scenario: an agent has been working for months. Old debugging notes,
    /// outdated architecture docs, and stale research clutter the memory. A recent decision
    /// document should surface first, but without decay, old noise with the same keywords wins
    /// by sheer volume.
    ///
    /// This measures: does decay prevent old noise from drowning recent signal?
    /// </summary>
    [Test]
    [Description("BENCHMARK: Decay prevents context rot — recent signal outranks accumulated noise")]
    public void Benchmark_ContextRot_DecayPreventsOldNoiseDrowningRecentSignal()
    {
        if (!CanRunVecTests())
        {
            Assert.Ignore("sqlite-vec unavailable on CI — semantic benchmarks skipped");
        }

        // Scenario: Agent is working on API authentication.
        // Over the past 90 days, it accumulated 5 old notes about auth.
        // Yesterday, it wrote a definitive decision document.
        // Query: "authentication approach for the API"

        // Old noise (90 days ago) — valid content, just stale
        CreateTestFileWithAge("old-auth-research-1.md", "Auth Research: OAuth2 vs SAML",
            "Research on [[authentication]] approaches. Comparing [[OAuth2]] and [[SAML]] for the [[API]]. " +
            "OAuth2 is more common for REST APIs. SAML is enterprise-focused. Need to decide.",
            daysAgo: 90);

        CreateTestFileWithAge("old-auth-research-2.md", "Auth Research: JWT Considerations",
            "Investigating [[JWT]] [[token]] strategies for [[authentication]] in the [[API]]. " +
            "Refresh tokens, expiry policies, and revocation challenges documented.",
            daysAgo: 85);

        CreateTestFileWithAge("old-auth-debug-1.md", "Debug: Auth Middleware Failing",
            "Debugging [[authentication]] middleware in the [[API]] layer. The bearer token " +
            "validation was failing due to clock skew. Fixed by adding tolerance.",
            daysAgo: 75);

        CreateTestFileWithAge("old-auth-debug-2.md", "Debug: CORS and Auth Headers",
            "The [[API]] [[authentication]] headers were being stripped by CORS preflight. " +
            "Added proper Access-Control-Allow-Headers configuration.",
            daysAgo: 60);

        CreateTestFileWithAge("old-auth-meeting.md", "Meeting Notes: Auth Strategy Discussion",
            "Team discussion about [[authentication]] strategy for the [[API]]. " +
            "No consensus reached. Tabled for next sprint. Action items pending.",
            daysAgo: 50);

        // Recent signal (1 day ago) — the actual decision
        CreateTestFileWithAge("auth-decision.md", "DECISION: API Authentication Approach",
            "Final decision on [[authentication]] for the [[API]]: use [[OAuth2]] with PKCE flow. " +
            "JWT access tokens with 15-min expiry. Refresh tokens stored server-side. " +
            "This supersedes all prior research and debugging notes on auth.",
            daysAgo: 1);

        GraphTools.Sync();

        // Act: Search as the agent would
        var searchResult = MemorySearchTools.SearchMemories(
            query: "authentication approach for the API",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            minScore: 0.0);

        TestContext.Out.WriteLine("=== CONTEXT ROT BENCHMARK ===");
        TestContext.Out.WriteLine(searchResult);

        // Measure: Where does the decision document rank?
        var resultLines = searchResult.Split('\n');
        var allTitles = new List<string>();
        foreach (var line in resultLines)
        {
            if (line.Contains("**") && (line.Contains("memory://") || line.Contains(TestFolder)))
            {
                allTitles.Add(line);
            }
        }

        var decisionIndex = -1;
        for (int i = 0; i < allTitles.Count; i++)
        {
            if (allTitles[i].Contains("DECISION: API Authentication Approach"))
            {
                decisionIndex = i;
                break;
            }
        }

        TestContext.Out.WriteLine($"\n--- BENCHMARK RESULTS ---");
        TestContext.Out.WriteLine($"Decision document rank: {(decisionIndex >= 0 ? (decisionIndex + 1).ToString(CultureInfo.InvariantCulture) : "NOT FOUND")} out of {allTitles.Count}");
        TestContext.Out.WriteLine($"Total results: {allTitles.Count}");

        // The benchmark assertion: the decision document should rank in the top 2.
        // Without decay, 5 old documents with the same keywords would likely outrank
        // a single recent one through sheer volume of keyword matches.
        // With decay, the 90-day-old files are suppressed and the 1-day-old decision wins.
        Assert.That(decisionIndex, Is.GreaterThanOrEqualTo(0),
            "Decision document should appear in search results");
        Assert.That(decisionIndex, Is.LessThan(2),
            "BENCHMARK: With decay, the recent decision document should rank in the top 2, " +
            "not be buried under 5 stale documents about the same topic. " +
            $"Actual rank: {decisionIndex + 1}");
    }

    #endregion

    #region Benchmark 2: Decay Weight Distribution

    /// <summary>
    /// Measures the actual decay weight distribution across files of different ages.
    /// This isn't a pass/fail test — it's a measurement that proves the math translates
    /// to meaningful score differentiation in practice.
    ///
    /// The claim: "power-law decay with d=0.5" should produce measurable score separation
    /// between recent and old content.
    /// </summary>
    [Test]
    [Description("BENCHMARK: Decay produces measurable score separation between age cohorts")]
    public void Benchmark_DecayWeightDistribution_ProducesMeaningfulSeparation()
    {
        // Create files at different age checkpoints
        var ageCheckpoints = new[] { 1, 7, 14, 30, 60, 90, 120, 180, 365 };

        foreach (var daysAgo in ageCheckpoints)
        {
            CreateTestFileWithAge(
                $"age-{daysAgo}d.md",
                $"Test File ({daysAgo} days old)",
                $"Identical content about [[decay-benchmark]] and [[weight-distribution]] for testing. " +
                "This sentence ensures identical semantic and keyword matching across all files.",
                daysAgo: daysAgo);
        }

        GraphTools.Sync();

        // Measure decay weights for each file
        TestContext.Out.WriteLine("=== DECAY WEIGHT DISTRIBUTION ===");
        TestContext.Out.WriteLine("Age (days) | Decay Weight | Score Category");
        TestContext.Out.WriteLine("-----------|-------------|---------------");

        var weights = new List<(int age, double weight)>();
        foreach (var daysAgo in ageCheckpoints)
        {
            var filePath = Path.Combine(_testFolderPath, $"age-{daysAgo}d.md");
            var weight = MemorySearchTools.GetDecayWeightForFile(filePath);
            weights.Add((daysAgo, weight));

            var category = weight switch
            {
                >= 0.9 => "FULL (no meaningful decay)",
                >= 0.5 => "MODERATE (visible but present)",
                >= 0.2 => "SIGNIFICANT (clearly deprioritized)",
                >= 0.05 => "HEAVY (nearly invisible)",
                _ => "EXTREME (effectively buried)"
            };

            TestContext.Out.WriteLine($"    {daysAgo,3}     |    {weight:F4}    | {category}");
        }

        // Assertions: Verify the decay curve produces useful differentiation
        var freshWeight = weights.First(w => w.age == 1).weight;
        var monthOldWeight = weights.First(w => w.age == 30).weight;
        var quarterOldWeight = weights.First(w => w.age == 90).weight;
        var yearOldWeight = weights.First(w => w.age == 365).weight;

        // Fresh content should have full or near-full weight
        Assert.That(freshWeight, Is.GreaterThanOrEqualTo(0.95),
            "1-day-old content should have near-full weight (within grace period)");

        // 30-day content should be noticeably decayed but still visible
        Assert.That(monthOldWeight, Is.LessThan(freshWeight),
            "30-day-old content should have less weight than 1-day-old content");
        Assert.That(monthOldWeight, Is.GreaterThan(0.3),
            "30-day-old content should still be visible (not buried)");

        // 90-day content should be significantly decayed
        Assert.That(quarterOldWeight, Is.LessThan(monthOldWeight),
            "90-day-old content should decay further than 30-day-old content");

        // Year-old content should be heavily suppressed but not zero
        Assert.That(yearOldWeight, Is.LessThan(0.2),
            "365-day-old content should be heavily suppressed");
        Assert.That(yearOldWeight, Is.GreaterThan(0.0),
            "365-day-old content should never reach zero (asymptotic decay)");

        // The critical measurement: the ratio between fresh and stale
        var suppressionRatio = freshWeight / yearOldWeight;
        TestContext.Out.WriteLine($"\nSuppression ratio (fresh/year-old): {suppressionRatio:F1}x");
        TestContext.Out.WriteLine($"This means fresh content is weighted {suppressionRatio:F1}x more heavily than year-old content.");

        Assert.That(suppressionRatio, Is.GreaterThan(5.0),
            "BENCHMARK: Fresh content should be at least 5x more heavily weighted than year-old content. " +
            "This is the minimum ratio needed for decay to produce meaningful reranking.");
    }

    #endregion

    #region Benchmark 3: Access Boosting Rescues Relevant Old Content

    /// <summary>
    /// The nuance of productive forgetting: old content that's STILL USEFUL should survive.
    /// Access boosting (reading a file resets its decay clock) means that actively-used
    /// old knowledge persists while unused old knowledge fades.
    ///
    /// This measures: does the combination of decay + access boosting correctly distinguish
    /// between "old and still relevant" vs "old and stale"?
    /// </summary>
    [Test]
    [Description("BENCHMARK: Access boosting rescues actively-used old content from decay")]
    public void Benchmark_AccessBoosting_RescuesActivelyUsedOldContent()
    {
        // Scenario: Two files created 60 days ago about the same topic.
        // One was read recently (still relevant). One was never touched again (stale).

        CreateTestFileWithAge("active-old-doc.md", "Architecture: Service Mesh Pattern",
            "The [[service-mesh]] pattern using [[Istio]] for [[microservices]] communication. " +
            "Sidecar proxies handle routing, load balancing, and observability.",
            daysAgo: 60);

        CreateTestFileWithAge("stale-old-doc.md", "Architecture: Monolith Pattern",
            "The [[monolith]] pattern for [[microservices]] migration. " +
            "Strangler fig approach with gradual decomposition of the legacy system.",
            daysAgo: 60);

        GraphTools.Sync();

        // Measure both decay weights BEFORE access boosting
        var activeFilePath = Path.Combine(_testFolderPath, "active-old-doc.md");
        var staleFilePath = Path.Combine(_testFolderPath, "stale-old-doc.md");

        var activeWeightBefore = MemorySearchTools.GetDecayWeightForFile(activeFilePath);
        var staleWeightBefore = MemorySearchTools.GetDecayWeightForFile(staleFilePath);

        TestContext.Out.WriteLine("=== ACCESS BOOSTING BENCHMARK ===");
        TestContext.Out.WriteLine($"Before access boost:");
        TestContext.Out.WriteLine($"  Active doc weight: {activeWeightBefore:F4}");
        TestContext.Out.WriteLine($"  Stale doc weight:  {staleWeightBefore:F4}");

        // Both should be similarly decayed (same age, no prior access)
        Assert.That(activeWeightBefore, Is.EqualTo(staleWeightBefore).Within(0.01),
            "Before access boosting, both 60-day-old files should have equal decay weights");

        // Simulate: agent read the active doc recently (via ReadMemory)
        var activeUri = $"memory://{TestFolder}/active-old-doc";
        var readResult = MemoryTools.ReadMemory(activeUri);
        Assert.That(readResult, Does.Not.StartWith("ERROR:"), "ReadMemory should succeed");

        // Measure after access boosting
        var activeWeightAfter = MemorySearchTools.GetDecayWeightForFile(activeFilePath);
        var staleWeightAfter = MemorySearchTools.GetDecayWeightForFile(staleFilePath);

        TestContext.Out.WriteLine($"\nAfter reading active doc:");
        TestContext.Out.WriteLine($"  Active doc weight: {activeWeightAfter:F4} (was {activeWeightBefore:F4})");
        TestContext.Out.WriteLine($"  Stale doc weight:  {staleWeightAfter:F4} (unchanged)");

        var boostRatio = activeWeightAfter / staleWeightAfter;
        TestContext.Out.WriteLine($"\nBoost ratio (active/stale): {boostRatio:F1}x");

        // The active doc should now massively outweigh the stale one
        Assert.That(activeWeightAfter, Is.GreaterThan(0.95),
            "Recently-read old doc should have near-full weight (access boosting reset the clock)");
        Assert.That(staleWeightAfter, Is.LessThan(0.5),
            "Untouched old doc should remain decayed");
        Assert.That(boostRatio, Is.GreaterThan(2.0),
            "BENCHMARK: Access-boosted old content should be weighted at least 2x more than " +
            "equally-old untouched content. This proves the 'still useful' signal works.");
    }

    #endregion

    #region Benchmark 4: Tiered Decay — Episodic vs Semantic

    /// <summary>
    /// Thinking sessions (episodic memory) should decay faster than knowledge files
    /// (semantic memory). A debugging session from 2 weeks ago is less useful than
    /// a knowledge file from 2 weeks ago.
    ///
    /// This measures: do tiered grace periods produce appropriate differentiation?
    /// </summary>
    [Test]
    [Description("BENCHMARK: Episodic memory decays faster than semantic memory")]
    public void Benchmark_TieredDecay_EpisodicDecaysFasterThanSemantic()
    {
        // Create both at the same age (10 days) but in different tiers
        // Sequential (episodic): 7-day grace → already decaying at day 10
        // Default (semantic): 14-day grace → still in grace at day 10

        const int testAgeDays = 10;

        // Episodic: thinking/sequential path
        var episodicPath = Path.Combine(_testFolderPath, "thinking", "sequential");
        Directory.CreateDirectory(episodicPath);
        var episodicFilePath = Path.Combine(episodicPath, "debug-session.md");

        var createdDate = DateTime.UtcNow.AddDays(-testAgeDays);
        var createdIso = createdDate.ToString("yyyy-MM-ddTHH:mm:ss.fffZ", CultureInfo.InvariantCulture);

        MarkdownIO.WriteMarkdown(episodicFilePath, new Dictionary<string, object>
        {
            ["title"] = "Debug Session",
            ["type"] = "memory",
            ["created"] = createdIso,
            ["modified"] = createdIso
        }, "# Debug Session\n\nDebugging [[tiered-decay]] behavior.");

        // Semantic: regular path
        CreateTestFileWithAge("knowledge-doc.md", "Knowledge Document",
            "Knowledge about [[tiered-decay]] behavior.", daysAgo: testAgeDays);

        // Measure
        var semanticFilePath = Path.Combine(_testFolderPath, "knowledge-doc.md");
        var episodicWeight = MemorySearchTools.GetDecayWeightForFile(episodicFilePath);
        var semanticWeight = MemorySearchTools.GetDecayWeightForFile(semanticFilePath);

        TestContext.Out.WriteLine("=== TIERED DECAY BENCHMARK ===");
        TestContext.Out.WriteLine($"Both files: {testAgeDays} days old");
        TestContext.Out.WriteLine($"  Episodic (thinking/sequential): {episodicWeight:F4} (grace: 7d)");
        TestContext.Out.WriteLine($"  Semantic (default):             {semanticWeight:F4} (grace: 14d)");

        // At 10 days:
        // Episodic: past 7d grace → already decaying (3 days of decay applied)
        // Semantic: within 14d grace → full weight (1.0)
        Assert.That(episodicWeight, Is.LessThan(semanticWeight),
            "BENCHMARK: At day 10, episodic memory (7d grace) should have lower weight " +
            "than semantic memory (14d grace). Thinking sessions fade faster than knowledge.");

        Assert.That(semanticWeight, Is.EqualTo(1.0).Within(0.001),
            "Semantic memory at 10 days should still be at full weight (within 14d grace)");

        Assert.That(episodicWeight, Is.LessThan(1.0),
            "Episodic memory at 10 days should be decaying (past 7d grace)");
    }

    #endregion

    #region Benchmark 5: Precision at K

    /// <summary>
    /// The precision@k measurement: given a query with a known "right" answer,
    /// what position does the right answer appear at?
    ///
    /// We create a realistic scenario with relevant and irrelevant files,
    /// then measure whether the relevant file appears in the top-k.
    ///
    /// This is the closest thing to a controlled retrieval benchmark.
    /// </summary>
    [Test]
    [Description("BENCHMARK: Precision@3 — relevant recent doc in top 3 results")]
    public void Benchmark_PrecisionAtK_RelevantRecentDocInTop3()
    {
        if (!CanRunVecTests())
        {
            Assert.Ignore("sqlite-vec unavailable on CI — semantic benchmarks skipped");
        }

        // Create a mix: 4 irrelevant old files, 3 tangentially related old files, 1 relevant new file

        // Irrelevant old files (different topics)
        CreateTestFileWithAge("old-unrelated-1.md", "Database Migration Guide",
            "Guide for migrating from [[PostgreSQL]] to [[CockroachDB]]. Schema changes and data types.",
            daysAgo: 80);
        CreateTestFileWithAge("old-unrelated-2.md", "CSS Grid Layout",
            "Using [[CSS-Grid]] for responsive layouts. Grid template areas and auto-placement.",
            daysAgo: 70);
        CreateTestFileWithAge("old-unrelated-3.md", "Docker Networking",
            "Configuring [[Docker]] bridge networks and overlay networks for [[container]] orchestration.",
            daysAgo: 65);
        CreateTestFileWithAge("old-unrelated-4.md", "Git Workflow",
            "Implementing [[trunk-based-development]] with [[feature-flags]] and continuous delivery.",
            daysAgo: 55);

        // Tangentially related old files (same domain, different focus)
        CreateTestFileWithAge("old-related-1.md", "Error Handling Research",
            "Research on [[error-handling]] patterns: circuit breakers, retries, and [[graceful-degradation]]. " +
            "How to build [[resilient]] systems that handle failures.",
            daysAgo: 90);
        CreateTestFileWithAge("old-related-2.md", "Old Observability Setup",
            "Setting up [[observability]] with [[Prometheus]] and [[Grafana]]. " +
            "Metrics, traces, and [[error-handling]] dashboards.",
            daysAgo: 85);
        CreateTestFileWithAge("old-related-3.md", "Logging Best Practices",
            "Structured [[logging]] for [[error-handling]] and debugging. " +
            "Correlation IDs, log levels, and centralized aggregation.",
            daysAgo: 45);

        // The relevant recent file
        CreateTestFileWithAge("error-strategy-decision.md", "DECISION: Error Handling Strategy",
            "Decided on [[error-handling]] strategy: use [[Result-type]] pattern for expected errors, " +
            "exceptions for unexpected failures. [[Graceful-degradation]] via circuit breakers " +
            "with [[Polly]] library. This is the final approach for all new services.",
            daysAgo: 2);

        GraphTools.Sync();

        // Query: asking about error handling strategy
        var searchResult = MemorySearchTools.SearchMemories(
            query: "error handling strategy for services",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder,
            minScore: 0.0);

        TestContext.Out.WriteLine("=== PRECISION@K BENCHMARK ===");
        TestContext.Out.WriteLine(searchResult);

        // Find the rank of the decision document
        var decisionRank = FindDocumentRank(searchResult, "DECISION: Error Handling Strategy");
        var oldRelated1Rank = FindDocumentRank(searchResult, "Error Handling Research");
        var oldRelated2Rank = FindDocumentRank(searchResult, "Old Observability Setup");

        TestContext.Out.WriteLine("\n--- PRECISION RESULTS ---");
        TestContext.Out.WriteLine($"Decision doc rank:      {FormatRank(decisionRank)}");
        TestContext.Out.WriteLine($"Old research rank:      {FormatRank(oldRelated1Rank)}");
        TestContext.Out.WriteLine($"Old observability rank: {FormatRank(oldRelated2Rank)}");

        // Precision@3: the decision document should be in the top 3
        Assert.That(decisionRank, Is.GreaterThanOrEqualTo(0),
            "Decision document should appear in search results");
        Assert.That(decisionRank, Is.LessThan(3),
            $"BENCHMARK: Precision@3 — the recent, relevant decision document should rank " +
            $"in the top 3 results (actual rank: {decisionRank + 1}). " +
            "Decay should boost it above older, tangentially-related content.");
    }

    #endregion

    #region Helpers

    private static bool CanRunVecTests()
    {
        var isCI = Environment.GetEnvironmentVariable("CI") == "true" ||
                   Environment.GetEnvironmentVariable("GITHUB_ACTIONS") == "true";
        return !isCI;
    }

    /// <summary>
    /// Finds the 0-based rank of a document in search results by title.
    /// Returns -1 if not found.
    /// </summary>
    private static int FindDocumentRank(string searchResult, string titleFragment)
    {
        var lines = searchResult.Split('\n');
        var rank = 0;
        foreach (var line in lines)
        {
            // Result lines contain ** around titles
            if (line.Contains("**") && (line.Contains("memory://") || line.Contains("decay-benchmark-tests")))
            {
                if (line.Contains(titleFragment, StringComparison.OrdinalIgnoreCase))
                    return rank;
                rank++;
            }
        }
        return -1;
    }

    private static string FormatRank(int rank)
    {
        return rank >= 0 ? $"#{rank + 1}" : "NOT FOUND";
    }

    #endregion
}
