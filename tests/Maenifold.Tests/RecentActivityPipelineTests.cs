// T-COV-001.1: RTM FR-17.3
// Integration tests for the RecentActivity pipeline:
//   RecentActivityTools, RecentActivityReader, RecentActivityFormatter
//
// Tests use real SQLite and real filesystem per testing philosophy (CLAUDE.md).
// No mocks, no stubs. Files are seeded, GraphTools.Sync() populates the DB,
// then pipeline methods are exercised and assertions verify real behavior.

using System.Globalization;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

[TestFixture]
[NonParallelizable]
[Category("T-COV-001.1")]
public class RecentActivityPipelineTests
{
    private string _testRoot = string.Empty;
    private string _previousMaenifoldRootEnv = string.Empty;
    private string _previousDatabasePathEnv = string.Empty;

    [SetUp]
    public void SetUp()
    {
        _previousMaenifoldRootEnv = Environment.GetEnvironmentVariable("MAENIFOLD_ROOT") ?? string.Empty;
        _previousDatabasePathEnv = Environment.GetEnvironmentVariable("MAENIFOLD_DATABASE_PATH") ?? string.Empty;

        var repoRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", ".."));
        _testRoot = Path.Combine(repoRoot, "test-outputs", "recent-activity-pipeline", $"run-{Guid.NewGuid():N}");
        Directory.CreateDirectory(_testRoot);

        var testDbPath = Path.Combine(_testRoot, "memory.db");
        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", _testRoot);
        Environment.SetEnvironmentVariable("MAENIFOLD_DATABASE_PATH", testDbPath);

        Config.OverrideRoot(_testRoot);
        Config.SetDatabasePath(testDbPath);
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
    }

    [TearDown]
    public void TearDown()
    {
        try
        {
            if (!string.IsNullOrWhiteSpace(_testRoot) && Directory.Exists(_testRoot))
            {
                var dir = new DirectoryInfo(_testRoot);
                foreach (var f in dir.GetFiles("*", SearchOption.AllDirectories))
                {
                    f.Attributes = FileAttributes.Normal;
                }
                Directory.Delete(_testRoot, recursive: true);
            }
        }
        catch
        {
            // Ignore cleanup failures; artifacts remain under test-outputs for debugging.
        }

        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", string.IsNullOrEmpty(_previousMaenifoldRootEnv) ? null : _previousMaenifoldRootEnv);
        Environment.SetEnvironmentVariable("MAENIFOLD_DATABASE_PATH", string.IsNullOrEmpty(_previousDatabasePathEnv) ? null : _previousDatabasePathEnv);

        if (string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv) && string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
        {
            Config.ResetOverrides();
        }
        else
        {
            if (!string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv))
                Config.OverrideRoot(_previousMaenifoldRootEnv);

            if (!string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
                Config.SetDatabasePath(_previousDatabasePathEnv);
        }

        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
    }

    // ============================================================
    // RecentActivityReader.CollectRecentActivity
    // ============================================================

    [Test]
    public void CollectRecentActivity_ReturnsAllFiles_WhenFilterIsNull()
    {
        // T-COV-001.1: RTM FR-17.3
        // Seed a sequential, a memory file, then sync. Query with no filter.
        SeedSequentialFile("session-all-test", "## Thought 1/3\nThis is thought one.");
        SeedMemoryFile("notes/note-all-test.md", "# Note All Test\n\n## Section\nSome content.");

        GraphTools.Sync();

        var results = RecentActivityReader.CollectRecentActivity(null, null, 50);

        Assert.That(results.Count, Is.GreaterThanOrEqualTo(2),
            "CollectRecentActivity with no filter should return all indexed files.");

        var paths = results.Select(r => r.filePath).ToList();
        Assert.That(paths, Has.Some.Contain("session-all-test"),
            "Sequential file should appear in unfiltered results.");
        Assert.That(paths, Has.Some.Contain("note-all-test"),
            "Memory file should appear in unfiltered results.");
    }

