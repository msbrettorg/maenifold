using System;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Deterministic performance-regression tests for CLI JSON output.
///
/// Goal (NFR-8.1.2): Prove JSON mode does not do extra work vs markdown without using wall-clock timing.
/// We measure:
/// - Allocation deltas (GC.GetAllocatedBytesForCurrentThread) for the same tool+dataset.
/// - Output size ratio (JSON vs markdown) on a fixed dataset.
/// - Shape assertions that catch "double work" patterns (e.g., markdown formatting embedded in JSON data).
/// </summary>
[NonParallelizable]
public class CliJsonPerformanceRegressionTests
{
    // T-CLI-JSON-001.6a: RTM NFR-8.1.2
    private const string TestFolder = "cli-json-perf-tests";
    private const string Query = "perf-regression-keyword";

    private string _testFolderPath = string.Empty;
    private static volatile int s_sink;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();

        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);
        Directory.CreateDirectory(_testFolderPath);

        // Fixed dataset: deterministic file names, deterministic content, fixed query term.
        // Keep dataset modest for CI stability.
        using (OutputContext.UseMarkdown())
        {
            for (int i = 0; i < 30; i++)
            {
                var title = $"Perf Regression Seed {i:00}";
                var body = $@"Seed file {i:00} for [[cli-json-output]] perf regression.

Contains the fixed query token: {Query}

Related concepts: [[search]] [[json]] [[markdown]] [[performance-testing]].";

                MemoryTools.WriteMemory(title, body, TestFolder);
            }
        }

        GraphTools.Sync();
    }

    [TearDown]
    public void TearDown()
    {
        OutputContext.Format = OutputFormat.Markdown;

        if (string.IsNullOrEmpty(_testFolderPath))
            return;

        var directory = new DirectoryInfo(_testFolderPath);
        if (directory.Exists)
        {
            foreach (var file in directory.EnumerateFiles("*", SearchOption.AllDirectories))
            {
                file.IsReadOnly = false;
                file.Delete();
            }
            directory.Delete(true);
        }
    }

    [Test]
    [Description("T-CLI-JSON-001.6a: Allocation delta JSON vs markdown (no wall-clock)")]
    public void SearchMemories_FullText_JsonMode_DoesNotAllocateSignificantlyMoreThanMarkdown()
    {
        // T-CLI-JSON-001.6a: RTM NFR-8.1.2

        static string RunMarkdown()
        {
            using (OutputContext.UseMarkdown())
            {
                return MemorySearchTools.SearchMemories(
                    query: Query,
                    mode: SearchMode.FullText,
                    pageSize: 10,
                    page: 1,
                    folder: TestFolder);
            }
        }

        static string RunJson()
        {
            using (OutputContext.UseJson())
            {
                return MemorySearchTools.SearchMemories(
                    query: Query,
                    mode: SearchMode.FullText,
                    pageSize: 10,
                    page: 1,
                    folder: TestFolder);
            }
        }

        // Warm up both paths to reduce JIT/caching noise.
        s_sink ^= RunMarkdown().Length;
        s_sink ^= RunJson().Length;

        // Measure allocations for the same work unit across multiple iterations.
        // This is deterministic within a single process and avoids wall-clock timing.
        const int iterations = 12;
        var markdownAllocated = MeasureAllocatedBytes(RunMarkdown, iterations);
        var jsonAllocated = MeasureAllocatedBytes(RunJson, iterations);

        // We allow a small slack for serialization overhead, but we are explicitly guarding
        // against "double work" regressions (e.g., building markdown and then serializing).
        var slack = Math.Max(4096L, markdownAllocated / 10);
        Assert.That(
            jsonAllocated,
            Is.LessThanOrEqualTo(markdownAllocated + slack),
            $"JSON mode allocated {jsonAllocated} bytes vs markdown {markdownAllocated} bytes (slack {slack}).");
    }

    [Test]
    [Description("T-CLI-JSON-001.6a: Output size ratio JSON vs markdown on fixed dataset")]
    public void SearchMemories_FullText_JsonMode_OutputSizeRatio_IsBounded()
    {
        // T-CLI-JSON-001.6a: RTM NFR-8.1.2

        string markdown;
        using (OutputContext.UseMarkdown())
        {
            markdown = MemorySearchTools.SearchMemories(
                query: Query,
                mode: SearchMode.FullText,
                pageSize: 10,
                page: 1,
                folder: TestFolder);
        }

        string json;
        using (OutputContext.UseJson())
        {
            json = MemorySearchTools.SearchMemories(
                query: Query,
                mode: SearchMode.FullText,
                pageSize: 10,
                page: 1,
                folder: TestFolder);
        }

        Assert.That(markdown.Length, Is.GreaterThan(0));
        Assert.That(json.Length, Is.GreaterThan(0));

        // Guard against the common "double work" regression where JSON embeds markdown-formatted output.
        Assert.That(json, Does.Not.Contain("Full-Text Search"));
        Assert.That(json, Does.Not.Contain("💡 Next:"));

        var ratio = (double)json.Length / markdown.Length;

        // Bounded ratio: JSON may be larger (field names), but it must not explode.
        // This is a deterministic fixed-dataset guardrail rather than a wall-clock test.
        Assert.That(ratio, Is.LessThan(2.75d), $"Unexpected JSON/markdown output size ratio: {ratio:F3}");
    }

    private static long MeasureAllocatedBytes(Func<string> action, int iterations)
    {
        var before = GC.GetAllocatedBytesForCurrentThread();

        for (int i = 0; i < iterations; i++)
        {
            // Consume the output to prevent dead-code elimination.
            s_sink ^= action().Length;
        }

        var after = GC.GetAllocatedBytesForCurrentThread();
        return after - before;
    }
}
