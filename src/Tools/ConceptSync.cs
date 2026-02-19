using System.Diagnostics;
using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;

namespace Maenifold.Tools;

// CA1812: EdgeData is instantiated via JSON/Dapper deserialization, not direct construction
#pragma warning disable CA1812
internal sealed class EdgeData
#pragma warning restore CA1812
{
    public long co_occurrence_count { get; set; }
    public string source_files { get; set; } = "";
}

public static class ConceptSync
{
    private static string MemoryPath => Config.MemoryPath;

    // CLEANUP-001.1: RTM FR-14.6 - Identify thinking session files in DB without reading all files.
    private static bool IsThinkingSessionUri(string memoryUri)
    {
        if (string.IsNullOrWhiteSpace(memoryUri))
        {
            return false;
        }

        var normalized = memoryUri.Replace('\\', '/');
        // NOTE: workflow sessions are stored under thinking/workflow/ (singular) in current path scheme.
        return normalized.ContainsOrdinal("memory://thinking/sequential/") ||
               normalized.ContainsOrdinal("memory://thinking/workflow/");
    }

    // CLEANUP-001.1: RTM FR-14.6 - Sweep abandoned sessions based on DB metadata.
    // Reads ONLY candidate files (status=active, thinking session paths, last_indexed older than threshold).
    private static int SweepAbandonedThinkingSessions(SqliteConnection conn, DateTime nowUtc)
    {
        if (!Config.EnableSessionCleanup)
        {
            return 0;
        }

        var cutoffUtc = nowUtc.Subtract(TimeSpan.FromMinutes(Config.SessionAbandonmentMinutes));
        var cutoff = CultureInvariantHelpers.FormatDateTime(cutoffUtc, "O");

        // Candidate selection is DB-only: no filesystem reads.
        var candidates = conn.Query<(string file_path, string? last_indexed)>(@"
            SELECT file_path, last_indexed
            FROM file_content
            WHERE status = 'active'
              AND last_indexed IS NOT NULL
              AND last_indexed < @cutoff",
            new { cutoff }).ToList();

        var updated = 0;
        foreach (var candidate in candidates)
        {
            if (!IsThinkingSessionUri(candidate.file_path))
            {
                continue;
            }

            if (!CultureInvariantHelpers.TryParseDateTime(candidate.last_indexed!, out var lastIndexedUtc))
            {
                continue;
            }

            var diskPath = UriToPath(candidate.file_path);
            if (!File.Exists(diskPath))
            {
                continue;
            }

            // Read only candidates, then verify frontmatter is still active.
            var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(diskPath);
            if (frontmatter == null)
            {
                continue;
            }

            var status = frontmatter.TryGetValue("status", out var statusValue) ? statusValue?.ToString() : null;
            if (!string.Equals(status, "active", StringComparison.Ordinal))
            {
                // DB may be stale; reconcile DB to file frontmatter status without rewriting the file.
                if (!string.IsNullOrWhiteSpace(status) && !string.Equals(status, "active", StringComparison.Ordinal))
                {
                    conn.Execute("UPDATE file_content SET status = @s WHERE file_path = @p", new { s = status, p = candidate.file_path });
                }
                continue;
            }

            if (SessionCleanup.TryMarkAbandonedFromLastIndexed(frontmatter, content, lastIndexedUtc, nowUtc, out var updatedContent))
            {
                // Persist updated frontmatter/content and keep DB status in sync.
                MarkdownIO.WriteMarkdown(diskPath, frontmatter, updatedContent);
                conn.Execute("UPDATE file_content SET status = 'abandoned' WHERE file_path = @p", new { p = candidate.file_path });
                updated++;
            }
        }

        return updated;
    }

    /// <summary>
    /// Helper to execute INSERT OR IGNORE/REPLACE SQL with consistent error handling.
    /// Consolidates duplicate SQL execution patterns (SIMP-004).
    /// </summary>
    private static void ExecuteInsert(SqliteConnection conn, string sql, object parameters)
    {
        conn.Execute(sql, parameters);
    }

