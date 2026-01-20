using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.Text;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using ModelContextProtocol.Server;

namespace Maenifold.Tools;

[McpServerToolType]
public static partial class PerformanceBenchmark
{
    [McpServerTool, Description("Benchmarks all maenifold performance claims including GRPH-009 CTE vs N+1, search performance, sync timing, and complex traversal bottlenecks")]
    public static string RunFullBenchmark(
            [Description("Number of test iterations per benchmark")] int iterations = 5,
            [Description("Maximum test files to use for benchmarks")] int maxTestFiles = 1000,
            [Description("Include expensive deep traversal tests")] bool includeDeepTraversal = true,
            [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(RunFullBenchmark));

        // ISSUE-006: Validate iterations parameter
        if (iterations <= 0)
        {
            throw new ArgumentException("iterations must be > 0", nameof(iterations));
        }

        var results = new StringBuilder();
        results.AppendLine("MA CORE PERFORMANCE BENCHMARK SUITE");
        results.AppendLine("=====================================");
        results.AppendLineInvariant($"Iterations: {iterations}, Max Test Files: {maxTestFiles}");
        results.AppendLineInvariant($"Started: {DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture)}");
        results.AppendLine();

        try
        {

            GraphDatabase.InitializeDatabase();


            results.AppendLine(BenchmarkGraphTraversal(iterations));
            results.AppendLine();


            results.AppendLine(BenchmarkSearchPerformance(iterations, maxTestFiles));
            results.AppendLine();


            results.AppendLine(BenchmarkSyncPerformance(iterations, maxTestFiles));
            results.AppendLine();


            if (includeDeepTraversal)
            {
                results.AppendLine(BenchmarkComplexTraversal(iterations));
                results.AppendLine();
            }


            results.AppendLine(GenerateSystemHealthReport());

        }
        catch (Exception ex)
        {
            results.AppendLine("BENCHMARK FAILED:");
            results.AppendLine(ex.Message);
        }

        results.AppendLineInvariant($"Completed: {DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture)}");
        return results.ToString();
    }

    private static string BenchmarkSearchPerformance(int iterations, int maxTestFiles)
    {
        var results = new StringBuilder();
        results.AppendLine("Search Performance Benchmark");
        results.AppendLine("Claims: Hybrid 33ms, Semantic 116ms, Full-text 47ms");
        results.AppendLine();


        var fileCount = Directory.GetFiles(Config.MemoryPath, "*.md", SearchOption.AllDirectories).Length;
        results.AppendLineInvariant($"Test dataset: {fileCount} files");

        if (fileCount < 100)
        {
            return $"Insufficient test data ({fileCount} files). Need at least 100 files for meaningful benchmarks.";
        }

        var testQueries = new[]
                {
            "machine learning algorithms",
            "performance optimization",
            "graph database",
            "vector embeddings",
            "concept relationships"
        };

        var hybridTimings = new List<long>();
        var semanticTimings = new List<long>();
        var fulltextTimings = new List<long>();

        foreach (var query in testQueries)
        {
            for (int i = 0; i < iterations; i++)
            {

                var timer = Stopwatch.StartNew();
                var hybridResult = MemorySearchTools.SearchMemories(query, SearchMode.Hybrid, pageSize: 10);
                timer.Stop();
                hybridTimings.Add(timer.ElapsedMilliseconds);


                timer.Restart();
                var semanticResult = MemorySearchTools.SearchMemories(query, SearchMode.Semantic, pageSize: 10);
                timer.Stop();
                semanticTimings.Add(timer.ElapsedMilliseconds);


                timer.Restart();
                var fulltextResult = MemorySearchTools.SearchMemories(query, SearchMode.FullText, pageSize: 10);
                timer.Stop();
                fulltextTimings.Add(timer.ElapsedMilliseconds);
            }
        }

        var hybridAvg = hybridTimings.Average();
        var semanticAvg = semanticTimings.Average();
        var fulltextAvg = fulltextTimings.Average();

        results.AppendLineInvariant($"Results ({iterations * testQueries.Length} iterations each):");
        results.AppendLineInvariant($"Hybrid Average: {hybridAvg:F1}ms (claim: 33ms)");
        results.AppendLineInvariant($"Semantic Average: {semanticAvg:F1}ms (claim: 116ms)");
        results.AppendLineInvariant($"Full-text Average: {fulltextAvg:F1}ms (claim: 47ms)");

        return results.ToString();
    }

