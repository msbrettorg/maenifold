using System;
using System.Globalization;
using System.IO;
using System.Text;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-SYNC-MTIME-001.1: RTM FR-14.1, FR-14.2, FR-14.3, FR-14.4; NFR-14.1.1, NFR-14.2.1, NFR-14.4.1
[TestFixture]
[NonParallelizable]
public class SyncMtimeOptimizationTests
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
        _testRoot = Path.Combine(repoRoot, "test-outputs", "sync-mtime-optimization", $"run-{Guid.NewGuid():N}");

        // Ensure a deterministic, repo-local test location.
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
                // Best-effort: ensure no read-only flags block cleanup.
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
            // Ignore cleanup failures; artifacts are under test-outputs for debugging.
        }

        // Restore shared test environment state.
        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", string.IsNullOrEmpty(_previousMaenifoldRootEnv) ? null : _previousMaenifoldRootEnv);
        Environment.SetEnvironmentVariable("MAENIFOLD_DATABASE_PATH", string.IsNullOrEmpty(_previousDatabasePathEnv) ? null : _previousDatabasePathEnv);

        // If the test suite started with no overrides, reset them rather than calling OverrideRoot("").
        if (string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv) && string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
        {
            Config.ResetOverrides();
        }
        else
        {
            if (!string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv))
            {
                Config.OverrideRoot(_previousMaenifoldRootEnv);
            }

            if (!string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
            {
                Config.SetDatabasePath(_previousDatabasePathEnv);
            }
        }
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
    }

    [Test]
    public void Sync_WhenMtimeUnchanged_SkipsWithoutRead_AndDoesNotRestoreMentionsOrEdges_OnUnix()
    {
        // T-SYNC-MTIME-001.1: RTM FR-14.1

        if (OperatingSystem.IsWindows())
        {
            Assert.Ignore("FR-14.1 'make unreadable without changing mtime' is Unix-only per test plan.");
        }

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-mtime-a-{unique}";
        var conceptB = $"sync-mtime-b-{unique}";

        var (filePath, memoryUri) = CreateTestMarkdownFile(
            $"# Sync Mtime Optimization Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n");

        GraphTools.Sync();

        var originalMtime = File.GetLastWriteTimeUtc(filePath);
        Assert.That(GetFileContentRow(memoryUri), Is.Not.Null, "Precondition: file_content row should exist after initial Sync().");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.EqualTo(1));
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.EqualTo(1));

        TamperDb_DeleteMentions(memoryUri);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.Null);

        var stderr = new StringBuilder();
        var previousStderr = Console.Error;
        using var capture = new StringWriter(stderr, CultureInfo.InvariantCulture);
        Console.SetError(capture);

        try
        {
            // Ensure mtime is unchanged; chmod should not mutate mtime.
            File.SetLastWriteTimeUtc(filePath, originalMtime);

            // Make file unreadable without changing mtime.
#pragma warning disable CA1416 // Guarded by OS check.
            var originalMode = File.GetUnixFileMode(filePath);
            File.SetUnixFileMode(filePath, UnixFileMode.None);
            try
            {
                Assert.That(File.GetLastWriteTimeUtc(filePath), Is.EqualTo(originalMtime),
                    "Precondition: making file unreadable must not change mtime.");
                GraphTools.Sync();
            }
            finally
            {
                try
                {
                    File.SetUnixFileMode(filePath, originalMode);
                }
                catch
                {
                    // Best-effort restore.
                }
            }
#pragma warning restore CA1416
        }
        finally
        {
            Console.SetError(previousStderr);
        }

        Assert.That(stderr.ToString(), Does.Not.Contain($"[SYNC ERROR] Failed to process file '{filePath}'"),
            "When mtime is unchanged, Sync() must skip without reading the file (unreadable file should not produce sync error output).");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "FR-14.1: unchanged mtime should skip processing (mentions not restored).");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.Null,
            "FR-14.1: unchanged mtime should skip processing (mentions not restored).");
    }

    [Test]
    public void Sync_WhenMtimeBumpedButContentSame_UpdatesLastIndexedOnly_AndDoesNotRestoreMentionsOrEdges()
    {
        // T-SYNC-MTIME-001.1: RTM FR-14.2

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-mtime-a-{unique}";
        var conceptB = $"sync-mtime-b-{unique}";

        var (filePath, memoryUri) = CreateTestMarkdownFile(
            $"# Sync Mtime Optimization Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n");
        GraphTools.Sync();

        var before = GetFileContentRow(memoryUri);
        Assert.That(before, Is.Not.Null);

        TamperDb_DeleteMentions(memoryUri);

        var desiredMtime = File.GetLastWriteTimeUtc(filePath).AddMinutes(2);
        File.SetLastWriteTimeUtc(filePath, desiredMtime);
        var actualNewMtime = File.GetLastWriteTimeUtc(filePath);

        GraphTools.Sync();

        var after = GetFileContentRow(memoryUri);
        Assert.That(after, Is.Not.Null);

        Assert.That(after!.LastIndexed, Is.EqualTo(CultureInvariantHelpers.FormatDateTime(actualNewMtime, "O")),
            "FR-14.2: when content hash matches, Sync() must update last_indexed to the filesystem mtime.");
        Assert.That(after.FileMd5, Is.EqualTo(before!.FileMd5),
            "FR-14.2: when content hash matches, Sync() must not update file_md5.");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "FR-14.2: hash guard path should not restore mentions (no reprocessing).");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.Null,
            "FR-14.2: hash guard path should not restore mentions (no reprocessing).");
    }

    [Test]
    public void Sync_WhenContentChanged_ReprocessesAndRestoresMentionsAndEdges_IncludingNewConcept()
    {
        // T-SYNC-MTIME-001.1: RTM FR-14.3

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-mtime-a-{unique}";
        var conceptB = $"sync-mtime-b-{unique}";
        var conceptC = $"sync-mtime-c-{unique}";

        var (filePath, memoryUri) = CreateTestMarkdownFile(
            $"# Sync Mtime Optimization Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n");
        GraphTools.Sync();

        var before = GetFileContentRow(memoryUri);
        Assert.That(before, Is.Not.Null);

        TamperDb_DeleteMentions(memoryUri);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null);

        // Change content to force hash difference (mtime will bump naturally).
        var contentV2 = $"# Sync Mtime Optimization Test\n\nReferences [[{conceptA}]] and [[{conceptB}]] and now [[{conceptC}]].\n";
        File.WriteAllText(filePath, contentV2);
        var newMtime = File.GetLastWriteTimeUtc(filePath);

        GraphTools.Sync();

        var after = GetFileContentRow(memoryUri);
        Assert.That(after, Is.Not.Null);
        Assert.That(after!.LastIndexed, Is.EqualTo(CultureInvariantHelpers.FormatDateTime(newMtime, "O")));
        Assert.That(after.FileMd5, Is.Not.EqualTo(before!.FileMd5),
            "FR-14.3: content change must update file_md5.");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.EqualTo(1));
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.EqualTo(1));
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptC)), Is.EqualTo(1),
            "FR-14.3: processing must restore mentions and include the new concept.");
    }

    private static (string filePath, string memoryUri) CreateTestMarkdownFile(string content)
    {
        var testFolder = Path.Combine(Config.MemoryPath, "tests", "sync-mtime");
        Directory.CreateDirectory(testFolder);

        var filePath = Path.Combine(testFolder, $"note-{Guid.NewGuid():N}.md");
        File.WriteAllText(filePath, content);

        var memoryUri = MarkdownIO.PathToUri(filePath, Config.MemoryPath);
        return (filePath, memoryUri);
    }

    private sealed record FileContentRow(string FilePath, string? LastIndexed, string? FileMd5, long? FileSize);

    private static FileContentRow? GetFileContentRow(string memoryUri)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT last_indexed, file_md5, file_size FROM file_content WHERE file_path = @path";
        cmd.Parameters.AddWithValue("@path", memoryUri);

        using var reader = cmd.ExecuteReader();
        if (!reader.Read())
        {
            return null;
        }

        var lastIndexed = reader.IsDBNull(0) ? null : reader.GetString(0);
        var fileMd5 = reader.IsDBNull(1) ? null : reader.GetString(1);
        long? fileSize = reader.IsDBNull(2) ? null : reader.GetInt64(2);
        return new FileContentRow(memoryUri, lastIndexed, fileMd5, fileSize);
    }

    private static int? GetMentionCount(string memoryUri, string normalizedConcept)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT mention_count FROM concept_mentions WHERE concept_name = @concept AND source_file = @file";
        cmd.Parameters.AddWithValue("@concept", normalizedConcept);
        cmd.Parameters.AddWithValue("@file", memoryUri);

        var result = cmd.ExecuteScalar();
        if (result == null || result == DBNull.Value)
        {
            return null;
        }

        return Convert.ToInt32(result, CultureInfo.InvariantCulture);
    }

    private static void TamperDb_DeleteMentions(string memoryUri)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        conn.Execute("DELETE FROM concept_mentions WHERE source_file = @file", new { file = memoryUri });
    }
}