    /// <summary>
    /// Process a single file: extract concepts, update file_content, concept_mentions, concept_graph, and embeddings.
    /// Returns true if the file was fully reprocessed, false if skipped (mtime unchanged or hash matched).
    /// </summary>
    internal static bool ProcessFile(SqliteConnection conn, string filePath, bool vectorReady)
    {
        var memoryUri = PathToUri(filePath);
        var timestamp = CultureInvariantHelpers.FormatDateTime(File.GetLastWriteTimeUtc(filePath), "O");

        // T-SYNC-MTIME-001.1: RTM FR-14.1 - Skip unchanged mtime WITHOUT reading the file
        // Read-only pre-check to avoid exceptions on unreadable files when no processing is required.
        (string? lastIndexed, string? fileMd5, long? fileSize)? existing = null;
        using (var cmdExisting = conn.CreateCommand())
        {
            cmdExisting.CommandText = "SELECT last_indexed, file_md5, file_size FROM file_content WHERE file_path = @path";
            cmdExisting.Parameters.AddWithValue("@path", memoryUri);

            using var reader = cmdExisting.ExecuteReader();
            if (reader.Read())
            {
                var lastIndexed = reader.IsDBNull(0) ? null : reader.GetString(0);
                var fileMd5 = reader.IsDBNull(1) ? null : reader.GetString(1);

                long? existingFileSize = null;
                if (!reader.IsDBNull(2))
                {
                    existingFileSize = reader.GetInt64(2);
                }

                existing = (lastIndexed, fileMd5, existingFileSize);
            }
        }

        if (existing.HasValue && string.Equals(timestamp, existing.Value.lastIndexed, StringComparison.Ordinal))
        {
            return false;
        }

        // Only read markdown when mtime differs (or file not yet indexed).
        var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(filePath);

        // Session cleanup with mtime re-read: if session cleanup mutates the file, mtime may change.
        if (Config.EnableSessionCleanup && frontmatter != null)
        {
            SessionCleanup.HandleSessionCleanup(frontmatter!, filePath, content);
            // Re-read content and mtime after session cleanup may have modified the file
            (frontmatter, content, _) = MarkdownIO.ReadMarkdown(filePath);
            timestamp = CultureInvariantHelpers.FormatDateTime(File.GetLastWriteTimeUtc(filePath), "O");
        }

        var concepts = MarkdownIO.ExtractWikiLinks(content).ToArray();

        var title = GetTitle(filePath, frontmatter!);
        var fileStatus = frontmatter?.ContainsKey("status") == true ? frontmatter["status"]?.ToString() : null;

        // T-SYNC-MTIME-001.2: RTM FR-14.4 - When mtime differs, use size guard before hashing
        var fileSize = new FileInfo(filePath).Length;

        var shouldHash = !existing.HasValue || !existing.Value.fileSize.HasValue || existing.Value.fileSize.Value == fileSize;

        // T-SYNC-MTIME-001.2: RTM FR-14.4 - MD5 guard over raw bytes
        // Only compute hash when size hasn't changed (or no size recorded yet).
        string? fileHash = null;
        if (shouldHash)
        {
            fileHash = ComputeMD5Base64(filePath);
        }

        // T-SYNC-MTIME-001.1: RTM FR-14.2 - Hash guard: update last_indexed ONLY when content hash matches
        if (fileHash != null && existing.HasValue && !string.IsNullOrWhiteSpace(existing.Value.fileMd5) &&
            string.Equals(fileHash, existing.Value.fileMd5, StringComparison.Ordinal))
        {
            conn.Execute("UPDATE file_content SET last_indexed = @t, file_size = @s WHERE file_path = @p", new { t = timestamp, s = fileSize, p = memoryUri });
            return false;
        }

        ExecuteInsert(conn,
            "INSERT OR REPLACE INTO file_content (file_path, title, content, last_indexed, status, file_md5, file_size) VALUES (@path, @title, @content, @indexed, @status, @md5, @size)",
            new { path = memoryUri, title, content, indexed = timestamp, status = fileStatus, md5 = fileHash ?? ComputeMD5Base64(filePath), size = fileSize });

        var fileCreated = CultureInvariantHelpers.FormatDateTime(File.GetCreationTimeUtc(filePath), "O");
        foreach (var concept in concepts)
        {
            ExecuteInsert(conn, "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (@name, @seen)",
                new { name = concept, seen = fileCreated });

            var count = MarkdownIO.CountConceptOccurrences(content, concept);
            ExecuteInsert(conn, "INSERT OR REPLACE INTO concept_mentions (concept_name, source_file, mention_count) VALUES (@concept, @file, @count)",
                new { concept, file = memoryUri, count });
        }

        ConceptSyncVectorSupport.GenerateConceptEmbeddings(conn, concepts, vectorReady);
        ConceptSyncVectorSupport.GenerateFileEmbedding(conn, memoryUri, content, vectorReady);

        BuildGraphEdges(conn, concepts, memoryUri);

        return true;
    }