    [Test]
    public void CollectRecentActivity_FilterThinking_ReturnsOnlyThinkingFiles()
    {
        // T-COV-001.1: RTM FR-17.3
        // filter=thinking should include only files under the thinking/ path.
        SeedSequentialFile("session-thinking-filter", "## Thought 1/2\nFirst thought.");
        SeedMemoryFile("notes/note-thinking-filter.md", "# Note\n\n## Section\nNot thinking.");

        GraphTools.Sync();

        var results = RecentActivityReader.CollectRecentActivity(null, "thinking", 50);

        Assert.That(results.Count, Is.GreaterThanOrEqualTo(1),
            "filter=thinking should return at least the sequential file.");

        var paths = results.Select(r => r.filePath).ToList();
        Assert.That(paths, Has.Some.Contain("session-thinking-filter"),
            "Sequential file must appear with filter=thinking.");

        // Memory-only notes must not appear
        Assert.That(paths, Has.None.Contain("note-thinking-filter"),
            "Regular memory note must not appear with filter=thinking.");
    }

    [Test]
    public void CollectRecentActivity_FilterMemory_ExcludesThinkingAndChatFiles()
    {
        // T-COV-001.1: RTM FR-17.3
        // filter=memory should exclude thinking/ and chat/ paths.
        SeedSequentialFile("session-mem-filter", "## Thought 1/2\nThought content.");
        SeedMemoryFile("docs/note-mem-filter.md", "# Doc Note\n\n## Section\nDoc content.");

        GraphTools.Sync();

        var results = RecentActivityReader.CollectRecentActivity(null, "memory", 50);

        var paths = results.Select(r => r.filePath).ToList();
        Assert.That(paths, Has.None.Contain("session-mem-filter"),
            "Sequential (thinking) file must not appear with filter=memory.");
        Assert.That(paths, Has.Some.Contain("note-mem-filter"),
            "Regular memory doc must appear with filter=memory.");
    }

    [Test]
    public void CollectRecentActivity_FilterAssumptions_ReturnsChatPathFiles()
    {
        // T-COV-001.1: RTM FR-17.3
        // filter=assumptions maps to memory://chat/% path.
        SeedChatFile("chat-assumptions-filter.md", "# Chat Session\n\n## Message\nHello from chat.");
        SeedMemoryFile("docs/regular-note.md", "# Note\n\n## Intro\nRegular content.");

        GraphTools.Sync();

        var results = RecentActivityReader.CollectRecentActivity(null, "assumptions", 50);

        var paths = results.Select(r => r.filePath).ToList();
        Assert.That(paths, Has.Some.Contain("chat-assumptions-filter"),
            "Chat file should appear with filter=assumptions.");
        Assert.That(paths, Has.None.Contain("regular-note"),
            "Regular memory note must not appear with filter=assumptions.");
    }

    [Test]
    public void CollectRecentActivity_TimespanFilter_ExcludesOldEntries()
    {
        // T-COV-001.1: RTM FR-17.3
        // A file with last_indexed set to the past must be excluded by a recent timespan.
        SeedSequentialFile("session-recent", "## Thought 1/2\nRecent thought.");
        SeedMemoryFile("docs/old-note.md", "# Old Note\n\n## Intro\nOld content.");

        GraphTools.Sync();

        // Back-date the old note to 2020 in the DB
        SetLastIndexed("old-note.md", "2020-01-01 00:00:00");

        // Query last 1 day — should exclude the 2020 file
        var timespan = TimeSpan.FromHours(24);
        var results = RecentActivityReader.CollectRecentActivity(timespan, null, 50);

        var paths = results.Select(r => r.filePath).ToList();
        Assert.That(paths, Has.None.Contain("old-note"),
            "File backdated to 2020 must be excluded by a 24-hour timespan filter.");
        Assert.That(paths, Has.Some.Contain("session-recent"),
            "Recently indexed file must appear within the 24-hour timespan.");
    }

