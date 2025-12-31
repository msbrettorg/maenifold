using System.Globalization;
using System.Text.Json;
using Microsoft.Data.Sqlite;
using Maenifold.Utils;

namespace Maenifold.Tools;

public partial class IncrementalSyncTools
{
    private static string MemoryPath => Config.MemoryPath;
    private static string SyncLogPath => Path.Combine(Config.MaenifoldRoot, "logs", "incremental-sync.log");

    private static string PathToUri(string path) => MarkdownIO.PathToUri(path, MemoryPath);

    private static void LogSync(string message, Exception? ex = null)
    {
        if (!Config.EnableSyncLogging)
            return;

        try
        {
            var directory = Path.GetDirectoryName(SyncLogPath);
            if (!string.IsNullOrEmpty(directory))
            {
                Directory.CreateDirectory(directory);
            }

            var line = ex == null
                ? message
                : $"{message}{Environment.NewLine}{ex}";
            var entry = $"[{DateTime.UtcNow:O}] {line}{Environment.NewLine}";
            File.AppendAllText(SyncLogPath, entry);
        }
        catch
        {

        }
    }

    private static void UpsertFileContent(SqliteConnection conn, string memoryUri, string title, string content, string? status)
    {
        conn.Execute(
            "INSERT OR REPLACE INTO file_content (file_path, title, content, last_indexed, status) VALUES (@path, @title, @content, @indexed, @status)",
            new
            {
                path = memoryUri,
                title,
                content,
                indexed = TimeZoneConverter.GetUtcNowIso(),
                status
            });
    }

    private static void UpsertConceptMetadata(SqliteConnection conn, string memoryUri, string content, IReadOnlyCollection<string> concepts, DateTime fileCreatedUtc)
    {
        conn.Execute("DELETE FROM concept_mentions WHERE source_file = @file", new { file = memoryUri });

        foreach (var concept in concepts)
        {
            conn.Execute(
                "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (@name, @seen)",
                new { name = concept, seen = fileCreatedUtc.ToString("O") });

            var mentionCount = MarkdownIO.CountConceptOccurrences(content, concept);
            conn.Execute(
                "INSERT OR REPLACE INTO concept_mentions (concept_name, source_file, mention_count) VALUES (@concept, @file, @count)",
                new
                {
                    concept,
                    file = memoryUri,
                    count = mentionCount
                });
        }
    }

    private static void RemoveFileFromGraph(SqliteConnection conn, string memoryUri)
    {
        var edges = conn.Query<(string conceptA, string conceptB, string files)>(
            "SELECT concept_a, concept_b, source_files FROM concept_graph");

        foreach (var (conceptA, conceptB, files) in edges)
        {
            if (string.IsNullOrWhiteSpace(files))
            {
                continue;
            }

            List<string>? fileList;
            try
            {
                fileList = JsonSerializer.Deserialize<List<string>>(files, SafeJson.Options);
            }
            catch
            {
                fileList = null;
            }

            if (fileList == null || !fileList.Remove(memoryUri))
            {
                continue;
            }

            if (fileList.Count == 0)
            {
                conn.Execute(
                    "DELETE FROM concept_graph WHERE concept_a = @a AND concept_b = @b",
                    new { a = conceptA, b = conceptB });
            }
            else
            {
                conn.Execute(
                    "UPDATE concept_graph SET co_occurrence_count = @count, source_files = @files WHERE concept_a = @a AND concept_b = @b",
                    new
                    {
                        a = conceptA,
                        b = conceptB,
                        count = fileList.Count,
                        files = JsonSerializer.Serialize(fileList)
                    });
            }
        }
    }

    private static long? GetFileRowId(SqliteConnection conn, string memoryUri)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT rowid FROM file_content WHERE file_path = @path";
        cmd.Parameters.AddWithValue("@path", memoryUri);
        var result = cmd.ExecuteScalar();
        if (result == null || result == DBNull.Value)
        {
            return null;
        }