    /// <summary>
    /// Remove a file from the concept graph database: delete mentions, clean graph edges, remove vector data, and delete file_content row.
    /// </summary>
    internal static void RemoveFile(SqliteConnection conn, string memoryUri, bool vectorReady)
    {
        conn.Execute("DELETE FROM concept_mentions WHERE source_file = @file", new { file = memoryUri });

        var edges = conn.Query<(string a, string b, string files)>(
            "SELECT concept_a, concept_b, source_files FROM concept_graph WHERE source_files LIKE @pattern",
            new { pattern = $"%{memoryUri}%" });

        foreach (var edge in edges)
        {
            var fileList = JsonSerializer.Deserialize<List<string>>(edge.files, SafeJson.Options) ?? new();
            fileList.Remove(memoryUri);

            if (fileList.Count == 0)
            {
                conn.Execute("DELETE FROM concept_graph WHERE concept_a = @a AND concept_b = @b", new { a = edge.a, b = edge.b });
            }
            else
            {
                conn.Execute("UPDATE concept_graph SET co_occurrence_count = @count, source_files = @files WHERE concept_a = @a AND concept_b = @b",
                    new { count = fileList.Count, files = JsonSerializer.Serialize(fileList), a = edge.a, b = edge.b });
            }
        }

        if (vectorReady)
        {
            try
            {
                conn.Execute("DELETE FROM vec_memory_files WHERE file_path = @file", new { file = memoryUri });
            }
            catch (Exception ex)
            {
                // vec table operations may fail even when extension loaded - log and continue
                Console.Error.WriteLine($"[SYNC WARNING] Failed to clean vec_memory_files for '{memoryUri}': {ex.Message}");
            }
        }

        conn.Execute("DELETE FROM file_content WHERE file_path = @file", new { file = memoryUri });
    }

    /// <summary>
    /// Batch entry point: process a set of file paths within a single transaction.
    /// Does NOT do orphan cleanup, VACUUM, or FTS optimization (those are full-sync-only or maintenance-only).
    /// </summary>
    public static string SyncFiles(string[] filePaths)
    {
        GraphDatabase.InitializeDatabase();

        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        var vectorReady = ConceptSyncVectorSupport.TryEnsureVectorSupport(conn);

        var filesProcessed = 0;

        using var transaction = conn.BeginTransaction();
        var memoryFullPath = Path.GetFullPath(MemoryPath).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;

        foreach (var filePath in filePaths)
        {
            try
            {
                // Reject files outside the memory directory — prevents corrupt URIs from entering the DB
                var fileFullPath = Path.GetFullPath(filePath);
                if (!fileFullPath.StartsWith(memoryFullPath, StringComparison.OrdinalIgnoreCase))
                {
                    Console.Error.WriteLine($"[SYNC] Skipping file outside memory directory: {filePath}");
                    continue;
                }

                if (ProcessFile(conn, fileFullPath, vectorReady))
                    filesProcessed++;
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[SYNC ERROR] Failed to process file '{filePath}': {ex.GetType().Name}: {ex.Message}");
            }
        }
        transaction.Commit();

        var result = new StringBuilder();
        result.AppendLine("SyncFiles complete:");
        result.AppendLineInvariant($"- {filesProcessed} FILES processed");
        return result.ToString().TrimEnd();
    }

