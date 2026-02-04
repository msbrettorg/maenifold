using System.Diagnostics;
using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;

namespace Maenifold.Tools;

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

    /// <summary>
    /// Helper to execute INSERT OR IGNORE/REPLACE SQL with consistent error handling.
    /// Consolidates duplicate SQL execution patterns (SIMP-004).
    /// </summary>
    private static void ExecuteInsert(SqliteConnection conn, string sql, object parameters)
    {
        conn.Execute(sql, parameters);
    }

    public static string Sync()
    {
        var totalSyncTimer = Stopwatch.StartNew();
        GraphDatabase.InitializeDatabase();

        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();

        var vectorReady = ConceptSyncVectorSupport.TryEnsureVectorSupport(conn);

        var stats = (filesProcessed: 0, conceptsFound: 0, relationsCreated: 0);
        var files = Directory.GetFiles(MemoryPath, "*.md", SearchOption.AllDirectories);
        var extractionTimer = new Stopwatch();
        var embeddingTimer = new Stopwatch();
        var uniqueConcepts = new HashSet<string>();
        var conceptEmbeddingsGenerated = 0;
        var fileEmbeddingsGenerated = 0;

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

            foreach (var filePath in files)
            {
                try
                {
                    var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(filePath);
                    if (Config.EnableSessionCleanup && frontmatter != null)
                    {
                        SessionCleanup.HandleSessionCleanup(frontmatter!, filePath, content);
                    }
                    extractionTimer.Start();
                    var concepts = MarkdownIO.ExtractWikiLinks(content).ToArray();
                    extractionTimer.Stop();
                    foreach (var concept in concepts)
                    {
                        uniqueConcepts.Add(concept);
                    }

                    var memoryUri = PathToUri(filePath);
                    var title = GetTitle(filePath, frontmatter!);
                    var fileStatus = frontmatter?.ContainsKey("status") == true ? frontmatter["status"]?.ToString() : null;
                    var timestamp = CultureInvariantHelpers.FormatDateTime(File.GetLastWriteTimeUtc(filePath), "O");
                    var fileHash = ComputeSHA256(content);

                    var existing = conn.QuerySingle<(string? t, string? m)?>(
                        "SELECT last_indexed, file_md5 FROM file_content WHERE file_path = @path",
                        new { path = memoryUri });

                    if (existing.HasValue && (timestamp == existing.Value.t || fileHash == existing.Value.m))
                    {
                        if (timestamp != existing.Value.t && fileHash == existing.Value.m)
                            conn.Execute("UPDATE file_content SET last_indexed = @t WHERE file_path = @p", new { t = timestamp, p = memoryUri });
                        continue;
                    }
                    ExecuteInsert(conn,
                        "INSERT OR REPLACE INTO file_content (file_path, title, content, last_indexed, status, file_md5) VALUES (@path, @title, @content, @indexed, @status, @md5)",
                        new { path = memoryUri, title, content, indexed = timestamp, status = fileStatus, md5 = fileHash });
                    var fileCreated = CultureInvariantHelpers.FormatDateTime(File.GetCreationTimeUtc(filePath), "O");
                    foreach (var concept in concepts)
                    {
                        ExecuteInsert(conn, "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (@name, @seen)",
                            new { name = concept, seen = fileCreated });

                        var count = MarkdownIO.CountConceptOccurrences(content, concept);
                        ExecuteInsert(conn, "INSERT OR REPLACE INTO concept_mentions (concept_name, source_file, mention_count) VALUES (@concept, @file, @count)",
                            new { concept, file = memoryUri, count });

                        stats.conceptsFound++;
                    }
                    embeddingTimer.Start();
                    var conceptEmbeddingsThisFile = ConceptSyncVectorSupport.GenerateConceptEmbeddings(conn, concepts, vectorReady);
                    var fileEmbeddingGenerated = ConceptSyncVectorSupport.GenerateFileEmbedding(conn, memoryUri, content, vectorReady);
                    embeddingTimer.Stop();

                    conceptEmbeddingsGenerated += conceptEmbeddingsThisFile;
                    if (fileEmbeddingGenerated) fileEmbeddingsGenerated++;
                    stats = BuildGraphEdges(conn, concepts, memoryUri, stats);

                    stats.filesProcessed++;
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
            var filePaths = conn.Query<string>("SELECT file_path FROM file_content").ToList();
            var deletedFilesCount = 0;

            foreach (var filePath in filePaths)
            {
                var diskPath = UriToPath(filePath);
                if (!File.Exists(diskPath))
                {
                    deletedFilesCount++;
                    var rowId = conn.QuerySingle<long?>("SELECT rowid FROM file_content WHERE file_path = @path", new { path = filePath });

                    conn.Execute("DELETE FROM concept_mentions WHERE source_file = @file", new { file = filePath });
                    var edges = conn.Query<(string a, string b, string files)>(
                        "SELECT concept_a, concept_b, source_files FROM concept_graph WHERE source_files LIKE @pattern",
                        new { pattern = $"%{filePath}%" });

                    foreach (var edge in edges)
                    {
                        var fileList = JsonSerializer.Deserialize<List<string>>(edge.files, SafeJson.Options) ?? new();
                        fileList.Remove(filePath);

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
                        conn.Execute("DELETE FROM vec_memory_files WHERE file_path = @file", new { file = filePath });
                    conn.Execute("DELETE FROM file_content WHERE file_path = @file", new { file = filePath });
                }
            }

            if (deletedFilesCount > 0)
                Console.Error.WriteLine($"[SYNC TELEMETRY] Removed {deletedFilesCount} orphaned file entries from database");

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

        Console.Error.WriteLine($"[SYNC TELEMETRY] Extracted {uniqueConcepts.Count} unique concepts in {extractionTimer.ElapsedMilliseconds}ms");
        Console.Error.WriteLine($"[SYNC TELEMETRY] Generated {conceptEmbeddingsGenerated} concept embeddings in {embeddingTimer.ElapsedMilliseconds}ms");
        Console.Error.WriteLine($"[SYNC TELEMETRY] Generated {fileEmbeddingsGenerated} file embeddings in {embeddingTimer.ElapsedMilliseconds}ms");
        using var cmdAfter = conn.CreateCommand();
        cmdAfter.CommandText = "SELECT COUNT(*) FROM file_search";
        var fileSearchCountAfter = Convert.ToInt32(cmdAfter.ExecuteScalar() ?? 0, CultureInfo.InvariantCulture);
        Console.Error.WriteLine($"[SYNC TELEMETRY] file_search rows after sync: {fileSearchCountAfter}");

        totalSyncTimer.Stop();
        Console.Error.WriteLine($"[SYNC TELEMETRY] Total sync completed in {totalSyncTimer.ElapsedMilliseconds}ms");

        var result = new StringBuilder();
        result.AppendLine("Sync complete:");
        result.AppendLineInvariant($"- {stats.filesProcessed} FILES processed");
        result.AppendLineInvariant($"- {stats.conceptsFound} CONCEPT mentions found");
        result.AppendLineInvariant($"- {stats.relationsCreated} CONCEPT relations created");
        if (orphanedCount > 0)
            result.AppendLineInvariant($"- {orphanedCount} ORPHANED concepts cleaned up");

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
    private static (int filesProcessed, int conceptsFound, int relationsCreated) BuildGraphEdges(SqliteConnection conn, string[] concepts, string memoryUri, (int filesProcessed, int conceptsFound, int relationsCreated) stats)
    {
        if (concepts.Length <= 1) return stats;

        for (int i = 0; i < concepts.Length - 1; i++)
            for (int j = i + 1; j < concepts.Length; j++)
            {
                var (a, b) = string.CompareOrdinal(concepts[i], concepts[j]) < 0 ? (concepts[i], concepts[j]) : (concepts[j], concepts[i]);
                var existingData = conn.Query<EdgeData>("SELECT co_occurrence_count, source_files FROM concept_graph WHERE concept_a = @a AND concept_b = @b",
                                new { a, b }).FirstOrDefault();

                if (existingData != null)
                {
                    var files = JsonSerializer.Deserialize<List<string>>(existingData.source_files, SafeJson.Options) ?? new();

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
                    stats.relationsCreated++;
                }
            }
        return stats;
    }

    private static string PathToUri(string filePath) => MarkdownIO.PathToUri(filePath, MemoryPath);
    private static string UriToPath(string uri) => MarkdownIO.UriToPath(uri, MemoryPath);
    private static string ComputeSHA256(string content) => Convert.ToBase64String(SHA256.HashData(Encoding.UTF8.GetBytes(content)));
    private static string GetTitle(string filePath, Dictionary<string, object?>? frontmatter)
    {
        var title = Path.GetFileNameWithoutExtension(filePath);
        if (string.IsNullOrWhiteSpace(title)) title = "Untitled";
        if (frontmatter?.TryGetValue("title", out var t) == true && !string.IsNullOrWhiteSpace(t?.ToString()))
            title = t.ToString()!;
        return title;
    }
}