    [Test]
    public void CollectRecentActivity_Limit_RespectsMaxResults()
    {
        // T-COV-001.1: RTM FR-17.3
        // Seed 5 files, request limit=2, expect exactly 2 results.
        for (int i = 0; i < 5; i++)
        {
            SeedMemoryFile($"docs/limit-note-{i}.md", $"# Note {i.ToString(CultureInfo.InvariantCulture)}\n\n## Section\nContent {i.ToString(CultureInfo.InvariantCulture)}.");
        }

        GraphTools.Sync();

        var results = RecentActivityReader.CollectRecentActivity(null, null, 2);

        Assert.That(results.Count, Is.EqualTo(2),
            "CollectRecentActivity must respect the limit parameter.");
    }

    [Test]
    public void CollectRecentActivity_ThrowsInvalidOperation_WhenNoDatabase()
    {
        // T-COV-001.1: RTM FR-17.3
        // Delete the DB to simulate no-sync state; CollectRecentActivity must throw.
        // We need to delete the file_content table — simplest is to create a brand-new empty DB path.
        var emptyDbPath = Path.Combine(_testRoot, "empty.db");
        Config.SetDatabasePath(emptyDbPath);

        // A new empty SQLite file has no file_content table.
        using (var conn = new SqliteConnection($"Data Source={emptyDbPath};Pooling=false"))
        {
            conn.Open();
            // Intentionally do NOT initialize the schema.
        }

        var ex = Assert.Throws<InvalidOperationException>(
            () => RecentActivityReader.CollectRecentActivity(null, null, 10));

        Assert.That(ex!.Message, Does.Contain("No database found"),
            "Exception message must indicate that no database was found.");

        // Restore the original DB for teardown
        Config.SetDatabasePath(Path.Combine(_testRoot, "memory.db"));
        GraphDatabase.InitializeDatabase();
    }

    // ============================================================
    // RecentActivityReader.ExtractSnippet
    // ============================================================

    [Test]
    public void ExtractSnippet_EmptyInput_ReturnsEmptyString()
    {
        // T-COV-001.1: RTM FR-17.3
        var result = RecentActivityReader.ExtractSnippet(string.Empty, 100);
        Assert.That(result, Is.EqualTo(string.Empty),
            "Empty input should produce an empty snippet.");
    }

    [Test]
    public void ExtractSnippet_ShortContent_ReturnsFullContent()
    {
        // T-COV-001.1: RTM FR-17.3
        const string content = "This is a short sentence.";
        var result = RecentActivityReader.ExtractSnippet(content, 200);
        Assert.That(result, Is.EqualTo(content),
            "Content shorter than maxLength should be returned intact.");
    }

    [Test]
    public void ExtractSnippet_LongContent_TruncatesWithEllipsis()
    {
        // T-COV-001.1: RTM FR-17.3
        var longContent = new string('A', 200);
        var result = RecentActivityReader.ExtractSnippet(longContent, 50);
        Assert.That(result.Length, Is.EqualTo(50),
            "Snippet must be exactly maxLength characters when truncated.");
        Assert.That(result, Does.EndWith("..."),
            "Truncated snippet must end with ellipsis.");
    }

    [Test]
    public void ExtractSnippet_BulletOnlyContent_FallsBackToAllLines()
    {
        // T-COV-001.1: RTM FR-17.3
        // When all lines start with *, the fallback should use all lines.
        const string content = "* Bullet one\n* Bullet two\n* Bullet three";
        var result = RecentActivityReader.ExtractSnippet(content, 1000);
        Assert.That(result, Is.Not.Empty,
            "Bullet-only content should still produce a non-empty snippet.");
        Assert.That(result, Does.Contain("Bullet"),
            "Fallback snippet must include the bullet content.");
    }

    [Test]
    public void ExtractSnippet_MixedContent_SkipsBulletLines()
    {
        // T-COV-001.1: RTM FR-17.3
        // Non-bullet lines take precedence over bullet lines.
        const string content = "* Metadata bullet\nActual prose content here.";
        var result = RecentActivityReader.ExtractSnippet(content, 1000);
        Assert.That(result, Does.Contain("prose content"),
            "Non-bullet lines should be preferred over bullet lines in the snippet.");
    }

