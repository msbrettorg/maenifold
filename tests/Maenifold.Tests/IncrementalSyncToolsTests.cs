using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Threading;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-COV-001.3: RTM FR-17.5
[NonParallelizable]
public class IncrementalSyncToolsTests
{
    private MethodInfo _createdHandler = null!;
    private MethodInfo _changedHandler = null!;
    private MethodInfo _deletedHandler = null!;
    private MethodInfo _errorHandler = null!;

    [OneTimeSetUp]
    public void OneTimeSetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();

        var type = typeof(IncrementalSyncTools);
        _createdHandler = type.GetMethod("ProcessFileCreated", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileCreated handler not found.");
        _changedHandler = type.GetMethod("ProcessFileChanged", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileChanged handler not found.");
        _deletedHandler = type.GetMethod("ProcessFileDeleted", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileDeleted handler not found.");
        _errorHandler = type.GetMethod("OnWatcherError", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("OnWatcherError handler not found.");
    }

    [SetUp]
    public void SetUp()
    {
        // Verify test isolation - ensure we're using test database
        Console.WriteLine($"[TEST SETUP] Using database: {Config.DatabasePath}");
        Console.WriteLine($"[TEST SETUP] Using memory path: {Config.MemoryPath}");

        // Ensure test database exists
        GraphDatabase.InitializeDatabase();

        // Clear any existing test data to ensure clean state
        try
        {
            using var conn = new SqliteConnection($"Data Source={Config.DatabasePath}");
            conn.OpenWithWAL();
            conn.Execute("DELETE FROM concept_mentions WHERE source_file LIKE 'memory://tests/%'");
            conn.Execute("DELETE FROM concept_graph_files WHERE source_file LIKE 'memory://tests/%'");
            conn.Execute("DELETE FROM concept_graph WHERE concept_a IN (SELECT concept_name FROM concepts WHERE concept_name LIKE '%test%' OR concept_name LIKE '%incremental%' OR concept_name LIKE '%watcher%')");
            conn.Execute("DELETE FROM file_content WHERE file_path LIKE 'memory://tests/%'");
            conn.Execute("DELETE FROM concepts WHERE concept_name LIKE '%test%' OR concept_name LIKE '%incremental%' OR concept_name LIKE '%watcher%'");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"[TEST SETUP WARNING] Failed to clean test data: {ex.Message}");
            // Re-initialize if needed
            GraphDatabase.InitializeDatabase();
        }
    }



    [Test]
    public void WatcherErrorsAreLogged()
    {
        // Note: This test doesn't use the database, so we don't need SetUp to run
        // We're only testing error logging functionality

        // Arrange: Capture Console.Error output
        var originalError = Console.Error;
        var capturedError = new StringWriter();
        Console.SetError(capturedError);

        try
        {
            // Arrange: Create test exception and ErrorEventArgs
            var testException = new InvalidOperationException("Test watcher error");
            var errorEventArgs = new ErrorEventArgs(testException);

            // Act: Invoke the error handler
            _errorHandler.Invoke(null, new object[] { null!, errorEventArgs });

            // Assert: Verify error was logged
            var errorOutput = capturedError.ToString();
            Assert.That(errorOutput, Does.Contain("[FileWatcher] Error: Test watcher error"),
                "Watcher errors should be logged to Console.Error with proper prefix");
        }
        finally
        {
            // Cleanup: Restore original Console.Error
            Console.SetError(originalError);
        }
    }

    // T-COV-001.3: RTM FR-17.5 — ProcessFileCreated inserts concepts and mentions into DB
    [Test]
    public void ProcessFileCreated_WithWikiLinks_InsertsConceptMentionsIntoDb()
    {
        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"incremental-sync-created-a-{unique}";
        var conceptB = $"incremental-sync-created-b-{unique}";
        var content = $"# Test Note\n\nThis references [[{conceptA}]] and [[{conceptB}]].\n";
        var (filePath, memoryUri) = CreateTestFile(content, "tests/incremental-sync-tools");

        try
        {
            InvokeHandler(_createdHandler, filePath);

            var countA = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA));
            var countB = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB));