    public static string Sync()
    {
        var totalSyncTimer = Stopwatch.StartNew();
        GraphDatabase.InitializeDatabase();

        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        var vectorReady = ConceptSyncVectorSupport.TryEnsureVectorSupport(conn);

        var filesProcessed = 0;
        var files = Directory.GetFiles(MemoryPath, "*.md", SearchOption.AllDirectories);

        Console.Error.WriteLine($"[SYNC TELEMETRY] Processing {files.Length} files");
        using var cmdBefore = conn.CreateCommand();
        cmdBefore.CommandText = "SELECT COUNT(*) FROM file_search";
        var fileSearchCountBefore = Convert.ToInt32(cmdBefore.ExecuteScalar() ?? 0, CultureInfo.InvariantCulture);
        Console.Error.WriteLine($"[SYNC TELEMETRY] file_search rows before sync: {fileSearchCountBefore}");

        SqliteTransaction? transaction = null;
        int orphanedCount = 0;
        try
        {
            transaction = conn.BeginTransaction();

            // CLEANUP-001.1: RTM FR-14.6 - Targeted sweep before full enumeration.
            // This updates only candidate thinking sessions; other files remain guarded by mtime/hash.
            var abandonedSwept = SweepAbandonedThinkingSessions(conn, DateTime.UtcNow);

            foreach (var filePath in files)
            {
                try
                {
                    if (ProcessFile(conn, filePath, vectorReady))
                        filesProcessed++;
                }
                catch (Exception ex)
                {
                    // RTM: ERROR-HANDLING-001 - Log file processing errors and continue
                    Console.Error.WriteLine($"[SYNC ERROR] Failed to process file '{filePath}': {ex.GetType().Name}: {ex.Message}");
                    if (ex.InnerException != null)
                    {
                        Console.Error.WriteLine($"[SYNC ERROR]   Inner exception: {ex.InnerException.GetType().Name}: {ex.InnerException.Message}");
                    }
                    Console.Error.WriteLine($"[SYNC ERROR]   Stack trace: {ex.StackTrace}");
                    // Continue processing other files despite this error
                }
            }

            var dbFilePaths = conn.Query<string>("SELECT file_path FROM file_content").ToList();
            var deletedFilesCount = 0;

            foreach (var dbFilePath in dbFilePaths)
            {
                try
                {
                    var diskPath = UriToPath(dbFilePath);
                    if (!File.Exists(diskPath))
                    {
                        deletedFilesCount++;
                        RemoveFile(conn, dbFilePath, vectorReady);
                    }
                }
                catch (Exception ex) when (ex is ArgumentException or PathTooLongException or NotSupportedException)
                {
                    // Corrupt URI (e.g. path traversal, too-long path) — remove the invalid entry
                    Console.Error.WriteLine($"[SYNC] Removing invalid DB entry ({ex.GetType().Name}): {dbFilePath}");
                    deletedFilesCount++;
                    RemoveFile(conn, dbFilePath, vectorReady);
                }
            }

            if (deletedFilesCount > 0)
                Console.Error.WriteLine($"[SYNC TELEMETRY] Removed {deletedFilesCount} orphaned file entries from database");

            if (abandonedSwept > 0)
                Console.Error.WriteLine($"[SYNC TELEMETRY] Marked {abandonedSwept} thinking sessions as abandoned");

            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"
        DELETE FROM concepts
        WHERE concept_name NOT IN (
            SELECT DISTINCT concept_name
            FROM concept_mentions
        )";
            orphanedCount = cmd.ExecuteNonQuery();

            transaction.Commit();
        }
        catch (Exception)
        {
            if (transaction != null)
            {
                try
                {
                    transaction.Rollback();
                }
                catch (Exception rollbackEx)
                {
                    // RTM: ERROR-HANDLING-002 - Log rollback failure but preserve original exception
                    Console.Error.WriteLine($"[SYNC ERROR] Transaction rollback failed: {rollbackEx.GetType().Name}: {rollbackEx.Message}");
                    Console.Error.WriteLine($"[SYNC ERROR]   Rollback stack trace: {rollbackEx.StackTrace}");
                    // Original exception will be re-thrown below
                }
            }
            throw;
        }
        finally
        {
            transaction?.Dispose();
        }

        using var cmdAfter = conn.CreateCommand();
        cmdAfter.CommandText = "SELECT COUNT(*) FROM file_search";
        var fileSearchCountAfter = Convert.ToInt32(cmdAfter.ExecuteScalar() ?? 0, CultureInfo.InvariantCulture);
        Console.Error.WriteLine($"[SYNC TELEMETRY] file_search rows after sync: {fileSearchCountAfter}");

        totalSyncTimer.Stop();
        Console.Error.WriteLine($"[SYNC TELEMETRY] Total sync completed in {totalSyncTimer.ElapsedMilliseconds}ms");

        // T-COMMUNITY-001.5: RTM FR-13.4 - Run community detection after concept extraction
        // T-COMMUNITY-001.11: RTM FAIL-001 remediation — set write guard so DB watcher skips during Sync
        int communityCount = 0;
        double modularity = 0.0;
        IncrementalSyncTools.SetCommunityWriteInProgress(true);
        try
        {
            (communityCount, modularity) = CommunityDetection.RunAndPersist(conn, Config.LouvainGamma);
            Console.Error.WriteLine($"[SYNC TELEMETRY] Community detection: {communityCount} communities, modularity {modularity:F3}");
        }
        finally
        {
            IncrementalSyncTools.SetCommunityWriteInProgress(false);
        }

        var conceptMentionCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_mentions");
        var edgeCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_graph");

        var result = new StringBuilder();
        result.AppendLine("Sync complete:");
        result.AppendLineInvariant($"- {filesProcessed} FILES processed");
        result.AppendLineInvariant($"- {conceptMentionCount} CONCEPT mentions found");
        result.AppendLineInvariant($"- {edgeCount} CONCEPT relations created");
        if (orphanedCount > 0)
            result.AppendLineInvariant($"- {orphanedCount} ORPHANED concepts cleaned up");
        result.AppendLineInvariant($"- {communityCount} COMMUNITIES detected (modularity {modularity:F3})");

        if (vectorReady)
        {
            foreach (var line in ConceptSyncVectorSupport.BuildVectorVerificationReport(conn))
            {
                result.AppendLine(line);
            }
        }
        else
        {
            result.AppendLine("- WARNING: sqlite-vec extension unavailable; semantic tooling disabled");
        }

        conn.Execute("VACUUM");
        Console.Error.WriteLine($"[SYNC TELEMETRY] Database VACUUMed to reclaim space");

        return result.ToString().TrimEnd();
    }