    [Test]
    public void ExtractSnippet_UsesDefaultLengthWhenMinusOne()
    {
        // T-COV-001.1: RTM FR-17.3
        // Passing maxLength=-1 should use Config.RecentActivitySnippetLength (default 1000).
        var longContent = new string('X', 2000);
        var result = RecentActivityReader.ExtractSnippet(longContent, -1);
        Assert.That(result.Length, Is.LessThanOrEqualTo(Config.RecentActivitySnippetLength),
            "Default maxLength (-1) should cap at Config.RecentActivitySnippetLength.");
    }

    // ============================================================
    // RecentActivityReader.ExtractSessionId
    // ============================================================

    [Test]
    public void ExtractSessionId_StandardSequentialPath_ExtractsId()
    {
        // T-COV-001.1: RTM FR-17.3
        // memory://thinking/sequential/2026/02/session-12345.md → session-12345
        var thinkingFolder = Path.GetFileName(Config.ThinkingPath);
        var filePath = $"memory://{thinkingFolder}/sequential/2026/02/session-12345.md";
        var id = RecentActivityReader.ExtractSessionId(filePath);
        Assert.That(id, Is.EqualTo("session-12345"),
            "ExtractSessionId should strip the path prefix and return the filename without extension.");
    }

    [Test]
    public void ExtractSessionId_SimpleMemoryPath_ExtractsFilename()
    {
        // T-COV-001.1: RTM FR-17.3
        // memory://docs/my-note.md → my-note
        var id = RecentActivityReader.ExtractSessionId("memory://docs/my-note.md");
        Assert.That(id, Is.EqualTo("my-note"),
            "ExtractSessionId on a simple memory path should return the base filename without extension.");
    }

    [Test]
    public void ExtractSessionId_ChatPath_ExtractsFilename()
    {
        // T-COV-001.1: RTM FR-17.3
        // memory://chat/2026/02/chat-session.md → chat-session
        var id = RecentActivityReader.ExtractSessionId("memory://chat/2026/02/chat-session.md");
        Assert.That(id, Is.EqualTo("chat-session"),
            "ExtractSessionId on a chat path should return the base filename without extension.");
    }

    // ============================================================
    // RecentActivityReader.DetermineFileType
    // ============================================================

    [Test]
    public void DetermineFileType_SequentialPath_ReturnsSequential()
    {
        // T-COV-001.1: RTM FR-17.3
        var thinkingFolder = Path.GetFileName(Config.ThinkingPath);
        var seqFolder = Path.GetFileName(Config.SequentialPath);
        var filePath = $"memory://{thinkingFolder}/{seqFolder}/2026/02/session-abc.md";
        var type = RecentActivityReader.DetermineFileType(filePath);
        Assert.That(type, Is.EqualTo("sequential"),
            "Sequential thinking session path must be identified as type 'sequential'.");
    }

    [Test]
    public void DetermineFileType_WorkflowPath_ReturnsWorkflow()
    {
        // T-COV-001.1: RTM FR-17.3
        var thinkingFolder = Path.GetFileName(Config.ThinkingPath);
        var filePath = $"memory://{thinkingFolder}/workflow/2026/02/wf-session.md";
        var type = RecentActivityReader.DetermineFileType(filePath);
        Assert.That(type, Is.EqualTo("workflow"),
            "Workflow session path must be identified as type 'workflow'.");
    }

    [Test]
    public void DetermineFileType_MemoryPath_ReturnsMemory()
    {
        // T-COV-001.1: RTM FR-17.3
        var type = RecentActivityReader.DetermineFileType("memory://docs/my-note.md");
        Assert.That(type, Is.EqualTo("memory"),
            "Regular memory file path must be identified as type 'memory'.");
    }

    [Test]
    public void DetermineFileType_ChatPath_ReturnsMemory()
    {
        // T-COV-001.1: RTM FR-17.3
        // Chat files (assumptions) are not in thinking/ — they resolve to "memory".
        var type = RecentActivityReader.DetermineFileType("memory://chat/2026/02/assumptions.md");
        Assert.That(type, Is.EqualTo("memory"),
            "Chat/assumption file path must resolve to type 'memory'.");
    }

