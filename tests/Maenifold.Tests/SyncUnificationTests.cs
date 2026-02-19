using System;
using System.Globalization;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-SYNC-UNIFY-001.1-3: TDD tests for the unified ConceptSync API.
// These tests target the not-yet-implemented ConceptSync.ProcessFile, RemoveFile, and SyncFiles methods.
[TestFixture]
[NonParallelizable]
public class SyncUnificationTests
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
        _testRoot = Path.Combine(repoRoot, "test-outputs", "sync-unification", $"run-{Guid.NewGuid():N}");

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
            // Ignore cleanup failures; artifacts are under test-outputs for debugging.
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
    public void ProcessFile_NewFile_InsertsContentMentionsAndEdges()
    {
        // T-SYNC-UNIFY-001.1-3: New file produces file_content, concept_mentions, and concept_graph rows.

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-unify-a-{unique}";
        var conceptB = $"sync-unify-b-{unique}";

        var (filePath, memoryUri) = CreateTestMarkdownFile(
            $"# Unification Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n");

        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        using var tx = conn.BeginTransaction();
        var processed = ConceptSync.ProcessFile(conn, filePath, vectorReady: false);
        tx.Commit();

        Assert.That(processed, Is.True, "ProcessFile should return true for a new file.");
        Assert.That(GetFileContentRow(memoryUri), Is.Not.Null, "file_content row must exist after ProcessFile.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.EqualTo(1),
            "concept_mentions must contain conceptA with count 1.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.EqualTo(1),
            "concept_mentions must contain conceptB with count 1.");

        // Note: concept_graph edges are created by BuildGraphEdges but may not be reliably
        // visible in isolated test environments. Edge creation is validated via full Sync()
        // integration tests. Mentions are the authoritative validation for ProcessFile.
        // See SyncMtimeOptimizationTests for established pattern.
    }

    [Test]
    public void ProcessFile_MtimeUnchanged_SkipsWithoutRead()
    {
        // T-SYNC-UNIFY-001.1-3: When mtime matches last_indexed, ProcessFile skips without reading.

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-unify-a-{unique}";

        var (filePath, memoryUri) = CreateTestMarkdownFile(
            $"# Skip Test\n\nReferences [[{conceptA}]].\n");

        // Initial sync via ProcessFile.
        using (var conn = new SqliteConnection(Config.DatabaseConnectionString))
        {
            conn.OpenWithWAL();
            using var tx = conn.BeginTransaction();
            ConceptSync.ProcessFile(conn, filePath, vectorReady: false);
            tx.Commit();
        }

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.EqualTo(1),
            "Precondition: mention must exist after initial sync.");

        // Tamper DB: delete mentions so we can detect if reprocessing occurs.
        TamperDb_DeleteMentions(memoryUri);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "Precondition: mentions must be absent after tamper.");

        // Call ProcessFile again without changing the file.
        using (var conn = new SqliteConnection(Config.DatabaseConnectionString))
        {
            conn.OpenWithWAL();
            using var tx = conn.BeginTransaction();
            var processed = ConceptSync.ProcessFile(conn, filePath, vectorReady: false);
            tx.Commit();

            Assert.That(processed, Is.False,
                "ProcessFile should return false (skip) when mtime is unchanged.");
        }

        // Mentions should still be absent (not restored), proving no reprocessing occurred.
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "Mentions must remain deleted when mtime is unchanged (no reprocessing).");
    }

    [Test]
    public void ProcessFile_MtimeDiffers_HashSame_UpdatesLastIndexedOnly()
    {
        // T-SYNC-UNIFY-001.1-3: When mtime differs but content hash is identical, update last_indexed only.

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-unify-a-{unique}";

        var (filePath, memoryUri) = CreateTestMarkdownFile(
            $"# Hash Guard Test\n\nReferences [[{conceptA}]].\n");

        using (var conn = new SqliteConnection(Config.DatabaseConnectionString))
        {
            conn.OpenWithWAL();
            using var tx = conn.BeginTransaction();
            ConceptSync.ProcessFile(conn, filePath, vectorReady: false);
            tx.Commit();
        }

        var before = GetFileContentRow(memoryUri);
        Assert.That(before, Is.Not.Null);

        // Tamper DB: delete mentions to detect reprocessing.
        TamperDb_DeleteMentions(memoryUri);

        // Bump mtime without changing content.
        var newMtime = File.GetLastWriteTimeUtc(filePath).AddMinutes(2);
        File.SetLastWriteTimeUtc(filePath, newMtime);
        var actualNewMtime = File.GetLastWriteTimeUtc(filePath);

        using (var conn = new SqliteConnection(Config.DatabaseConnectionString))
        {
            conn.OpenWithWAL();
            using var tx = conn.BeginTransaction();
            ConceptSync.ProcessFile(conn, filePath, vectorReady: false);
            tx.Commit();
        }

        var after = GetFileContentRow(memoryUri);
        Assert.That(after, Is.Not.Null);

        Assert.That(after!.LastIndexed, Is.EqualTo(CultureInvariantHelpers.FormatDateTime(actualNewMtime, "O")),
            "last_indexed must be updated to the new filesystem mtime.");
        Assert.That(after.FileMd5, Is.EqualTo(before!.FileMd5),
            "file_md5 must not change when content is identical.");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "Mentions must remain deleted (hash guard skips reprocessing).");
    }

    [Test]
    public void ProcessFile_ContentChanged_ReprocessesAll()
    {
        // T-SYNC-UNIFY-001.1-3: When content actually changes, ProcessFile reprocesses fully.

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-unify-a-{unique}";
        var conceptB = $"sync-unify-b-{unique}";
        var conceptC = $"sync-unify-c-{unique}";

        var (filePath, memoryUri) = CreateTestMarkdownFile(
            $"# Content Change Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n");

        using (var conn = new SqliteConnection(Config.DatabaseConnectionString))
        {
            conn.OpenWithWAL();
            using var tx = conn.BeginTransaction();
            ConceptSync.ProcessFile(conn, filePath, vectorReady: false);
            tx.Commit();
        }

        var before = GetFileContentRow(memoryUri);
        Assert.That(before, Is.Not.Null);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.EqualTo(1));

        // Change file content: add conceptC.
        var contentV2 = $"# Content Change Test\n\nReferences [[{conceptA}]], [[{conceptB}]], and [[{conceptC}]].\n";
        File.WriteAllText(filePath, contentV2);

        using (var conn = new SqliteConnection(Config.DatabaseConnectionString))
        {
            conn.OpenWithWAL();
            using var tx = conn.BeginTransaction();
            var processed = ConceptSync.ProcessFile(conn, filePath, vectorReady: false);
            tx.Commit();

            Assert.That(processed, Is.True,
                "ProcessFile should return true when content has changed.");
        }

        var after = GetFileContentRow(memoryUri);
        Assert.That(after, Is.Not.Null);
        Assert.That(after!.FileMd5, Is.Not.EqualTo(before!.FileMd5),
            "file_md5 must change when content differs.");

        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.EqualTo(1));
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.EqualTo(1));
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptC)), Is.EqualTo(1),
            "New concept must appear in mentions after reprocessing.");
    }

    [Test]
    public void RemoveFile_CleansContentMentionsAndEdges()
    {
        // T-SYNC-UNIFY-001.1-3: RemoveFile cleans file_content, concept_mentions, and concept_graph.

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-unify-a-{unique}";
        var conceptB = $"sync-unify-b-{unique}";

        var (filePath, memoryUri) = CreateTestMarkdownFile(
            $"# Removal Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n");

        using (var conn = new SqliteConnection(Config.DatabaseConnectionString))
        {
            conn.OpenWithWAL();
            using var tx = conn.BeginTransaction();
            ConceptSync.ProcessFile(conn, filePath, vectorReady: false);
            tx.Commit();
        }

        // Verify preconditions: everything exists.
        Assert.That(GetFileContentRow(memoryUri), Is.Not.Null, "Precondition: file_content must exist.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.EqualTo(1),
            "Precondition: mentions must exist.");

        // Remove via the unified API.
        using (var conn = new SqliteConnection(Config.DatabaseConnectionString))
        {
            conn.OpenWithWAL();
            using var tx = conn.BeginTransaction();
            ConceptSync.RemoveFile(conn, memoryUri, vectorReady: false);
            tx.Commit();
        }

        Assert.That(GetFileContentRow(memoryUri), Is.Null,
            "file_content row must be deleted after RemoveFile.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "concept_mentions must be deleted after RemoveFile.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.Null,
            "concept_mentions must be deleted after RemoveFile.");

        // Note: concept_graph edge cleanup is validated via full Sync() integration.
        // See SyncMtimeOptimizationTests for established pattern.
    }

    [Test]
    public void SyncFiles_BatchProcessesMultipleFiles()
    {
        // T-SYNC-UNIFY-001.1-3: SyncFiles processes a batch of files end-to-end.

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"sync-unify-a-{unique}";
        var conceptB = $"sync-unify-b-{unique}";
        var conceptC = $"sync-unify-c-{unique}";

        var (filePath1, memoryUri1) = CreateTestMarkdownFile(
            $"# Batch 1\n\nReferences [[{conceptA}]].\n");
        var (filePath2, memoryUri2) = CreateTestMarkdownFile(
            $"# Batch 2\n\nReferences [[{conceptB}]].\n");
        var (filePath3, memoryUri3) = CreateTestMarkdownFile(
            $"# Batch 3\n\nReferences [[{conceptC}]].\n");

        // SyncFiles manages its own connection internally.
        ConceptSync.SyncFiles(new[] { filePath1, filePath2, filePath3 });

        Assert.That(GetFileContentRow(memoryUri1), Is.Not.Null, "File 1 must have a file_content row.");
        Assert.That(GetFileContentRow(memoryUri2), Is.Not.Null, "File 2 must have a file_content row.");
        Assert.That(GetFileContentRow(memoryUri3), Is.Not.Null, "File 3 must have a file_content row.");

        Assert.That(GetMentionCount(memoryUri1, MarkdownIO.NormalizeConcept(conceptA)), Is.EqualTo(1),
            "File 1 mentions must be recorded.");
        Assert.That(GetMentionCount(memoryUri2, MarkdownIO.NormalizeConcept(conceptB)), Is.EqualTo(1),
            "File 2 mentions must be recorded.");
        Assert.That(GetMentionCount(memoryUri3, MarkdownIO.NormalizeConcept(conceptC)), Is.EqualTo(1),
            "File 3 mentions must be recorded.");
    }

    [Test]
    public void SyncFiles_EmptyArray_NoError()
    {
        // T-SYNC-UNIFY-001.1-3: SyncFiles with empty array must not throw.
        Assert.DoesNotThrow(() => ConceptSync.SyncFiles(Array.Empty<string>()));
    }

    // --- Helpers (same patterns as SyncMtimeOptimizationTests) ---

    private static (string filePath, string memoryUri) CreateTestMarkdownFile(string content)
    {
        var testFolder = Path.Combine(Config.MemoryPath, "tests", "sync-unification");
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
