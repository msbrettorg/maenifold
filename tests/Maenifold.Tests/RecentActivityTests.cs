using System.Diagnostics;
using System.Text;
using System.Globalization;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// End-to-end tests for RecentActivity tool using the compiled CLI binary.
/// - Uses environment isolation via MAENIFOLD_ROOT per test run
/// - Seeds markdown files in memory/ and runs Sync to populate SQLite
/// - Invokes RecentActivity with varied payloads and validates output
/// </summary>
[NonParallelizable] // Database operations need sequential execution in CI
public class RecentActivityTests
{
    private string _root = string.Empty;           // MAENIFOLD_ROOT
    private string _memoryPath = string.Empty;     // {_root}/memory
    private string _dbPath = string.Empty;         // {_root}/memory.db
    private string _cliPath = string.Empty;        // path to Maenifold CLI in test bin directory

    [SetUp]
    public void SetUp()
    {
        // Unique test root under test-outputs
        var baseOutputs = Path.Combine(TestContext.CurrentContext.WorkDirectory, "recent-activity-tests");
        Directory.CreateDirectory(baseOutputs);
        _root = Path.Combine(baseOutputs, Guid.NewGuid().ToString("N"));
        _memoryPath = Path.Combine(_root, "memory");
        Directory.CreateDirectory(_memoryPath);
        _dbPath = Path.Combine(_root, "memory.db");

        // Determine CLI location (copied next to test binaries by build)
        // Linux/macOS: no extension; Windows: .exe
        var baseDir = AppContext.BaseDirectory;
        var exe = Path.Combine(baseDir, "Maenifold");
        var exeWin = exe + ".exe";
        _cliPath = File.Exists(exe) ? exe : exeWin;
        Assert.That(File.Exists(_cliPath), Is.True, $"Maenifold CLI not found at {_cliPath}. Build the solution to generate it.");
    }

    [TearDown]
    public void TearDown()
    {
        // Keep artifacts under test-outputs for debugging per Ma Protocol
        // No cleanup
    }

    [Test, Ignore("Environment isolation issue - database from other tests persists despite MAENIFOLD_ROOT")]
    public void RecentActivityNoDatabaseShowsGuidance()
    {
        // Ensure database doesn't exist for this test
        if (File.Exists(_dbPath))
            File.Delete(_dbPath);

        var output = RunCli("RecentActivity", "{\"limit\":5}");
        Assert.That(output, Does.Contain("No database found. Run the `Sync` command first to index memory files"));
    }