    // ============================================================
    // RecentActivityFormatter.FormatActivityReport
    // ============================================================

    [Test]
    public void FormatActivityReport_EmptyResults_ReturnsNoActivityMessage()
    {
        // T-COV-001.1: RTM FR-17.3
        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>();
        var output = RecentActivityFormatter.FormatActivityReport(results, false);
        Assert.That(output, Is.EqualTo("No recent activity found"),
            "Empty results must produce the 'No recent activity found' message.");
    }

    [Test]
    public void FormatActivityReport_SequentialEntry_IncludesExpectedFields()
    {
        // T-COV-001.1: RTM FR-17.3
        var thinkingFolder = Path.GetFileName(Config.ThinkingPath);
        var seqFolder = Path.GetFileName(Config.SequentialPath);
        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>
        {
            (
                $"memory://{thinkingFolder}/{seqFolder}/2026/02/session-fmt-001.md",
                "Test Sequential Session",
                "2026-02-18 10:00:00",
                "# Test Sequential Session\n\n## Thought 1/2\nFirst thought content here.\n\n## Thought 2/2\nSecond thought content here.",
                "active"
            )
        };

        var output = RecentActivityFormatter.FormatActivityReport(results, false);

        Assert.That(output, Does.Contain("# Recent Activity"),
            "Output header must be present.");
        Assert.That(output, Does.Contain("session-fmt-001"),
            "Session ID must appear in formatted output.");
        Assert.That(output, Does.Contain("(sequential)"),
            "Type label '(sequential)' must appear for a sequential session.");
        Assert.That(output, Does.Contain("Thoughts:"),
            "Sequential session must show thought count.");
        Assert.That(output, Does.Contain("Status: active"),
            "Session status must appear in formatted output.");
    }

    [Test]
    public void FormatActivityReport_SequentialEntry_IncludeContentTrue_ShowsH2ContentFields()
    {
        // T-COV-001.1: RTM FR-17.3
        var thinkingFolder = Path.GetFileName(Config.ThinkingPath);
        var seqFolder = Path.GetFileName(Config.SequentialPath);
        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>
        {
            (
                $"memory://{thinkingFolder}/{seqFolder}/2026/02/session-fmt-content.md",
                "Content Session",
                "2026-02-18 11:00:00",
                "# Content Session\n\n## Thought 1/2\nFirst prose.\n\n## Thought 2/2\nSecond prose.",
                "active"
            )
        };

        var output = RecentActivityFormatter.FormatActivityReport(results, includeContent: true);

        Assert.That(output, Does.Contain("First H2:"),
            "includeContent=true must produce 'First H2:' field.");
        Assert.That(output, Does.Contain("First H2 Content:"),
            "includeContent=true must produce 'First H2 Content:' field.");
    }

    [Test]
    public void FormatActivityReport_SequentialEntry_IncludeContentFalse_ShowsSnippet()
    {
        // T-COV-001.1: RTM FR-17.3
        var thinkingFolder = Path.GetFileName(Config.ThinkingPath);
        var seqFolder = Path.GetFileName(Config.SequentialPath);
        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>
        {
            (
                $"memory://{thinkingFolder}/{seqFolder}/2026/02/session-fmt-snippet.md",
                "Snippet Session",
                "2026-02-18 12:00:00",
                "# Snippet Session\n\n## Thought 1/3\nThought content for snippet.\n\n## Thought 2/3\nSecond thought.\n\n## Thought 3/3\nThird thought.",
                "active"
            )
        };

        var output = RecentActivityFormatter.FormatActivityReport(results, includeContent: false);

        Assert.That(output, Does.Contain("First:"),
            "includeContent=false must produce 'First:' snippet field.");
        Assert.That(output, Does.Not.Contain("First H2:"),
            "includeContent=false must NOT produce 'First H2:' field.");
    }