    private static string BenchmarkSyncPerformance(int iterations, int maxTestFiles)
    {
        var results = new StringBuilder();
        results.AppendLine("Sync Performance Benchmark");
        results.AppendLine("Claim: 27s for 2,500 files");
        results.AppendLine();

        var fileCount = Directory.GetFiles(Config.MemoryPath, "*.md", SearchOption.AllDirectories).Length;
        results.AppendLineInvariant($"Test dataset: {fileCount} files");

        var syncTimings = new List<long>();

        for (int i = 0; i < Math.Min(iterations, 3); i++)
        {
            var timer = Stopwatch.StartNew();
            var syncResult = ConceptSync.Sync();
            timer.Stop();
            syncTimings.Add(timer.ElapsedMilliseconds);

            results.AppendLineInvariant($"Sync iteration {i + 1}: {timer.ElapsedMilliseconds}ms");
        }

        var avgSyncTime = syncTimings.Average();
        var projectedTimeFor2500 = (avgSyncTime / fileCount) * 2500;

        results.AppendLine();
        results.AppendLine("Results:");
        results.AppendLineInvariant($"Average sync time: {avgSyncTime:F0}ms for {fileCount} files");
        results.AppendLineInvariant($"Projected time for 2,500 files: {projectedTimeFor2500:F0}ms ({projectedTimeFor2500 / 1000:F1}s)");
        results.AppendLineInvariant($"Claim verification: {(projectedTimeFor2500 / 1000 <= 35 ? "REASONABLE" : "SLOWER THAN CLAIMED")} (target: 27s)");

        return results.ToString();
    }

    private static string GenerateSystemHealthReport()
    {
        var results = new StringBuilder();
        results.AppendLine("System Health Report");
        results.AppendLine();

        try
        {
            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenWithWAL();

            var conceptCount = conn.QuerySingle<int>("SELECT COUNT(*) FROM concepts");
            var relationCount = conn.QuerySingle<int>("SELECT COUNT(*) FROM concept_graph");
            var fileCount = conn.QuerySingle<int>("SELECT COUNT(*) FROM file_content");

            results.AppendLine("Database health:");
            results.AppendLineInvariant($"  Concepts: {conceptCount:N0}");
            results.AppendLineInvariant($"  Relations: {relationCount:N0}");
            results.AppendLineInvariant($"  Files indexed: {fileCount:N0}");


            try
            {
                conn.LoadVectorExtension();
                var vecConcepts = conn.QuerySingle<int>("SELECT COUNT(*) FROM vec_concepts WHERE embedding IS NOT NULL");
                var vecFiles = conn.QuerySingle<int>("SELECT COUNT(*) FROM vec_memory_files WHERE embedding IS NOT NULL");
                results.AppendLineInvariant($"  Vector embeddings: {vecConcepts:N0} concepts, {vecFiles:N0} files");
            }
            catch
            {
                results.AppendLine("  Vector embeddings: sqlite-vec unavailable");
            }

            var dbSizeMB = new FileInfo(Config.DatabasePath).Length / (1024.0 * 1024.0);
            results.AppendLineInvariant($"  Database size: {dbSizeMB:F1} MB");
        }
        catch (Exception ex)
        {
            results.AppendLineInvariant($"Health check failed: {ex.Message}");
        }

        return results.ToString();
    }
}