            Assert.That(countA, Is.EqualTo(1),
                "ProcessFileCreated must insert a concept_mention row for each WikiLink in the created file.");
            Assert.That(countB, Is.EqualTo(1),
                "ProcessFileCreated must insert a concept_mention row for each WikiLink in the created file.");

            var fileRow = GetFileContentRow(memoryUri);
            Assert.That(fileRow, Is.Not.Null,
                "ProcessFileCreated must insert a file_content row for the created file.");
        }
        finally
        {
            CleanupFile(filePath);
        }
    }

    // T-COV-001.3: RTM FR-17.5 — ProcessFileChanged updates DB when file hash changes
    [Test]
    public void ProcessFileChanged_WithNewContent_AddsMentionForNewConceptAndUpdatesFileContent()
    {
        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"incremental-sync-changed-a-{unique}";
        var conceptB = $"incremental-sync-changed-b-{unique}";
        var contentV1 = $"# Test Note\n\nThis references [[{conceptA}]].\n";
        var (filePath, memoryUri) = CreateTestFile(contentV1, "tests/incremental-sync-tools");

        try
        {
            InvokeHandler(_createdHandler, filePath);

            var countAfterCreate = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA));
            Assert.That(countAfterCreate, Is.EqualTo(1),
                "ProcessFileCreated must insert mention for initial concept.");

            var hashAfterCreate = GetFileMd5(memoryUri);
            Assert.That(hashAfterCreate, Is.Not.Null,
                "ProcessFileCreated must persist a content hash in file_content.");

            // Write different content with a new concept so the hash will differ
            var contentV2 = $"# Test Note\n\nThis now also references [[{conceptB}]].\n";
            File.WriteAllText(filePath, contentV2);

            InvokeHandler(_changedHandler, filePath);

            // Incremental sync must insert/update the mention for the new concept
            var countBAfterChange = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB));
            Assert.That(countBAfterChange, Is.EqualTo(1),
                "ProcessFileChanged must add a mention for a newly introduced WikiLink.");

            // The file_content row must reflect the updated content hash
            var hashAfterChange = GetFileMd5(memoryUri);
            Assert.That(hashAfterChange, Is.Not.EqualTo(hashAfterCreate),
                "ProcessFileChanged must update the content hash when file content changes.");
        }
        finally
        {
            CleanupFile(filePath);
        }
    }

    // T-COV-001.3: RTM FR-17.5 — ProcessFileDeleted removes DB entries for the file
    [Test]
    public void ProcessFileDeleted_AfterCreate_RemovesFileContentAndMentionsFromDb()
    {
        var unique = Guid.NewGuid().ToString("N");
        var concept = $"incremental-sync-deleted-{unique}";
        var content = $"# Test Note\n\nThis references [[{concept}]].\n";
        var (filePath, memoryUri) = CreateTestFile(content, "tests/incremental-sync-tools");

        try
        {
            InvokeHandler(_createdHandler, filePath);

            var countAfterCreate = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(concept));
            Assert.That(countAfterCreate, Is.EqualTo(1),
                "ProcessFileCreated must insert mention before deletion test.");

            var fileRowAfterCreate = GetFileContentRow(memoryUri);
            Assert.That(fileRowAfterCreate, Is.Not.Null,
                "ProcessFileCreated must insert file_content row before deletion test.");

            File.Delete(filePath);
            InvokeHandler(_deletedHandler, filePath);

            var fileRowAfterDelete = GetFileContentRow(memoryUri);
            Assert.That(fileRowAfterDelete, Is.Null,
                "ProcessFileDeleted must remove the file_content row from the database.");

            var countAfterDelete = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(concept));
            Assert.That(countAfterDelete, Is.Null,
                "ProcessFileDeleted must remove concept_mention rows for the deleted file.");
        }
        finally
        {
            if (File.Exists(filePath))
                File.Delete(filePath);
        }
    }

    // T-COV-001.3: RTM FR-17.5 — FileSystemWatcher is configured with *.md filter
    [Test]
    public void StartWatcher_ConfiguresWatcherWithMarkdownFilter()
    {
        // Ensure watcher is stopped before this test
        IncrementalSyncTools.StopWatcher();

        IncrementalSyncTools.StartWatcher();
        try
        {
            var watcherField = typeof(IncrementalSyncTools)
                .GetField("_watcher", BindingFlags.Static | BindingFlags.NonPublic);
            Assert.That(watcherField, Is.Not.Null, "_watcher field must exist.");
            var watcher = watcherField!.GetValue(null) as FileSystemWatcher;
            Assert.That(watcher, Is.Not.Null, "_watcher must be non-null after StartWatcher.");

            // Verify the watcher is configured to watch only *.md files
            Assert.That(watcher!.Filter, Is.EqualTo("*.md"),
                "FileSystemWatcher must be configured with '*.md' filter to ignore non-markdown files.");

            // Verify subdirectories are included
            Assert.That(watcher.IncludeSubdirectories, Is.True,
                "FileSystemWatcher must include subdirectories to watch nested memory folders.");

            // Verify the watcher is watching the correct path
            Assert.That(watcher.Path, Is.EqualTo(Config.MemoryPath),
                "FileSystemWatcher must watch the configured MemoryPath.");
        }
        finally
        {
            IncrementalSyncTools.StopWatcher();
        }
    }

    // T-COV-001.3: RTM FR-17.5 — ProcessFileCreated handles path outside memory path gracefully
    [Test]
    public void ProcessFileCreated_WithPathOutsideMemoryPath_DoesNotThrow()
    {
        var outsidePath = Path.Combine(Path.GetTempPath(), $"outside-memory-{Guid.NewGuid():N}.md");
        File.WriteAllText(outsidePath, "# Outside\n\n[[some-concept]]\n");

        try
        {
            Assert.DoesNotThrow(() => InvokeHandler(_createdHandler, outsidePath),
                "ProcessFileCreated must handle paths outside the memory path without throwing.");
        }
        finally
        {
            CleanupFile(outsidePath);
        }
    }

    // T-COV-001.3: RTM FR-17.5 — StartWatcher/StopWatcher lifecycle
    [Test]
    public void StartWatcher_ThenStopWatcher_LifecycleReturnsExpectedMessages()
    {
        // Ensure watcher is not running from a previous test
        IncrementalSyncTools.StopWatcher();

        var startResult = IncrementalSyncTools.StartWatcher();
        try
        {
            Assert.That(startResult, Does.Contain("Started watching"),
                "StartWatcher must return a confirmation message containing 'Started watching'.");
            Assert.That(startResult, Does.Contain(Config.MemoryPath),
                "StartWatcher confirmation must include the watched path.");

            // Calling StartWatcher again while running must return guard message
            var secondStartResult = IncrementalSyncTools.StartWatcher();
            Assert.That(secondStartResult, Is.EqualTo("Watcher already running"),
                "StartWatcher called while already running must return 'Watcher already running'.");

            // Verify watcher state via reflection
            var watcherField = typeof(IncrementalSyncTools)
                .GetField("_watcher", BindingFlags.Static | BindingFlags.NonPublic);
            Assert.That(watcherField, Is.Not.Null, "_watcher static field must exist.");
            var watcher = watcherField!.GetValue(null);
            Assert.That(watcher, Is.Not.Null, "_watcher must be non-null after StartWatcher.");
        }
        finally
        {
            var stopResult = IncrementalSyncTools.StopWatcher();
            Assert.That(stopResult, Is.EqualTo("Stopped incremental sync watcher"),
                "StopWatcher must return 'Stopped incremental sync watcher'.");
        }

        // After stopping, calling StopWatcher again must return guard message
        var secondStopResult = IncrementalSyncTools.StopWatcher();
        Assert.That(secondStopResult, Is.EqualTo("Watcher not running"),
            "StopWatcher called when not running must return 'Watcher not running'.");

        // Verify watcher is null after stop
        var watcherFieldAfterStop = typeof(IncrementalSyncTools)
            .GetField("_watcher", BindingFlags.Static | BindingFlags.NonPublic);
        var watcherAfterStop = watcherFieldAfterStop!.GetValue(null);
        Assert.That(watcherAfterStop, Is.Null, "_watcher must be null after StopWatcher.");
    }

    // T-COV-001.3: RTM FR-17.5 — Debounce coalesces rapid changes to same path
    [Test]
    public void Debounce_RapidChangesToSamePath_CoalescesIntoSingleProcessing()
    {
        var unique = Guid.NewGuid().ToString("N");
        var conceptFinal = $"incremental-sync-debounce-final-{unique}";
        var (filePath, memoryUri) = CreateTestFile(
            $"# Version 1\n\n[[incremental-sync-debounce-v1-{unique}]]\n",
            "tests/incremental-sync-tools");

        try
        {
            // Write final content before starting the debounce test
            var finalContent = $"# Final Version\n\n[[{conceptFinal}]]\n";
            File.WriteAllText(filePath, finalContent);

            // Access the private _debounceTimers dictionary via reflection
            var timersField = typeof(IncrementalSyncTools)
                .GetField("_debounceTimers", BindingFlags.Static | BindingFlags.NonPublic);
            Assert.That(timersField, Is.Not.Null, "_debounceTimers field must exist.");

            // Access DebounceFileOperation via reflection to test the coalescing mechanism
            var debounceMethod = typeof(IncrementalSyncTools)
                .GetMethod("DebounceFileOperation", BindingFlags.Static | BindingFlags.NonPublic);
            Assert.That(debounceMethod, Is.Not.Null, "DebounceFileOperation method must exist.");

            // Schedule multiple rapid debounces for the same path
            for (var i = 0; i < 5; i++)
            {
                debounceMethod!.Invoke(null, new object[] { filePath, WatcherChangeTypes.Changed, null! });
                Thread.Sleep(10); // 10ms between calls — well within default 150ms debounce window
            }

            // Verify that only one timer is pending for this path (debounce coalescing)
            var timers = timersField!.GetValue(null) as Dictionary<string, (Timer timer, WatcherChangeTypes changeType, object? eventArgs)>;
            Assert.That(timers, Is.Not.Null, "_debounceTimers must be a Dictionary.");
            Assert.That(timers!.ContainsKey(filePath), Is.True,
                "After rapid debounce calls, exactly one timer must be pending for the path.");

            // Wait for debounce window to fire and processing to complete
            Thread.Sleep(400); // 150ms debounce + 250ms buffer

            // Verify the final state reflects the last write
            var count = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptFinal));
            Assert.That(count, Is.EqualTo(1),
                "After debounce coalescing, the final file content must be processed exactly once.");
        }
        finally
        {
            CleanupFile(filePath);
        }
    }

    private static void InvokeHandler(MethodInfo handler, string path)
    {
        handler.Invoke(null, new object[] { path });
    }

    private static (string FilePath, string MemoryUri) CreateTestFile(string content, string subfolder)
    {
        var testFolder = Path.Combine(Config.MemoryPath, subfolder);
        Directory.CreateDirectory(testFolder);

        var uniqueSuffix = Guid.NewGuid().ToString("N");
        var filePath = Path.Combine(testFolder, $"note-{uniqueSuffix}.md");
        File.WriteAllText(filePath, content);

        var memoryUri = MarkdownIO.PathToUri(filePath, Config.MemoryPath);
        return (filePath, memoryUri);
    }

    private static void CleanupFile(string filePath)
    {
        try { if (File.Exists(filePath)) File.Delete(filePath); }
        catch { /* best-effort cleanup */ }
    }

    private static int? GetMentionCount(string memoryUri, string conceptName)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT mention_count FROM concept_mentions WHERE concept_name = @concept AND source_file = @file";
        cmd.Parameters.AddWithValue("@concept", conceptName);
        cmd.Parameters.AddWithValue("@file", memoryUri);

        var result = cmd.ExecuteScalar();
        if (result == null || result == DBNull.Value)
            return null;

        return Convert.ToInt32(result, CultureInfo.InvariantCulture);
    }

    private static string? GetFileContentRow(string memoryUri)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT file_path FROM file_content WHERE file_path = @path";
        cmd.Parameters.AddWithValue("@path", memoryUri);

        using var reader = cmd.ExecuteReader();
        return reader.Read() ? reader.GetString(0) : null;
    }

    private static string? GetFileMd5(string memoryUri)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT file_md5 FROM file_content WHERE file_path = @path";
        cmd.Parameters.AddWithValue("@path", memoryUri);

        using var reader = cmd.ExecuteReader();
        if (!reader.Read()) return null;
        return reader.IsDBNull(0) ? null : reader.GetString(0);
    }
}