    [Test]
    public void FormatActivityReport_WorkflowEntry_IncludesStepsAndStatus()
    {
        // T-COV-001.1: RTM FR-17.3
        var thinkingFolder = Path.GetFileName(Config.ThinkingPath);
        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>
        {
            (
                $"memory://{thinkingFolder}/workflow/2026/02/wf-fmt-001.md",
                "Workflow Fmt Test",
                "2026-02-18 13:00:00",
                "# Workflow Fmt Test\n\n## Step 1: Init\nInitialization response.\n\n## Response\nFinal result.",
                "active"
            )
        };

        var output = RecentActivityFormatter.FormatActivityReport(results, false);

        Assert.That(output, Does.Contain("wf-fmt-001"),
            "Workflow session ID must appear in output.");
        Assert.That(output, Does.Contain("(workflow)"),
            "Type label '(workflow)' must appear for a workflow session.");
        Assert.That(output, Does.Contain("Steps:"),
            "Workflow entry must show step count.");
        Assert.That(output, Does.Contain("Status: active"),
            "Workflow status must appear in formatted output.");
    }

    [Test]
    public void FormatActivityReport_MemoryEntry_IncludesTitleAndSections()
    {
        // T-COV-001.1: RTM FR-17.3
        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>
        {
            (
                "memory://docs/memory-fmt-test.md",
                "Memory Format Test",
                "2026-02-18 14:00:00",
                "# Memory Format Test\n\n## Overview\nOverview content.\n\n## Details\nDetailed content here.",
                "active"
            )
        };

        var output = RecentActivityFormatter.FormatActivityReport(results, false);

        Assert.That(output, Does.Contain("memory-fmt-test"),
            "Memory file slug must appear in output.");
        Assert.That(output, Does.Contain("(memory)"),
            "Type label '(memory)' must appear for a regular memory file.");
        Assert.That(output, Does.Contain("Title: Memory Format Test"),
            "Memory file title must appear in formatted output.");
        Assert.That(output, Does.Contain("Sections:"),
            "Memory file must show section count.");
    }

    [Test]
    public void FormatActivityReport_MemoryEntry_IncludeContentTrue_ShowsH2Fields()
    {
        // T-COV-001.1: RTM FR-17.3
        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>
        {
            (
                "memory://docs/mem-content-test.md",
                "Memory Content Test",
                "2026-02-18 15:00:00",
                "# Memory Content Test\n\n## First Section\nFirst section content.\n\n## Last Section\nLast section content.",
                "active"
            )
        };

        var output = RecentActivityFormatter.FormatActivityReport(results, includeContent: true);

        Assert.That(output, Does.Contain("First H2: First Section"),
            "includeContent=true must show 'First H2:' with heading name for memory file.");
        Assert.That(output, Does.Contain("First H2 Content:"),
            "includeContent=true must show 'First H2 Content:' for memory file.");
        Assert.That(output, Does.Contain("Last H2: Last Section"),
            "includeContent=true must show 'Last H2:' with heading name for memory file.");
    }

    [Test]
    public void FormatActivityReport_MultipleEntries_AllEntitiesPresent()
    {
        // T-COV-001.1: RTM FR-17.3
        var thinkingFolder = Path.GetFileName(Config.ThinkingPath);
        var seqFolder = Path.GetFileName(Config.SequentialPath);
        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>
        {
            (
                $"memory://{thinkingFolder}/{seqFolder}/2026/02/session-multi-a.md",
                "Session A",
                "2026-02-18 16:00:00",
                "# Session A\n\n## Thought 1/1\nA thought.",
                "active"
            ),
            (
                "memory://docs/note-multi-b.md",
                "Note B",
                "2026-02-18 15:00:00",
                "# Note B\n\n## Section B\nSome note content.",
                "active"
            )
        };

        var output = RecentActivityFormatter.FormatActivityReport(results, false);

        Assert.That(output, Does.Contain("session-multi-a"),
            "First entry (sequential) must appear in output.");
        Assert.That(output, Does.Contain("note-multi-b"),
            "Second entry (memory) must appear in output.");
        Assert.That(output, Does.Contain("(sequential)"),
            "Sequential type label must appear.");
        Assert.That(output, Does.Contain("(memory)"),
            "Memory type label must appear.");
    }

    // ============================================================
    // RecentActivityTools.RecentActivity — end-to-end
    // ============================================================

