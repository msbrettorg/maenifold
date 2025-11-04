using ModelContextProtocol.Server;
using System.ComponentModel;
using System.Text;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;

namespace Maenifold.Tools;

[McpServerToolType]
public class VectorSearchTools
{
    [McpServerTool, Description(@"Find concepts similar to input using vector embeddings for semantic similarity discovery.
Select when AI needs to discover related concepts based on semantic meaning rather than co-occurrence patterns.
Uses 384-dimensional embeddings with cosine similarity to find conceptually related terms.
Integrates with BuildContext for relationship exploration and SearchMemories for related file discovery.
Returns ranked concepts by semantic similarity score for knowledge graph exploration.")]
    public static string FindSimilarConcepts(
        [Description("Concept name to find similar concepts for")] string conceptName,
        [Description("Maximum number of similar concepts to return")] int maxResults = 10,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(FindSimilarConcepts).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(FindSimilarConcepts)}";
            return File.ReadAllText(helpPath);
        }

        try
        {
            if (string.IsNullOrWhiteSpace(conceptName))
            {
                return "ERROR: conceptName parameter is required and cannot be empty";
            }

            conceptName = MarkdownIO.NormalizeConcept(conceptName);


            var queryEmbedding = VectorTools.GenerateEmbedding(conceptName);
            var embeddingBlob = VectorTools.ToSqliteVectorBlob(queryEmbedding);
            if (embeddingBlob.Length == 0)
            {
                return $"Unable to generate embedding for '{conceptName}'.";
            }


            using var connection = new SqliteConnection(Config.DatabaseConnectionString);
            connection.OpenReadOnlyWithVector();

            var sql = @"
                SELECT concept_name, vec_distance_cosine(embedding, @embedding) as distance
                FROM vec_concepts
                WHERE concept_name != @concept AND concept_name != ''
                ORDER BY distance
                LIMIT @maxResults";

            using var command = new SqliteCommand(sql, connection);
            command.Parameters.Add("@embedding", SqliteType.Blob).Value = embeddingBlob;
            command.Parameters.AddWithValue("@concept", conceptName);
            command.Parameters.Add("@maxResults", SqliteType.Integer).Value = maxResults;

            var results = new List<(string concept, double similarity)>();
            using var reader = command.ExecuteReader();
            while (reader.Read())
            {
                var concept = reader.GetString(0);


                if (reader.IsDBNull(1))
                {
                    continue;
                }

                var distance = reader.GetDouble(1);


                var similarity = 1.0 / (1.0 + Math.Max(0, distance));
                results.Add((concept, similarity));
            }

            if (!results.Any())
            {
                return $"No similar concepts found for '{conceptName}'. Run Sync to ensure concept embeddings are generated.";
            }

            var sb = new StringBuilder();
            sb.AppendLineInvariant($"Similar concepts to '{conceptName}' (by semantic similarity):\n");

            foreach (var (concept, similarity) in results)
            {
                sb.AppendLineInvariant($"  • {concept} (similarity: {similarity:F3})");
            }

            return ToolResponse.WithNextSteps(
                sb.ToString(),
                "BuildContext", "SearchMemories"
            );
        }
        catch (Exception ex)
        {
            return $"Error finding similar concepts: {ex.Message}. Ensure vector extension is available and run Sync to generate concept embeddings.";
        }
    }
}
