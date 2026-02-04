using System.Globalization;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;

namespace Maenifold.Tools;

internal static class ConceptSyncVectorSupport
{
    public static int GenerateConceptEmbeddings(SqliteConnection conn, string[] concepts, bool vectorReady)
    {
        if (!vectorReady || concepts.Length == 0) return 0;

        var embeddingsGenerated = 0;
        try
        {
            var expectedBytes = VectorTools.EmbeddingLength * sizeof(float);

            foreach (var concept in concepts)
            {
                var hasEmbedding = conn.QuerySingle<bool>(
                    "SELECT COUNT(*) > 0 FROM vec_concepts WHERE concept_name = @concept AND typeof(embedding) = 'blob' AND length(embedding) = @expectedBytes",
                    new { concept, expectedBytes });

                if (hasEmbedding) continue;

                var embedding = VectorTools.GenerateEmbedding(concept);
                var embeddingBlob = VectorTools.ToSqliteVectorBlob(embedding);
                if (embeddingBlob.Length != expectedBytes) continue;

                conn.Execute(
                                    "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (@concept, @embedding)",
                                    new { concept, embedding = embeddingBlob });

                embeddingsGenerated++;
            }
        }
        catch (Exception)
        {

        }

        return embeddingsGenerated;
    }

    public static bool GenerateFileEmbedding(SqliteConnection conn, string filePath, string content, bool vectorReady)
    {
        if (!vectorReady) return false;

        try
        {
            var expectedBytes = VectorTools.EmbeddingLength * sizeof(float);
            var hasEmbedding = conn.QuerySingle<bool>(
                "SELECT COUNT(*) > 0 FROM vec_memory_files WHERE file_path = @filePath AND typeof(embedding) = 'blob' AND length(embedding) = @expectedBytes",
                new { filePath, expectedBytes });

            if (hasEmbedding) return false;


            var textToEmbed = content.Length > 1000 ? content.Substring(0, 1000) : content;
            var embedding = VectorTools.GenerateEmbedding(textToEmbed);
            var embeddingBlob = VectorTools.ToSqliteVectorBlob(embedding);
            if (embeddingBlob.Length != expectedBytes) return false;

            conn.Execute(
                            "INSERT OR REPLACE INTO vec_memory_files (file_path, embedding) VALUES (@filePath, @embedding)",
                            new { filePath, embedding = embeddingBlob });

            return true;
        }
        catch (Exception)
        {

            return false;
        }
    }

    public static bool TryEnsureVectorSupport(SqliteConnection conn)
    {
        try
        {
            conn.LoadVectorExtension();

            // Verify that vec tables actually work - not just that extension loaded.
            // On some CI environments (GitHub Actions Linux), the extension loads but
            // virtual table operations fail with SQLite Error 16.
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT COUNT(*) FROM vec_concepts";
            cmd.ExecuteScalar();

            return true;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Warning: sqlite-vec unavailable: {ex.Message}");
            return false;
        }
    }

    public static IEnumerable<string> BuildVectorVerificationReport(SqliteConnection conn)
    {
        var lines = new List<string>();

        try
        {
            var conceptCount = ExecuteScalarInt(conn, "SELECT COUNT(*) FROM vec_concepts WHERE embedding IS NOT NULL");
            var fileCount = ExecuteScalarInt(conn, "SELECT COUNT(*) FROM vec_memory_files WHERE embedding IS NOT NULL");

            if (conceptCount == 0 && fileCount == 0)
            {
                lines.Add("- WARNING: vector tables are empty after sync");
                return lines;
            }

            lines.Add("Vector index verification:");

            if (conceptCount > 0)
            {
                var conceptBytes = ExecuteScalarInt(conn, "SELECT length(embedding) FROM vec_concepts WHERE embedding IS NOT NULL LIMIT 1");
                var conceptDims = conceptBytes / sizeof(float);
                var conceptSelf = ExecuteScalarDouble(conn, "SELECT vec_distance_cosine(embedding, embedding) FROM vec_concepts WHERE embedding IS NOT NULL LIMIT 1");
                lines.Add($"- CONCEPT embeddings: {conceptCount} rows ({conceptDims} dims, self-distance {conceptSelf?.ToString("F6", CultureInfo.InvariantCulture) ?? "n/a"})");
            }
            else
            {
                lines.Add("- CONCEPT embeddings: none");
            }

            if (fileCount > 0)
            {
                var fileBytes = ExecuteScalarInt(conn, "SELECT length(embedding) FROM vec_memory_files WHERE embedding IS NOT NULL LIMIT 1");
                var fileDims = fileBytes / sizeof(float);
                var fileSelf = ExecuteScalarDouble(conn, "SELECT vec_distance_cosine(embedding, embedding) FROM vec_memory_files WHERE embedding IS NOT NULL LIMIT 1");
                lines.Add($"- MEMORY embeddings: {fileCount} rows ({fileDims} dims, self-distance {fileSelf?.ToString("F6", CultureInfo.InvariantCulture) ?? "n/a"})");
            }
            else
            {
                lines.Add("- MEMORY embeddings: none");
            }
        }
        catch (Exception ex)
        {
            lines.Clear();
            lines.Add($"- WARNING: vector verification skipped ({ex.Message})");
        }

        return lines;
    }

    private static int ExecuteScalarInt(SqliteConnection connection, string sql)
    {
        using var cmd = connection.CreateCommand();
        cmd.CommandText = sql;
        var value = cmd.ExecuteScalar();
        return value == null || value == DBNull.Value
            ? 0
            : Convert.ToInt32(value, CultureInfo.InvariantCulture);
    }

    private static double? ExecuteScalarDouble(SqliteConnection connection, string sql)
    {
        using var cmd = connection.CreateCommand();
        cmd.CommandText = sql;
        var value = cmd.ExecuteScalar();
        return value == null || value == DBNull.Value
            ? null
            : Convert.ToDouble(value, CultureInfo.InvariantCulture);
    }
}
