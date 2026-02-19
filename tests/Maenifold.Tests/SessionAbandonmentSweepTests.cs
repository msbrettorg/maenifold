using System;
using System.Globalization;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// CLEANUP-001.1: RTM FR-14.6
// Goal: prove Sync-time abandoned-session sweep marks stale active sessions as abandoned,
// even when mtime/hash guards would normally skip reading unchanged files.
[TestFixture]
[NonParallelizable]
[Category("CLEANUP-001.1")]
public class SessionAbandonmentSweepTests
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
        _testRoot = Path.Combine(repoRoot, "test-outputs", "session-abandonment-sweep", $"run-{Guid.NewGuid():N}");
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
    public void CLEANUP_001_1_Sync_StaleWorkflowSession_MarkedAbandoned_EvenWhenMtimeGuardWouldSkipRead()
    {
        // CLEANUP-001.1: RTM FR-14.6

        var sessionId = $"workflow-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}-{RandomNumberGenerator.GetInt32(10000, 99999)}";
        var staleUtc = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));

        var (sessionPath, memoryUri) = CreateThinkingSessionFile("workflow", sessionId, modifiedUtc: staleUtc, status: "active");
        SeedIndexedRowSoMtimeGuardSkips(sessionPath, memoryUri, status: "active");

        // Precondition: Sync would normally skip reading this file (mtime == last_indexed).
        Assert.That(GetFileContentLastIndexed(memoryUri), Is.EqualTo(CultureInvariantHelpers.FormatDateTime(File.GetLastWriteTimeUtc(sessionPath), "O")));

        GraphTools.Sync();

        var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(sessionPath);
        Assert.That(frontmatter, Is.Not.Null);
        Assert.That(frontmatter!["status"], Is.EqualTo("abandoned"),
            "FR-14.6: stale workflow session must be marked abandoned during Sync even if mtime guard skips content read.");
        Assert.That(content, Does.Contain("Session Abandoned"),
            "FR-14.6: abandoned session should record a Session Abandoned section.");
    }

    [Test]
    public void CLEANUP_001_1_Sync_StaleSequentialSession_MarkedAbandoned_EvenWhenMtimeGuardWouldSkipRead()
    {
        // CLEANUP-001.1: RTM FR-14.6

        var sessionId = $"session-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}-{RandomNumberGenerator.GetInt32(10000, 99999)}";
        var staleUtc = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));

        var (sessionPath, memoryUri) = CreateThinkingSessionFile("sequential", sessionId, modifiedUtc: staleUtc, status: "active");
        SeedIndexedRowSoMtimeGuardSkips(sessionPath, memoryUri, status: "active");

        // Precondition: Sync would normally skip reading this file (mtime == last_indexed).
        Assert.That(GetFileContentLastIndexed(memoryUri), Is.EqualTo(CultureInvariantHelpers.FormatDateTime(File.GetLastWriteTimeUtc(sessionPath), "O")));

        GraphTools.Sync();

        var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(sessionPath);
        Assert.That(frontmatter, Is.Not.Null);
        Assert.That(frontmatter!["status"], Is.EqualTo("abandoned"),
            "FR-14.6: stale sequential session must be marked abandoned during Sync even if mtime guard skips content read.");
        Assert.That(content, Does.Contain("Session Abandoned"),
            "FR-14.6: abandoned session should record a Session Abandoned section.");
    }

    [Test]
    public void CLEANUP_001_1_Sync_DoesNotRewriteNormalMemoryFile_SolelyDueToAge_MtimeGuardRemains()
    {
        // CLEANUP-001.1: RTM FR-14.6

        // This test is a safety property: Sync-time abandoned-session sweep must not spill over
        // into non-session memory. Old notes should not be rewritten just because they are old.
        var filePath = CreateNormalMemoryFile(modifiedUtc: DateTime.UtcNow.AddDays(-7));
        var memoryUri = MarkdownIO.PathToUri(filePath, Config.MemoryPath);
        SeedIndexedRowSoMtimeGuardSkips(filePath, memoryUri, status: null);

        var beforeMtime = File.GetLastWriteTimeUtc(filePath);
        var beforeText = File.ReadAllText(filePath);
        var beforeChecksum = MarkdownIO.GenerateChecksum(beforeText);

        GraphTools.Sync();

        var afterMtime = File.GetLastWriteTimeUtc(filePath);
        var afterText = File.ReadAllText(filePath);
        var afterChecksum = MarkdownIO.GenerateChecksum(afterText);

        Assert.That(afterMtime, Is.EqualTo(beforeMtime),
            "FR-14.6: Sync-time session sweep must not rewrite normal memory files solely due to age.");
        Assert.That(afterChecksum, Is.EqualTo(beforeChecksum),
            "FR-14.6: Sync-time session sweep must not mutate normal memory file content.");
    }

    private static (string filePath, string memoryUri) CreateThinkingSessionFile(string thinkingType, string sessionId, DateTime modifiedUtc, string status)
    {
        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = $"Test {thinkingType} session {sessionId}",
            ["permalink"] = sessionId,
            ["type"] = thinkingType,
            ["status"] = status,
            ["modified"] = modifiedUtc.ToString("O", CultureInfo.InvariantCulture)
        };

        MarkdownIO.CreateSession(thinkingType, sessionId, frontmatter, "# Session\n\nTest content.");
        var path = MarkdownIO.GetSessionPath(thinkingType, sessionId);

        // Align file mtime with the 'modified' time so last_indexed can be set to this value.
        File.SetLastWriteTimeUtc(path, modifiedUtc);
        var normalizedMtime = File.GetLastWriteTimeUtc(path);
        if (normalizedMtime != modifiedUtc)
        {
            // Ensure deterministic equality with the stored filesystem value.
            File.SetLastWriteTimeUtc(path, normalizedMtime);
        }

        var memoryUri = MarkdownIO.PathToUri(path, Config.MemoryPath);
        return (path, memoryUri);
    }

    private static string CreateNormalMemoryFile(DateTime modifiedUtc)
    {
        var folder = Path.Combine(Config.MemoryPath, "notes", "cleanup-001-1");
        Directory.CreateDirectory(folder);
        var filePath = Path.Combine(folder, $"note-{Guid.NewGuid():N}.md");

        var content = "# Normal Note\n\nThis is a normal file, not a thinking session.\n";
        File.WriteAllText(filePath, content);

        File.SetLastWriteTimeUtc(filePath, modifiedUtc);
        return filePath;
    }

    private static void SeedIndexedRowSoMtimeGuardSkips(string filePath, string memoryUri, string? status)
    {
        var mtime = File.GetLastWriteTimeUtc(filePath);
        var lastIndexed = CultureInvariantHelpers.FormatDateTime(mtime, "O");
        var fileSize = new FileInfo(filePath).Length;
        var md5 = ComputeMd5Base64(filePath);
        var title = Path.GetFileNameWithoutExtension(filePath);

        var (_, content, _) = MarkdownIO.ReadMarkdown(filePath);

        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        conn.Execute(
            "INSERT OR REPLACE INTO file_content (file_path, title, content, last_indexed, status, file_md5, file_size) VALUES (@path, @title, @content, @indexed, @status, @md5, @size)",
            new { path = memoryUri, title, content, indexed = lastIndexed, status, md5, size = fileSize });
    }

    private static string? GetFileContentLastIndexed(string memoryUri)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT last_indexed FROM file_content WHERE file_path = @path";
        cmd.Parameters.AddWithValue("@path", memoryUri);

        var result = cmd.ExecuteScalar();
        if (result == null || result == DBNull.Value)
        {
            return null;
        }

        return Convert.ToString(result, CultureInfo.InvariantCulture);
    }

    private static string ComputeMd5Base64(string filePath)
    {
#pragma warning disable CA5351
        // MD5 is used here only to align with ConceptSync file_md5 change-detection semantics.
        // This is non-adversarial and not used for cryptographic integrity.
        using var stream = File.OpenRead(filePath);
        var hash = MD5.HashData(stream);
        return Convert.ToBase64String(hash);
#pragma warning restore CA5351
    }
}