    [Test, Ignore("Test involves multiple sync operations on 1M+ relations database")]
    public void RecentActivityFiltersTimespanIncludeContentWorkflowAndSequential()
    {
        // Seed memory files
        var memArticleTitle = "Article Alpha";
        var memArticle = "# Article Alpha\n\n## Intro\nMemory body A\n\n## More\nMemory body B\n";
        WriteFile(Path.Combine(_memoryPath, "article-alpha.md"), memArticle);

        var seqDir = Path.Combine(_memoryPath, "thinking", "sequential", DateTime.UtcNow.Year.ToString(CultureInfo.InvariantCulture), DateTime.UtcNow.ToString("MM", CultureInfo.InvariantCulture));
        Directory.CreateDirectory(seqDir);
        var seqContent = "# Sequential Session\n\n## Thought 1/2\nFirst thought\n\n## Thought 2/2\nSecond thought\n";
        var seqPath = Path.Combine(seqDir, "session-1.md");
        WriteFile(seqPath, seqContent);

        var wfDir = Path.Combine(_memoryPath, "thinking", "workflow", DateTime.UtcNow.Year.ToString(CultureInfo.InvariantCulture), DateTime.UtcNow.ToString("MM", CultureInfo.InvariantCulture));
        Directory.CreateDirectory(wfDir);
        var wfContent = "# Workflow Run\n\n## Step 1\nStep one response\n\n## Step 2\nStep two response\n";
        var wfPath = Path.Combine(wfDir, "workflow-1.md");
        WriteFile(wfPath, wfContent);

        var chatDir = Path.Combine(_memoryPath, "chat", DateTime.UtcNow.Year.ToString(CultureInfo.InvariantCulture), DateTime.UtcNow.ToString("MM", CultureInfo.InvariantCulture));
        Directory.CreateDirectory(chatDir);
        var chatContent = "# Chat Log\n\n## Message\nHello from chat\n";
        var chatPath = Path.Combine(chatDir, "chat-1.md");
        WriteFile(chatPath, chatContent);

        // Run Sync to populate DB
        var syncOutput = RunCli("Sync", "{}");
        Assert.That(syncOutput, Does.Contain("Sync"), "Sync should run successfully");
        Assert.That(File.Exists(_dbPath), Is.True, "Database should be created by Sync");

        // Make article appear old to test timespan cutoff
        using (var conn = new SqliteConnection($"Data Source={_dbPath}"))
        {
            conn.Open();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "UPDATE file_content SET last_indexed=@ts WHERE title=@title";
            cmd.Parameters.AddWithValue("@ts", "2020-01-01 00:00:00");
            cmd.Parameters.AddWithValue("@title", memArticleTitle);
            cmd.ExecuteNonQuery();
        }

        // 1) all (default)
        var allOut = RunCli("RecentActivity", "{\"limit\":10}");
        Assert.That(allOut, Does.Contain("# Recent Activity"));
        Assert.That(allOut, Does.Contain("(sequential)"));
        Assert.That(allOut, Does.Contain("(workflow)"));
        Assert.That(allOut, Does.Contain("(memory)"));

        // 2) filter=thinking → sequential/workflow only
        var thinkOut = RunCli("RecentActivity", "{\"limit\":10,\"filter\":\"thinking\"}");
        Assert.That(thinkOut, Does.Contain("(sequential)"));
        Assert.That(thinkOut, Does.Contain("(workflow)"));
        Assert.That(thinkOut, Does.Not.Contain("(memory)"));

        // 3) filter=memory → exclude thinking and chat
        var memOut = RunCli("RecentActivity", "{\"limit\":10,\"filter\":\"memory\"}");
        Assert.That(memOut, Does.Contain("(memory)"));
        Assert.That(memOut, Does.Not.Contain("(sequential)"));
        Assert.That(memOut, Does.Not.Contain("(workflow)"));

        // 4) filter=assumptions (supported) → assumption/chat entries only (printed as memory type)
        var chatOut = RunCli("RecentActivity", "{\"limit\":10,\"filter\":\"assumptions\"}");
        Assert.That(chatOut, Does.Contain("(memory)"), "Assumption entries render as memory type");
        Assert.That(chatOut, Does.Contain("Title: chat-1"));
        Assert.That(chatOut, Does.Not.Contain("Sequential Session"));
        Assert.That(chatOut, Does.Not.Contain("Workflow Run"));

        // 5) includeContent=true → show explicit H2 headings and content
        var includeOut = RunCli("RecentActivity", "{\"limit\":10,\"includeContent\":true}");
        Assert.That(includeOut, Does.Contain("First H2:"));
        Assert.That(includeOut, Does.Contain("First H2 Content:"));
        Assert.That(includeOut, Does.Contain("Thought 1/2"));
        Assert.That(includeOut, Does.Contain("Step 1"));

        // 6) timespan filter should exclude the old article (set to 2020)
        var recentOnly = RunCli("RecentActivity", "{\"limit\":10,\"timespan\":\"1.00:00:00\"}");
        Assert.That(recentOnly, Does.Not.Contain("Article Alpha"));
        Assert.That(recentOnly, Does.Contain("(sequential)"));
        Assert.That(recentOnly, Does.Contain("(workflow)"));

        // 7) negative timespan → error
        var bad = RunCli("RecentActivity", "{\"timespan\":\"-1.00:00:00\"}");
        Assert.That(bad, Does.Contain("ERROR: timespan parameter must be positive"));

        // 8) limit=1 should only show a single entry block (count headings with **session**)
        var one = RunCli("RecentActivity", "{\"limit\":1}");
        var count = one.Split('\n').Count(l => l.StartsWith("**", StringComparison.Ordinal));
        Assert.That(count, Is.EqualTo(1), "Limit=1 should return a single activity item");
    }

    [Test]
    [Ignore("Disabled due to SQLite permission errors in CI - needs investigation")]
    public void RecentActivityShowsConclusionInLastField()
    {
        // Regression test for RTM-014: Ensure Conclusion sections appear in Last field
        var year = DateTime.UtcNow.Year.ToString(CultureInfo.InvariantCulture);
        var month = DateTime.UtcNow.ToString("MM", CultureInfo.InvariantCulture);
        var day = DateTime.UtcNow.ToString("dd", CultureInfo.InvariantCulture);
        var seqDir = Path.Combine(_memoryPath, "thinking", "sequential", year, month, day);
        Directory.CreateDirectory(seqDir);

        // Create a session with 1 thought + Conclusion (mimics session-1759338074329 structure)
        var sessionContent = @"---
title: Test Session with Conclusion
type: sequential
status: completed
thoughtCount: 1
---

# Sequential Thinking Session

## Thought 1/1 [test]

This is the first thought content.

## Conclusion

This is the conclusion content that should appear in Last field.
";
        var sessionPath = Path.Combine(seqDir, "test-session-conclusion.md");
        WriteFile(sessionPath, sessionContent);

        // Run Sync to populate database
        var syncOutput = RunCli("Sync", "{}");
        Assert.That(syncOutput, Does.Contain("Sync"), "Sync should complete successfully");

        // Run RecentActivity
        var output = RunCli("RecentActivity", "{\"limit\":10,\"filter\":\"thinking\"}");

        // Verify output contains the session (session ID will be extracted from path, e.g., "2025")
        Assert.That(output, Does.Contain("(sequential)"), "Session should be typed as sequential");
        Assert.That(output, Does.Contain("Thoughts: 1"), "Should show 1 thought");
        Assert.That(output, Does.Contain("Status: completed"), "Should show completed status");

        // CRITICAL: Verify First field contains thought content
        Assert.That(output, Does.Contain("First: \"This is the first thought content.\""),
            "First field should contain thought content");

        // CRITICAL: Verify Last field contains Conclusion content
        Assert.That(output, Does.Contain("Last: \"This is the conclusion content that should appear in Last field.\""),
            "Last field should contain Conclusion content - this is the regression test for RTM-014");

        // Ensure old metadata sections are NOT in the output
        Assert.That(output, Does.Not.Contain("Completion"), "Old Completion sections should be filtered out");
        Assert.That(output, Does.Not.Contain("Cancellation"), "Old Cancellation sections should be filtered out");
    }