    [Test]
    public void RecentActivity_NegativeTimespan_ReturnsError()
    {
        // T-COV-001.1: RTM FR-17.3
        // Negative timespan should return an error string immediately.
        var negativeTimespan = TimeSpan.FromHours(-1);
        var result = RecentActivityTools.RecentActivity(timespan: negativeTimespan);
        Assert.That(result, Does.Contain("ERROR: timespan parameter must be positive"),
            "Negative timespan must produce a descriptive error message.");
    }

    [Test]
    public void RecentActivity_NoDatabase_ReturnsNoDatabaseMessage()
    {
        // T-COV-001.1: RTM FR-17.3
        // Point to a DB path that has no file_content table; the tool should handle gracefully.
        var emptyDbPath = Path.Combine(_testRoot, "no-table.db");
        Config.SetDatabasePath(emptyDbPath);

        // Create an empty SQLite file with no schema
        using (var conn = new SqliteConnection($"Data Source={emptyDbPath};Pooling=false"))
        {
            conn.Open();
        }

        var result = RecentActivityTools.RecentActivity();

        Assert.That(result, Does.Contain("No database found"),
            "When file_content table is missing, tool must return 'No database found' message.");

        // Restore original DB for teardown
        Config.SetDatabasePath(Path.Combine(_testRoot, "memory.db"));
        GraphDatabase.InitializeDatabase();
    }

    [Test]
    public void RecentActivity_LearnMode_ReturnsHelpContent()
    {
        // T-COV-001.1: RTM FR-17.3
        var result = RecentActivityTools.RecentActivity(learn: true);
        Assert.That(result, Is.Not.Null.And.Not.Empty,
            "Learn mode must return non-empty help documentation.");
        // Help content is expected to contain a tool name reference or description
        Assert.That(result.Length, Is.GreaterThan(20),
            "Learn mode must return substantial documentation, not an empty string.");
    }

    [Test]
    public void RecentActivity_NormalQuery_ReturnsFormattedReport()
    {
        // T-COV-001.1: RTM FR-17.3
        // End-to-end: seed files → Sync → RecentActivity → check output.
        SeedSequentialFile("session-e2e-001", "## Thought 1/2\nFirst thought.\n\n## Thought 2/2\nSecond thought.");
        SeedMemoryFile("docs/e2e-note.md", "# End-To-End Note\n\n## Section One\nNote content.");

        GraphTools.Sync();

        var result = RecentActivityTools.RecentActivity(limit: 20);

        Assert.That(result, Does.Contain("# Recent Activity"),
            "Output must start with '# Recent Activity' header.");
        Assert.That(result, Does.Contain("session-e2e-001"),
            "Sequential session must appear in end-to-end output.");
        Assert.That(result, Does.Contain("e2e-note"),
            "Memory note must appear in end-to-end output.");
    }

    [Test]
    public void RecentActivity_FilterThinking_ExcludesMemoryFiles()
    {
        // T-COV-001.1: RTM FR-17.3
        // End-to-end with filter=thinking: memory files must not appear.
        SeedSequentialFile("session-e2e-filter", "## Thought 1/1\nSingle thought.");
        SeedMemoryFile("docs/excluded-note.md", "# Excluded Note\n\n## Section\nMust not appear.");

        GraphTools.Sync();

        var result = RecentActivityTools.RecentActivity(filter: "thinking", limit: 20);

        Assert.That(result, Does.Contain("session-e2e-filter"),
            "Sequential session must appear in thinking-filtered output.");
        Assert.That(result, Does.Not.Contain("excluded-note"),
            "Regular memory note must not appear with filter=thinking.");
    }

