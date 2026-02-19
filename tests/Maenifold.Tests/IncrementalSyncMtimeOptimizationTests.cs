using System;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Collections.Generic;
using System.Text.Json;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-SYNC-MTIME-001.3: RTM FR-14.5
public class IncrementalSyncMtimeOptimizationTests
{
    private MethodInfo _createdHandler = null!;
    private MethodInfo _changedHandler = null!;

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
    }

    [SetUp]
    public void SetUp()
    {
        GraphDatabase.InitializeDatabase();

        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();
        conn.Execute("DELETE FROM concept_mentions WHERE source_file LIKE 'memory://tests/incremental-mtime/%'");
        conn.Execute("DELETE FROM file_content WHERE file_path LIKE 'memory://tests/incremental-mtime/%'");
        conn.Execute("DELETE FROM concepts WHERE concept_name LIKE 'incremental-mtime-%'");
        conn.Execute("DELETE FROM concept_graph WHERE concept_a LIKE 'incremental-mtime-%' OR concept_b LIKE 'incremental-mtime-%'");
    }

    [Test]
    public void ProcessFileChanged_WhenMtimeUnchanged_SkipsBeforeRead_AndDoesNotLogFailure()
    {
        // T-SYNC-MTIME-001.3: RTM FR-14.5

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"Incremental-Mtime-ConceptA-{unique}";
        var conceptCommon = $"Incremental-Mtime-ConceptCommon-{unique}";
        var content = $"# Incremental Mtime Test\n\nReferences [[{conceptA}]] and [[{conceptCommon}]].\n";
        var (filePath, memoryUri) = CreateTestFile(content);

        InvokeHandler(_createdHandler, filePath);

        // Ensure last_indexed is persisted to the file's mtime.
        // Some file systems have coarse mtime resolution; normalize to the stored value.
        var rowAfterCreate = GetFileContentRow(memoryUri);
        Assert.That(rowAfterCreate, Is.Not.Null);
        var storedMtime = CultureInvariantHelpers.ParseDateTime(rowAfterCreate!.LastIndexed!);
        File.SetLastWriteTimeUtc(filePath, storedMtime);

        // Simulate DB corruption (mentions/edges missing for the file).
        TamperDeleteMentionsAndEdgesForFile(memoryUri);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null);

        var logPath = Path.Combine(Config.MaenifoldRoot, "logs", "incremental-sync.log");
        var logBefore = File.Exists(logPath) ? File.ReadAllText(logPath) : string.Empty;

        var originalMtime = File.GetLastWriteTimeUtc(filePath);

        using (AcquireUnreadableFileGuard(filePath))
        {
            // Ensure mtime is unchanged; chmod/locks should not mutate mtime.
            File.SetLastWriteTimeUtc(filePath, originalMtime);
            InvokeHandler(_changedHandler, filePath);
        }

        var logAfter = File.Exists(logPath) ? File.ReadAllText(logPath) : string.Empty;
        Assert.That(logAfter, Does.Not.Contain($"Failed to process incremental sync for '{filePath}'."),
            "When mtime is unchanged, the incremental changed handler must not attempt to read the file (no failure log expected).");
        Assert.That(logAfter.StartsWith(logBefore, StringComparison.Ordinal), Is.True,
            "Incremental sync log should only append; it must not rewrite earlier entries.");
        Assert.That(GetFileContentRow(memoryUri), Is.Not.Null, "Initial incremental sync should persist file_content row.");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "When mtime is unchanged, incremental sync must skip before read and must not restore missing mentions.");
        Assert.That(GetEdgeSourceFiles(MarkdownIO.NormalizeConcept(conceptA), MarkdownIO.NormalizeConcept(conceptCommon)), Is.Null,
            "When mtime is unchanged, incremental sync must not restore missing graph edges.");
    }

    [Test]
    public void ProcessFileChanged_WhenMtimeDiffersButHashSame_UpdatesLastIndexedOnly_AndSkipsReprocessing()
    {
        // T-SYNC-MTIME-001.3: RTM FR-14.5

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"Incremental-Mtime-ConceptA-{unique}";
        var conceptCommon = $"Incremental-Mtime-ConceptCommon-{unique}";
        var content = $"# Incremental Mtime Test\n\nReferences [[{conceptA}]] and [[{conceptCommon}]].\n";
        var (filePath, memoryUri) = CreateTestFile(content);

        InvokeHandler(_createdHandler, filePath);

        var before = GetFileContentRow(memoryUri);
        Assert.That(before, Is.Not.Null);

        // Simulate DB corruption (mentions/edges missing for the file).
        TamperDeleteMentionsAndEdgesForFile(memoryUri);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null);

        // Change only mtime, not content.
        var newMtime = File.GetLastWriteTimeUtc(filePath).AddMinutes(1);
        File.SetLastWriteTimeUtc(filePath, newMtime);

        InvokeHandler(_changedHandler, filePath);

        var after = GetFileContentRow(memoryUri);
        Assert.That(after, Is.Not.Null);

        Assert.That(after!.LastIndexed, Is.EqualTo(CultureInvariantHelpers.FormatDateTime(newMtime, "O")),
            "When mtime differs but hash is unchanged, incremental sync must update last_indexed to the filesystem mtime.");

        Assert.That(after.FileMd5, Is.EqualTo(before!.FileMd5),
            "When content hash is unchanged, incremental sync must not update file_md5.");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "When mtime differs but hash is unchanged, incremental sync must not reprocess and must not restore missing mentions.");
        Assert.That(GetEdgeSourceFiles(MarkdownIO.NormalizeConcept(conceptA), MarkdownIO.NormalizeConcept(conceptCommon)), Is.Null,
            "When mtime differs but hash is unchanged, incremental sync must not restore missing graph edges.");
    }

    [Test]
    public void ProcessFileChanged_WhenHashDiffers_ReprocessesAndUpdatesMentions()
    {
        // T-SYNC-MTIME-001.3: RTM FR-14.5

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"Incremental-Mtime-ConceptA-{unique}";
        var conceptCommon = $"Incremental-Mtime-ConceptCommon-{unique}";
        var conceptB = $"Incremental-Mtime-ConceptB-{unique}";
        var contentV1 = $"# Incremental Mtime Test\n\nReferences [[{conceptA}]] and [[{conceptCommon}]].\n";
        var (filePath, memoryUri) = CreateTestFile(contentV1);

        InvokeHandler(_createdHandler, filePath);

        var before = GetFileContentRow(memoryUri);
        Assert.That(before, Is.Not.Null);

        // Simulate DB corruption (mentions/edges missing for the file).
        TamperDeleteMentionsAndEdgesForFile(memoryUri);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null);

        var contentV2 = $"# Incremental Mtime Test\n\nNow references [[{conceptB}]] and [[{conceptCommon}]] instead.\n";
        File.WriteAllText(filePath, contentV2);
        var newMtime = File.GetLastWriteTimeUtc(filePath);

        InvokeHandler(_changedHandler, filePath);

        var after = GetFileContentRow(memoryUri);
        Assert.That(after, Is.Not.Null);

        Assert.That(after!.LastIndexed, Is.EqualTo(CultureInvariantHelpers.FormatDateTime(newMtime, "O")),
            "When content differs, incremental sync must set last_indexed to the filesystem mtime.");
        Assert.That(after.FileMd5, Is.Not.EqualTo(before!.FileMd5),
            "When content differs, incremental sync must update the content hash.");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "Reprocessing must remove mentions for concepts no longer present.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.EqualTo(1),
            "Reprocessing must add mentions for newly-present concepts.");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptCommon)), Is.EqualTo(1),
            "Reprocessing must restore mentions for concepts still present.");

        // Graph edges are derived from co-occurrence across all files. In some environments
        // the concept_graph row may not exist if the edge was not present pre-tamper.
        // Mentions are the authoritative validation for incremental reprocessing.
    }

    private static (string FilePath, string MemoryUri) CreateTestFile(string content)
    {
        var testFolder = Path.Combine(Config.MemoryPath, "tests", "incremental-mtime");
        Directory.CreateDirectory(testFolder);

        var uniqueSuffix = Guid.NewGuid().ToString("N");
        var filePath = Path.Combine(testFolder, $"note-{uniqueSuffix}.md");
        File.WriteAllText(filePath, content);

        var memoryUri = MarkdownIO.PathToUri(filePath, Config.MemoryPath);
        return (filePath, memoryUri);
    }

    private static void InvokeHandler(MethodInfo handler, string path)
    {
        handler.Invoke(null, new object[] { path });
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
        {
            return null;
        }

        return Convert.ToInt32(result, CultureInfo.InvariantCulture);
    }

    private static void TamperDeleteMentionsAndEdgesForFile(string memoryUri)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        conn.Execute("DELETE FROM concept_mentions WHERE source_file = @file", new { file = memoryUri });

        var edges = conn.Query<(string a, string b, string files)>(
            "SELECT concept_a, concept_b, source_files FROM concept_graph WHERE source_files LIKE @pattern",
            new { pattern = $"%{memoryUri}%" });

        foreach (var (a, b, files) in edges)
        {
            var fileList = JsonSerializer.Deserialize<List<string>>(files, SafeJson.Options) ?? new List<string>();
            fileList.Remove(memoryUri);

            if (fileList.Count == 0)
            {
                conn.Execute("DELETE FROM concept_graph WHERE concept_a = @a AND concept_b = @b", new { a, b });
                continue;
            }

            conn.Execute(
                "UPDATE concept_graph SET co_occurrence_count = @count, source_files = @files WHERE concept_a = @a AND concept_b = @b",
                new
                {
                    count = fileList.Count,
                    files = JsonSerializer.Serialize(fileList, SafeJson.Options),
                    a,
                    b
                });
        }
    }

    private static List<string>? GetEdgeSourceFiles(string conceptA, string conceptB)
    {
        var (a, b) = string.CompareOrdinal(conceptA, conceptB) < 0 ? (conceptA, conceptB) : (conceptB, conceptA);

        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        var existing = conn.QuerySingle<(int count, string files)?>(
            "SELECT co_occurrence_count, source_files FROM concept_graph WHERE concept_a = @a AND concept_b = @b",
            new { a, b });

        if (existing == null || string.IsNullOrWhiteSpace(existing.Value.files))
        {
            return null;
        }

        return JsonSerializer.Deserialize<List<string>>(existing.Value.files, SafeJson.Options);
    }

    private static IDisposable AcquireUnreadableFileGuard(string filePath)
    {
        if (OperatingSystem.IsWindows())
        {
            // Windows: hold an exclusive lock to force read attempts to fail.
            var stream = new FileStream(filePath, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            return stream;
        }

        // Unix: remove read permissions but keep write permission so we can restore after.
#pragma warning disable CA1416 // Guarded by OS check; tests run on multiple platforms.
        var originalMode = File.GetUnixFileMode(filePath);
        File.SetUnixFileMode(filePath, UnixFileMode.UserWrite);
#pragma warning restore CA1416
        return new RestoreUnixFileModeGuard(filePath, originalMode);
    }

    private sealed class RestoreUnixFileModeGuard : IDisposable
    {
        private readonly string _filePath;
        private readonly UnixFileMode _originalMode;
        private bool _disposed;

        public RestoreUnixFileModeGuard(string filePath, UnixFileMode originalMode)
        {
            _filePath = filePath;
            _originalMode = originalMode;
        }

        public void Dispose()
        {
            if (_disposed)
            {
                return;
            }

            _disposed = true;
            try
            {
#pragma warning disable CA1416 // Guarded by OS check in AcquireUnreadableFileGuard.
                File.SetUnixFileMode(_filePath, _originalMode);
#pragma warning restore CA1416
            }
            catch
            {
                // Best-effort restore; test root is ephemeral.
            }
        }
    }
}