    [Test, Explicit("Perf measurement - run manually")]
    public void RecentActivityPerformanceComparedToFileScan()
    {
        // Generate a moderate dataset (e.g., 300 files)
        var year = DateTime.UtcNow.Year.ToString(CultureInfo.InvariantCulture);
        var month = DateTime.UtcNow.ToString("MM", CultureInfo.InvariantCulture);
        var seqDir = Path.Combine(_memoryPath, "thinking", "sequential", year, month);
        var memDir = Path.Combine(_memoryPath, "docs");
        Directory.CreateDirectory(seqDir);
        Directory.CreateDirectory(memDir);

        for (int i = 0; i < 150; i++)
        {
            WriteFile(Path.Combine(seqDir, $"session-{i.ToString("D3", CultureInfo.InvariantCulture)}.md"), $"# S{i.ToString(CultureInfo.InvariantCulture)}\n\n## Thought 1/2\nHello {i.ToString(CultureInfo.InvariantCulture)}\n\n## Thought 2/2\nWorld {i.ToString(CultureInfo.InvariantCulture)}\n");
            WriteFile(Path.Combine(memDir, $"article-{i.ToString("D3", CultureInfo.InvariantCulture)}.md"), $"# A{i.ToString(CultureInfo.InvariantCulture)}\n\n## H2-1\nAlpha {i.ToString(CultureInfo.InvariantCulture)}\n\n## H2-2\nBeta {i.ToString(CultureInfo.InvariantCulture)}\n");
        }

        RunCli("Sync", "{}");

        var swDb = Stopwatch.StartNew();
        RunCli("RecentActivity", "{\"limit\":200}");
        swDb.Stop();

        var swScan = Stopwatch.StartNew();
        var scanCount = ScanFilesForH2(_memoryPath).Count;
        swScan.Stop();

        Assert.Pass($"Perf measured - DB RecentActivity: {swDb.ElapsedMilliseconds} ms, File scan (count={scanCount}): {swScan.ElapsedMilliseconds} ms");
    }

    // Helpers

    private string RunCli(string tool, string payloadJson)
    {
        var psi = new ProcessStartInfo
        {
            FileName = _cliPath,
            Arguments = $"--tool {tool} --payload \"{payloadJson.Replace("\"", "\\\"")}\"",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false
        };
        psi.Environment["MAENIFOLD_ROOT"] = _root;
        var proc = Process.Start(psi)!;
        var stdout = new StringBuilder();
        proc.OutputDataReceived += (_, e) => { if (e.Data != null) stdout.AppendLine(e.Data); };
        proc.BeginOutputReadLine();
        var stderr = proc.StandardError.ReadToEnd();
        proc.WaitForExit(60000);
        if (proc.ExitCode != 0)
        {
            Assert.Fail($"CLI exited with code {proc.ExitCode}. STDERR: {stderr}");
        }
        return stdout.ToString();
    }

    private static void WriteFile(string path, string content)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, content);
    }

    private static List<(string file, List<(string h2, string content)> sections)> ScanFilesForH2(string root)
    {
        var list = new List<(string, List<(string, string)>)>();
        foreach (var file in Directory.EnumerateFiles(root, "*.md", SearchOption.AllDirectories))
        {
            var text = File.ReadAllText(file);
            var sections = new List<(string, string)>();
            var lines = text.Split('\n');
            string currentH2 = string.Empty;
            var buf = new StringBuilder();
            foreach (var line in lines)
            {
                if (line.StartsWith("## ", StringComparison.Ordinal))
                {
                    if (!string.IsNullOrEmpty(currentH2))
                    {
                        sections.Add((currentH2, buf.ToString().Trim()));
                        buf.Clear();
                    }
                    currentH2 = line.Substring(3).Trim();
                }
                else if (!line.StartsWith("# ", StringComparison.Ordinal))
                {
                    buf.AppendLine(line);
                }
            }
            if (!string.IsNullOrEmpty(currentH2))
            {
                sections.Add((currentH2, buf.ToString().Trim()));
            }
            list.Add((file, sections));
        }
        return list;
    }
}