        return Convert.ToInt64(result, CultureInfo.InvariantCulture);
    }

    private static void UpdateConceptRelations(SqliteConnection conn, IReadOnlyList<string> concepts, string memoryUri)
    {
        RemoveFileFromGraph(conn, memoryUri);

        if (concepts.Count <= 1)
        {
            return;
        }

        for (int i = 0; i < concepts.Count - 1; i++)
        {
            for (int j = i + 1; j < concepts.Count; j++)
            {
                var (a, b) = string.CompareOrdinal(concepts[i], concepts[j]) < 0
                    ? (concepts[i], concepts[j])
                    : (concepts[j], concepts[i]);

                var existing = conn.QuerySingle<(int count, string files)?>(
                    "SELECT co_occurrence_count, source_files FROM concept_graph WHERE concept_a = @a AND concept_b = @b",
                    new { a, b });

                List<string> fileList = existing?.files != null
                    ? JsonSerializer.Deserialize<List<string>>(existing.Value.files, SafeJson.Options) ?? new List<string>()
                    : new List<string>();

                if (!fileList.Contains(memoryUri))
                {
                    fileList.Add(memoryUri);
                }

                conn.Execute(
                    "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (@a, @b, @count, @files)",
                    new
                    {
                        a,
                        b,
                        count = fileList.Count,
                        files = JsonSerializer.Serialize(fileList)
                    });
            }
        }
    }

    private static void UpdateVectorState(SqliteConnection conn, string memoryUri, string content, IReadOnlyCollection<string> concepts)
    {
        try
        {
            conn.LoadVectorExtension();
        }
        catch (Exception ex)
        {
            LogSync("Failed to load sqlite-vec extension during incremental sync.", ex);
            return;
        }

        try
        {
            var textToEmbed = content.Length > 1000 ? content[..1000] : content;
            var fileEmbedding = VectorTools.GenerateEmbedding(textToEmbed);
            var fileBytes = new byte[fileEmbedding.Length * sizeof(float)];
            Buffer.BlockCopy(fileEmbedding, 0, fileBytes, 0, fileBytes.Length);

            using var fileCmd = conn.CreateCommand();
            fileCmd.CommandText = "INSERT OR REPLACE INTO vec_memory_files (file_path, embedding) VALUES (@path, @embedding)";
            fileCmd.Parameters.AddWithValue("@path", memoryUri);
            var fileParam = fileCmd.CreateParameter();
            fileParam.ParameterName = "@embedding";
            fileParam.SqliteType = SqliteType.Blob;
            fileParam.Value = fileBytes;
            fileCmd.Parameters.Add(fileParam);
            fileCmd.ExecuteNonQuery();
        }
        catch (Exception ex)
        {
            LogSync($"Failed to update vector embedding for file '{memoryUri}'.", ex);
        }

        foreach (var concept in concepts)
        {
            try
            {
                var hasEmbedding = conn.QuerySingle<bool>(
                    "SELECT COUNT(*) > 0 FROM vec_concepts WHERE concept_name = @concept",
                    new { concept });

                if (hasEmbedding)
                {
                    continue;
                }

                var embedding = VectorTools.GenerateEmbedding(concept);
                var bytes = new byte[embedding.Length * sizeof(float)];
                Buffer.BlockCopy(embedding, 0, bytes, 0, bytes.Length);

                using var cmd = conn.CreateCommand();
                cmd.CommandText = "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (@concept, @embedding)";
                cmd.Parameters.AddWithValue("@concept", concept);
                var vectorParam = cmd.CreateParameter();
                vectorParam.ParameterName = "@embedding";
                vectorParam.SqliteType = SqliteType.Blob;
                vectorParam.Value = bytes;
                cmd.Parameters.Add(vectorParam);
                cmd.ExecuteNonQuery();
            }
            catch (Exception ex)
            {
                LogSync($"Failed to update vector embedding for concept '{concept}'.", ex);
            }
        }
    }

    private static void RemoveVectorState(SqliteConnection conn, string memoryUri)
    {
        try
        {
            conn.LoadVectorExtension();
        }
        catch
        {
            return;
        }

        conn.Execute(
            "DELETE FROM vec_memory_files WHERE file_path = @path",
            new { path = memoryUri });
    }

    private static void UpdateFullTextIndex(SqliteConnection conn, string memoryUri)
    {
        var rowId = GetFileRowId(conn, memoryUri);
        if (!rowId.HasValue)
        {
            return;
        }

        conn.Execute("INSERT INTO file_search(file_search, rowid) VALUES('delete', @rowid)", new { rowid = rowId.Value });
        conn.Execute(@"
            INSERT INTO file_search(rowid, title, content, summary)
            SELECT rowid, title, content, summary
            FROM file_content
            WHERE rowid = @rowid",
            new { rowid = rowId.Value });
    }

    private static void RemoveFullTextIndex(SqliteConnection conn, long rowId)
    {
        conn.Execute("INSERT INTO file_search(file_search, rowid) VALUES('delete', @rowid)", new { rowid = rowId });
    }

    private static void RunMaintenance(bool runOptimize, bool runVacuum)
    {
        try
        {
            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenWithWAL();

            if (runOptimize)
            {
                conn.Execute("INSERT INTO file_search(file_search) VALUES('optimize')");
            }

            if (runVacuum)
            {
                conn.Execute("INSERT INTO file_search(file_search) VALUES('rebuild')");
                conn.Execute("PRAGMA wal_checkpoint(TRUNCATE)");
                conn.Execute("VACUUM");
                conn.Execute("PRAGMA journal_mode=WAL");
                conn.Execute("PRAGMA synchronous=NORMAL");
            }

            if (runOptimize || runVacuum)
            {
                var message = runOptimize && runVacuum
                    ? "Incremental maintenance: optimized FTS and vacuumed database."
                    : runOptimize
                        ? "Incremental maintenance: optimized FTS index."
                        : "Incremental maintenance: vacuumed database.";
                LogSync(message);
            }
        }
        catch (Exception ex)
        {
            LogSync("Incremental maintenance failed.", ex);
        }
    }
}
