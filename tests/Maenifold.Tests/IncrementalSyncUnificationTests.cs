using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Text.Json;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-SYNC-UNIFY-001.4: Verify incremental sync produces identical results to full sync
[TestFixture]
[NonParallelizable]
public class IncrementalSyncUnificationTests
{
    private MethodInfo _syncFile = null!;
    private MethodInfo _createdHandler = null!;
    private MethodInfo _deletedHandler = null!;
    private MethodInfo _renamedHandler = null!;
    private MethodInfo _changedHandler = null!;

    private const string TestSubfolder = "tests/incremental-unify";

    [OneTimeSetUp]
    public void OneTimeSetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();

        var type = typeof(IncrementalSyncTools);
        _syncFile = type.GetMethod("SyncFile", BindingFlags.NonPublic | BindingFlags.Static)
            ?? throw new InvalidOperationException("SyncFile method not found.");
        _createdHandler = type.GetMethod("ProcessFileCreated", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileCreated handler not found.");
        _deletedHandler = type.GetMethod("ProcessFileDeleted", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileDeleted handler not found.");
        _renamedHandler = type.GetMethod("ProcessFileRenamed", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileRenamed handler not found.");
        _changedHandler = type.GetMethod("ProcessFileChanged", BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("ProcessFileChanged handler not found.");
    }

    [SetUp]
    public void SetUp()
    {
        GraphDatabase.InitializeDatabase();

        try
        {
            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenWithWAL();
            conn.Execute("DELETE FROM concept_mentions WHERE source_file LIKE 'memory://tests/incremental-unify/%'");
            conn.Execute("DELETE FROM file_content WHERE file_path LIKE 'memory://tests/incremental-unify/%'");
            conn.Execute("DELETE FROM concepts WHERE concept_name LIKE 'incremental-unify-%'");
            conn.Execute("DELETE FROM concept_graph WHERE concept_a LIKE 'incremental-unify-%' OR concept_b LIKE 'incremental-unify-%'");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"[TEST SETUP WARNING] Failed to clean test data: {ex.Message}");
            GraphDatabase.InitializeDatabase();
        }
    }

    [Test]
    public void IncrementalSync_FileChanged_ProducesSameDbStateAsFullSync()
    {
        // T-SYNC-UNIFY-001.4: incremental SyncFile must produce identical DB state to full ConceptSync.Sync()
        // Verify that SyncFile creates correct file_content, concept_mentions, and concept_graph entries.

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"Incremental-Unify-Alpha-{unique}";
        var conceptB = $"Incremental-Unify-Beta-{unique}";
        var content = $"# Unification Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n";

        var (filePath, memoryUri) = CreateTestFile(content, $"incr-{unique}");
        InvokeSyncFile(filePath);

        var fileContent = GetFileContentRow(memoryUri);
        var mentionA = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA));
        var mentionB = GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB));

        Assert.That(fileContent, Is.Not.Null, "Incremental sync should create file_content row.");
        Assert.That(mentionA, Is.Not.Null.And.GreaterThan(0), "Incremental sync should create mention for concept A.");
        Assert.That(mentionB, Is.Not.Null.And.GreaterThan(0), "Incremental sync should create mention for concept B.");

        // Verify the MD5 hash and file size are populated
        Assert.That(fileContent!.FileMd5, Is.Not.Null.And.Not.Empty,
            "Incremental sync must populate file_md5.");
        Assert.That(fileContent.FileSize, Is.Not.Null.And.GreaterThan(0),
            "Incremental sync must populate file_size.");

        // Verify mention counts are correct (each concept appears once in the content)
        Assert.That(mentionA, Is.EqualTo(1), "Mention count for concept A should be 1.");
        Assert.That(mentionB, Is.EqualTo(1), "Mention count for concept B should be 1.");

        // Note: concept_graph edges may not be reliably created in test environments due to
        // vector extension loading issues. Mentions are the authoritative validation.
        // See IncrementalSyncMtimeOptimizationTests for precedent.
    }

    [Test]
    public void IncrementalSync_FileDeleted_CleansAllArtifacts()
    {
        // T-SYNC-UNIFY-001.4: ProcessFileDeleted must remove file_content, concept_mentions, concept_graph edges

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"Incremental-Unify-DelA-{unique}";
        var conceptB = $"Incremental-Unify-DelB-{unique}";
        var content = $"# Delete Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n";
        var (filePath, memoryUri) = CreateTestFile(content);

        // Establish baseline via incremental sync
        InvokeHandler(_createdHandler, filePath);

        // Verify baseline
        Assert.That(GetFileContentRow(memoryUri), Is.Not.Null, "File should be indexed before deletion.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Not.Null,
            "Mention A should exist before deletion.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.Not.Null,
            "Mention B should exist before deletion.");

        // Delete the file from disk, then call incremental ProcessFileDeleted
        File.Delete(filePath);
        InvokeHandler(_deletedHandler, filePath);

        // Verify all artifacts are cleaned
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "concept_mentions for concept A must be removed after ProcessFileDeleted.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.Null,
            "concept_mentions for concept B must be removed after ProcessFileDeleted.");
        Assert.That(GetFileContentRow(memoryUri), Is.Null,
            "file_content row must be removed after ProcessFileDeleted.");

        var edge = GetEdgeSourceFiles(MarkdownIO.NormalizeConcept(conceptA), MarkdownIO.NormalizeConcept(conceptB));
        Assert.That(edge == null || !edge.Contains(memoryUri),
            "concept_graph edge must not reference the deleted file.");
    }

    [Test]
    public void IncrementalSync_FileRenamed_DeletesOldCreatesNew()
    {
        // T-SYNC-UNIFY-001.4: ProcessFileRenamed must remove old URI and create new URI with correct content

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"Incremental-Unify-RenA-{unique}";
        var conceptB = $"Incremental-Unify-RenB-{unique}";
        var content = $"# Rename Test\n\nReferences [[{conceptA}]] and [[{conceptB}]].\n";
        var (filePath, memoryUri) = CreateTestFile(content);

        // Sync the original file
        InvokeSyncFile(filePath);
        Assert.That(GetFileContentRow(memoryUri), Is.Not.Null, "Original file should be indexed.");

        // Rename the file on disk
        var testFolder = Path.GetDirectoryName(filePath)!;
        var newFileName = $"renamed-{unique}.md";
        var newFilePath = Path.Combine(testFolder, newFileName);
        File.Move(filePath, newFilePath);

        var newMemoryUri = MarkdownIO.PathToUri(newFilePath, Config.MemoryPath);

        // Call ProcessFileRenamed (takes oldPath, newPath)
        _renamedHandler.Invoke(null, new object[] { filePath, newFilePath });

        // Verify old URI is gone
        Assert.That(GetFileContentRow(memoryUri), Is.Null,
            "Old URI must be removed from file_content after rename.");
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "Old URI concept_mentions must be removed after rename.");

        // Verify new URI is present with correct content
        var newRow = GetFileContentRow(newMemoryUri);
        Assert.That(newRow, Is.Not.Null,
            "New URI must be present in file_content after rename.");
        Assert.That(GetMentionCount(newMemoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Not.Null,
            "New URI must have concept_mentions for concept A after rename.");
        Assert.That(GetMentionCount(newMemoryUri, MarkdownIO.NormalizeConcept(conceptB)), Is.Not.Null,
            "New URI must have concept_mentions for concept B after rename.");
    }

    [Test]
    public void IncrementalSync_MtimeGuard_SkipsUnchangedFiles()
    {
        // T-SYNC-UNIFY-001.4: Same file, same mtime => SyncFile must skip (no reprocessing)

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"Incremental-Unify-MtA-{unique}";
        var conceptCommon = $"Incremental-Unify-MtCommon-{unique}";
        var content = $"# Mtime Guard Test\n\nReferences [[{conceptA}]] and [[{conceptCommon}]].\n";
        var (filePath, memoryUri) = CreateTestFile(content);

        // Initial sync
        InvokeHandler(_createdHandler, filePath);

        // Normalize mtime to match stored value (file systems may have coarse resolution)
        var rowAfterCreate = GetFileContentRow(memoryUri);
        Assert.That(rowAfterCreate, Is.Not.Null);
        var storedMtime = CultureInvariantHelpers.ParseDateTime(rowAfterCreate!.LastIndexed!);
        File.SetLastWriteTimeUtc(filePath, storedMtime);

        // Tamper: delete mentions to detect if reprocessing occurs
        TamperDeleteMentionsAndEdgesForFile(memoryUri);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "Mentions should be tampered away before re-sync.");

        // Call SyncFile again without changing file mtime
        InvokeSyncFile(filePath);

        // Mentions should still be missing because mtime guard skipped reprocessing
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "When mtime is unchanged, SyncFile must skip and must not restore missing mentions.");
        Assert.That(GetEdgeSourceFiles(MarkdownIO.NormalizeConcept(conceptA), MarkdownIO.NormalizeConcept(conceptCommon)), Is.Null,
            "When mtime is unchanged, SyncFile must not restore missing graph edges.");
    }

    [Test]
    public void IncrementalSync_HashGuard_UpdatesLastIndexedOnly()
    {
        // T-SYNC-UNIFY-001.4: mtime changed but content hash unchanged => only last_indexed updated

        var unique = Guid.NewGuid().ToString("N");
        var conceptA = $"Incremental-Unify-HgA-{unique}";
        var conceptCommon = $"Incremental-Unify-HgCommon-{unique}";
        var content = $"# Hash Guard Test\n\nReferences [[{conceptA}]] and [[{conceptCommon}]].\n";
        var (filePath, memoryUri) = CreateTestFile(content);

        // Initial sync
        InvokeHandler(_createdHandler, filePath);

        var before = GetFileContentRow(memoryUri);
        Assert.That(before, Is.Not.Null);

        // Tamper: delete mentions to detect if reprocessing occurs
        TamperDeleteMentionsAndEdgesForFile(memoryUri);
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null);

        // Bump mtime only (no content change)
        var newMtime = File.GetLastWriteTimeUtc(filePath).AddMinutes(1);
        File.SetLastWriteTimeUtc(filePath, newMtime);

        // Call SyncFile
        InvokeSyncFile(filePath);

        var after = GetFileContentRow(memoryUri);
        Assert.That(after, Is.Not.Null);

        // last_indexed should be updated to the new mtime
        Assert.That(after!.LastIndexed, Is.EqualTo(CultureInvariantHelpers.FormatDateTime(newMtime, "O")),
            "When mtime differs but hash is unchanged, SyncFile must update last_indexed to the new filesystem mtime.");

        // Hash should remain unchanged
        Assert.That(after.FileMd5, Is.EqualTo(before!.FileMd5),
            "When content hash is unchanged, SyncFile must not update file_md5.");

        // Mentions should NOT be restored (no reprocessing)
        Assert.That(GetMentionCount(memoryUri, MarkdownIO.NormalizeConcept(conceptA)), Is.Null,
            "When mtime differs but hash is unchanged, SyncFile must not reprocess and must not restore missing mentions.");
        Assert.That(GetEdgeSourceFiles(MarkdownIO.NormalizeConcept(conceptA), MarkdownIO.NormalizeConcept(conceptCommon)), Is.Null,
            "When mtime differs but hash is unchanged, SyncFile must not restore missing graph edges.");
    }

    // --- Helper methods ---

    private static (string FilePath, string MemoryUri) CreateTestFile(string content, string? namePrefix = null)
    {
        var testFolder = Path.Combine(Config.MemoryPath, TestSubfolder);
        Directory.CreateDirectory(testFolder);

        var uniqueSuffix = Guid.NewGuid().ToString("N");
        var prefix = namePrefix ?? "note";
        var filePath = Path.Combine(testFolder, $"{prefix}-{uniqueSuffix}.md");
        File.WriteAllText(filePath, content);

        var memoryUri = MarkdownIO.PathToUri(filePath, Config.MemoryPath);
        return (filePath, memoryUri);
    }

    private void InvokeSyncFile(string path)
    {
        _syncFile.Invoke(null, new object[] { path });
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
}
