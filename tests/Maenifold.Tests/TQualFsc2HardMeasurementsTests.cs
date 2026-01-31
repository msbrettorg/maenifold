#pragma warning disable CA1861
using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using Maenifold.Utils;
using Maenifold.Tools;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

[NonParallelizable]
public class TQualFsc2HardMeasurementsTests
{
    [Test]
    public void CaptureEmbeddingPlateauDiagnostics()
    {
        // T-QUAL-FSC2: RTM FR-7.4
        GraphDatabase.InitializeDatabase();

        var queries = new[]
        {
            "mcp",
            "oauth2",
            "finops",
            "graphrag",
            "xqz",
            "zzzzzzzz",
            "graph-rag"
        };

        using var connection = new SqliteConnection(Config.DatabaseConnectionString);
        connection.Open();
        connection.LoadVectorExtension();

        var conceptNames = new List<string>();
        var hashGroups = new Dictionary<string, List<string>>(StringComparer.OrdinalIgnoreCase);
        var inserted = 0;

        try
        {
            foreach (var query in queries)
            {
                // T-QUAL-FSC2: RTM FR-7.4
                var embedding = VectorTools.GenerateEmbedding(query);
                var blob = VectorTools.ToSqliteVectorBlob(embedding);
                var hash = ComputeSha256Hex(blob);

                if (!hashGroups.TryGetValue(hash, out var list))
                {
                    list = new List<string>();
                    hashGroups[hash] = list;
                }

                list.Add(query);
                conceptNames.Add(query);

                using var insert = connection.CreateCommand();
                insert.CommandText = "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (@concept, @embedding)";
                insert.Parameters.AddWithValue("@concept", query);
                insert.Parameters.Add("@embedding", SqliteType.Blob).Value = blob;
                inserted += insert.ExecuteNonQuery();
            }

            TestContext.Out.WriteLine("Embedding SHA256 groups with identical hashes:");
            var duplicateGroups = hashGroups.Where(g => g.Value.Count > 1).ToList();
            if (duplicateGroups.Count == 0)
            {
                TestContext.Out.WriteLine("  (none)");
            }
            else
            {
                foreach (var group in duplicateGroups)
                {
                    TestContext.Out.WriteLine($"  {group.Key}: {string.Join(", ", group.Value)}");
                }
            }

            var probeEmbedding = VectorTools.GenerateEmbedding("mcp");
            var probeBlob = VectorTools.ToSqliteVectorBlob(probeEmbedding);

            var count1e6 = CountDistancesAtOrBelow(connection, conceptNames, probeBlob, 1e-6);
            var count1e12 = CountDistancesAtOrBelow(connection, conceptNames, probeBlob, 1e-12);

            TestContext.Out.WriteLine($"Distances <= 1e-6 (probe=mcp): {count1e6}");
            TestContext.Out.WriteLine($"Distances <= 1e-12 (probe=mcp): {count1e12}");

            TestContext.Out.WriteLine("Top distances (probe=mcp):");
            foreach (var row in TopDistances(connection, conceptNames, probeBlob, 20))
            {
                TestContext.Out.WriteLine($"  {row.ConceptName} => {row.Distance.ToString("F12", CultureInfo.InvariantCulture)}");
            }

            Assert.That(inserted, Is.EqualTo(queries.Length), "Expected all embeddings to be inserted.");
            Assert.That(count1e6, Is.GreaterThanOrEqualTo(1), "Expected probe embedding to match at least itself.");
        }
        finally
        {
            CleanupSeededConcepts(connection, conceptNames);
        }
    }

    private static int CountDistancesAtOrBelow(SqliteConnection connection, List<string> conceptNames, byte[] probeBlob, double threshold)
    {
        // T-QUAL-FSC2: RTM FR-7.4
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $@"
            SELECT COUNT(*)
            FROM vec_concepts
            WHERE {BuildConceptFilterSql(conceptNames)}
              AND vec_distance_cosine(embedding, @probe) <= @threshold";

        AddConceptParameters(cmd, conceptNames);
        cmd.Parameters.Add("@probe", SqliteType.Blob).Value = probeBlob;
        cmd.Parameters.AddWithValue("@threshold", threshold);

        return Convert.ToInt32(cmd.ExecuteScalar(), CultureInfo.InvariantCulture);
    }

    private static IEnumerable<(string ConceptName, double Distance)> TopDistances(
        SqliteConnection connection,
        List<string> conceptNames,
        byte[] probeBlob,
        int limit)
    {
        // T-QUAL-FSC2: RTM FR-7.4
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $@"
            SELECT concept_name, vec_distance_cosine(embedding, @probe) AS distance
            FROM vec_concepts
            WHERE {BuildConceptFilterSql(conceptNames)}
            ORDER BY distance ASC
            LIMIT @limit";

        AddConceptParameters(cmd, conceptNames);
        cmd.Parameters.Add("@probe", SqliteType.Blob).Value = probeBlob;
        cmd.Parameters.AddWithValue("@limit", limit);

        using var reader = cmd.ExecuteReader();
        while (reader.Read())
        {
            var name = reader.GetString(0);
            var distance = reader.IsDBNull(1) ? double.NaN : reader.GetDouble(1);
            yield return (name, distance);
        }
    }

    private static void CleanupSeededConcepts(SqliteConnection connection, List<string> conceptNames)
    {
        if (conceptNames.Count == 0)
        {
            return;
        }

        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"DELETE FROM vec_concepts WHERE {BuildConceptFilterSql(conceptNames)}";
        AddConceptParameters(cmd, conceptNames);
        cmd.ExecuteNonQuery();
    }

    private static string BuildConceptFilterSql(List<string> conceptNames)
    {
        var placeholders = new StringBuilder();
        for (var i = 0; i < conceptNames.Count; i++)
        {
            if (i > 0)
            {
                placeholders.Append(", ");
            }
            placeholders.Append(CultureInfo.InvariantCulture, $"@concept{i}");
        }

        return $"concept_name IN ({placeholders})";
    }

    private static void AddConceptParameters(SqliteCommand cmd, List<string> conceptNames)
    {
        for (var i = 0; i < conceptNames.Count; i++)
        {
            cmd.Parameters.AddWithValue($"@concept{i}", conceptNames[i]);
        }
    }

    private static string ComputeSha256Hex(byte[] data)
    {
        var hash = SHA256.HashData(data);
        var builder = new StringBuilder(hash.Length * 2);
        foreach (var value in hash)
        {
            builder.Append(value.ToString("x2", CultureInfo.InvariantCulture));
        }

        return builder.ToString();
    }
}

#pragma warning restore CA1861