    [Test]
    public void RecentActivity_Limit_ReturnsAtMostLimitEntries()
    {
        // T-COV-001.1: RTM FR-17.3
        // Seed 4 memory files, query with limit=2, count result entries.
        for (int i = 0; i < 4; i++)
        {
            SeedMemoryFile($"docs/limit-e2e-{i}.md", $"# Limit Note {i.ToString(CultureInfo.InvariantCulture)}\n\n## S\nContent.");
        }

        GraphTools.Sync();

        var result = RecentActivityTools.RecentActivity(limit: 2);

        // Count bold entry headers (each entry starts with "**<id>**")
        var boldLineCount = result.Split('\n')
            .Count(line => line.TrimStart().StartsWith("**", StringComparison.Ordinal));

        Assert.That(boldLineCount, Is.EqualTo(2),
            "limit=2 must return exactly 2 activity entries in the formatted output.");
    }

    [Test]
    public void RecentActivity_TimespanFilter_ExcludesOldEntries_EndToEnd()
    {
        // T-COV-001.1: RTM FR-17.3
        // Seed a file, sync, back-date it, query with 1-day timespan.
        SeedMemoryFile("docs/e2e-old-note.md", "# Old E2E Note\n\n## Intro\nOld content.");
        SeedMemoryFile("docs/e2e-recent-note.md", "# Recent E2E Note\n\n## Intro\nRecent content.");

        GraphTools.Sync();

        SetLastIndexed("e2e-old-note.md", "2019-06-01 00:00:00");

        var result = RecentActivityTools.RecentActivity(timespan: TimeSpan.FromHours(24), limit: 20);

        Assert.That(result, Does.Not.Contain("e2e-old-note"),
            "File backdated to 2019 must be excluded by a 24-hour timespan filter in end-to-end flow.");
        Assert.That(result, Does.Contain("e2e-recent-note"),
            "Recently indexed file must appear within the 24-hour timespan in end-to-end flow.");
    }

    // ============================================================
    // Helpers
    // ============================================================

    private static void SeedSequentialFile(string sessionId, string thoughtContent)
    {
        var seqDir = Path.Combine(
            Config.SequentialPath,
            DateTime.UtcNow.Year.ToString(CultureInfo.InvariantCulture),
            DateTime.UtcNow.ToString("MM", CultureInfo.InvariantCulture));
        Directory.CreateDirectory(seqDir);

        var content = $"# {sessionId}\n\n{thoughtContent}\n";
        File.WriteAllText(Path.Combine(seqDir, $"{sessionId}.md"), content);
    }

    private static void SeedMemoryFile(string relativePath, string content)
    {
        var fullPath = Path.Combine(Config.MemoryPath, relativePath);
        Directory.CreateDirectory(Path.GetDirectoryName(fullPath)!);
        File.WriteAllText(fullPath, content);
    }

    private static void SeedChatFile(string fileName, string content)
    {
        var chatDir = Path.Combine(
            Config.MemoryPath,
            "chat",
            DateTime.UtcNow.Year.ToString(CultureInfo.InvariantCulture),
            DateTime.UtcNow.ToString("MM", CultureInfo.InvariantCulture));
        Directory.CreateDirectory(chatDir);
        File.WriteAllText(Path.Combine(chatDir, fileName), content);
    }

    /// <summary>
    /// Back-dates last_indexed for a file whose URI fragment matches the given name (without extension).
    /// Used to simulate stale entries for timespan filter tests.
    /// The file_content table stores URIs without .md extension, so the fragment should omit it.
    /// </summary>
    private static void SetLastIndexed(string fileNameFragmentWithoutExtension, string timestampUtc)
    {
        // Strip .md from the fragment if caller passed it with extension
        var fragment = fileNameFragmentWithoutExtension.EndsWith(".md", StringComparison.OrdinalIgnoreCase)
            ? fileNameFragmentWithoutExtension[..^3]
            : fileNameFragmentWithoutExtension;

        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "UPDATE file_content SET last_indexed = @ts WHERE file_path LIKE @pattern";
        cmd.Parameters.AddWithValue("@ts", timestampUtc);
        cmd.Parameters.AddWithValue("@pattern", $"%{fragment}%");
        var rowsAffected = cmd.ExecuteNonQuery();
        Assert.That(rowsAffected, Is.GreaterThanOrEqualTo(1),
            $"SetLastIndexed: expected to update at least one row matching fragment '{fragment}' but updated {rowsAffected} rows.");
    }
}