    private static void BuildGraphEdges(SqliteConnection conn, string[] concepts, string memoryUri)
    {
        if (concepts.Length <= 1) return;

        for (int i = 0; i < concepts.Length - 1; i++)
            for (int j = i + 1; j < concepts.Length; j++)
            {
                var (a, b) = string.CompareOrdinal(concepts[i], concepts[j]) < 0 ? (concepts[i], concepts[j]) : (concepts[j], concepts[i]);
                var existingData = conn.Query<EdgeData>("SELECT co_occurrence_count, source_files FROM concept_graph WHERE concept_a = @a AND concept_b = @b",
                                new { a, b }).FirstOrDefault();

                if (existingData != null)
                {
                    var files = string.IsNullOrWhiteSpace(existingData.source_files)
                        ? new List<string>()
                        : JsonSerializer.Deserialize<List<string>>(existingData.source_files, SafeJson.Options) ?? new();

                    if (!files.Contains(memoryUri))
                    {
                        files.Add(memoryUri);
                        conn.Execute("UPDATE concept_graph SET co_occurrence_count = @count, source_files = @files WHERE concept_a = @a AND concept_b = @b",
                            new { count = files.Count, files = JsonSerializer.Serialize(files), a, b });
                    }
                }
                else
                {
                    ExecuteInsert(conn, "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (@a, @b, @count, @files)",
                        new { a, b, count = 1, files = JsonSerializer.Serialize(new[] { memoryUri }) });
                }
            }
    }

    private static string PathToUri(string filePath) => MarkdownIO.PathToUri(filePath, MemoryPath);
    private static string UriToPath(string uri) => MarkdownIO.UriToPath(uri, MemoryPath);

    // T-SYNC-MTIME-001.2: RTM NFR-14.2.2 - MD5 over raw file bytes for cheap change detection
    // CA5351 is about cryptographic security. Here, MD5 is used only for non-adversarial change detection.
#pragma warning disable CA5351
    internal static string ComputeMD5Base64(string filePath)
    {
        using var stream = File.OpenRead(filePath);
        var hash = MD5.HashData(stream);
        return Convert.ToBase64String(hash);
    }
#pragma warning restore CA5351
    private static string GetTitle(string filePath, Dictionary<string, object?>? frontmatter)
    {
        var title = Path.GetFileNameWithoutExtension(filePath);
        if (string.IsNullOrWhiteSpace(title)) title = "Untitled";
        if (frontmatter?.TryGetValue("title", out var t) == true && !string.IsNullOrWhiteSpace(t?.ToString()))
            title = t.ToString()!;
        return title;
    }
}
