using ModelContextProtocol.Server;
using System.ComponentModel;
using System.Globalization;
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
        if (learn) return ToolHelpers.GetLearnContent(nameof(FindSimilarConcepts));

        try
        {
            // T-QUAL-FSC2: RTM FR-7.4
            const int MaxResultsMax_T_QUAL_FSC2 = 50;
            if (maxResults <= 0 || maxResults > MaxResultsMax_T_QUAL_FSC2)
            {
                return "ERROR: maxResults must be between 1 and 50";
            }

            if (string.IsNullOrWhiteSpace(conceptName))
            {
                return "ERROR: conceptName parameter is required and cannot be empty";
            }

            // T-QUAL-FSC2: RTM FR-7.4
            // Hard cap to prevent output amplification and pathological scanning/normalization on huge inputs.
            const int MaxConceptNameLength_T_QUAL_FSC2 = 256;
            if (conceptName.Length > MaxConceptNameLength_T_QUAL_FSC2)
            {
                return $"ERROR: conceptName must be {MaxConceptNameLength_T_QUAL_FSC2} characters or fewer";
            }

            // T-QUAL-FSC2: RTM FR-7.4
            // Preserve user input for diagnostics; normalization may collapse tokens.
            var originalConceptName = conceptName;

            // T-QUAL-FSC2: RTM FR-7.4
            // NFKC normalization (validation-only) closes Unicode bracket confusables.
            var conceptNameNfkcForValidation = conceptName.Normalize(NormalizationForm.FormKC);
            if (conceptNameNfkcForValidation.Contains('[', StringComparison.Ordinal) || conceptNameNfkcForValidation.Contains(']', StringComparison.Ordinal))
            {
                return ToolResponse.WithNextSteps(
                    $"DIAGNOSTIC: conceptName '{originalConceptName}' contains bracket characters (or Unicode confusables). Remove brackets and try again.",
                    "BuildContext", "SearchMemories"
                );
            }

            // T-QUAL-FSC2: RTM FR-7.4
            // Reject other bracket-like delimiters to prevent malformed concept-token lookups and odd embedding behavior.
            if (ContainsBracketLikeDelimiter_T_QUAL_FSC2(conceptNameNfkcForValidation))
            {
                return ToolResponse.WithNextSteps(
                    $"DIAGNOSTIC: conceptName '{originalConceptName}' contains bracket-like delimiter characters. Remove them and try again.",
                    "BuildContext", "SearchMemories"
                );
            }

            // T-QUAL-FSC2: RTM FR-7.4
            // Reject low-information queries (punctuation-only, marks-only, etc.).
            if (IsLowInformationQuery_T_QUAL_FSC2(conceptNameNfkcForValidation))
            {
                return ToolResponse.WithNextSteps(
                    $"DIAGNOSTIC: conceptName '{originalConceptName}' is too low-information to embed reliably. Provide a few letters or words.",
                    "BuildContext", "SearchMemories"
                );
            }

            conceptName = MarkdownIO.NormalizeConcept(conceptName);
            if (string.IsNullOrWhiteSpace(conceptName))
            {
                return ToolResponse.WithNextSteps(
                    $"DIAGNOSTIC: conceptName '{originalConceptName}' normalizes to empty. Provide a few letters or words.",
                    "BuildContext", "SearchMemories"
                );
            }


            var queryEmbedding = VectorTools.GenerateEmbedding(conceptName);
            // T-QUAL-FSC2: RTM FR-7.4
            if (!IsValidEmbeddingForSearch_T_QUAL_FSC2(queryEmbedding))
            {
                return ToolResponse.WithNextSteps(
                    $"DIAGNOSTIC: unable to generate a valid embedding for '{originalConceptName}' (normalized: '{conceptName}').",
                    "BuildContext", "SearchMemories"
                );
            }

            var embeddingBlob = VectorTools.ToSqliteVectorBlob(queryEmbedding);
            if (embeddingBlob.Length == 0)
            {
                return ToolResponse.WithNextSteps(
                    $"DIAGNOSTIC: unable to generate embedding for '{originalConceptName}' (normalized: '{conceptName}').",
                    "BuildContext", "SearchMemories"
                );
            }


            using var connection = new SqliteConnection(Config.DatabaseConnectionString);
            connection.OpenReadOnlyWithVector();

            var sql = @"
                SELECT concept_name, vec_distance_cosine(embedding, @embedding) as distance
                FROM vec_concepts
                WHERE concept_name != @concept
                  AND concept_name != ''
                  AND instr(concept_name, '[') = 0
                  AND instr(concept_name, ']') = 0
                ORDER BY distance
                LIMIT @maxResults";

            using var command = new SqliteCommand(sql, connection);
            command.Parameters.Add("@embedding", SqliteType.Blob).Value = embeddingBlob;
            command.Parameters.AddWithValue("@concept", conceptName);
            command.Parameters.Add("@maxResults", SqliteType.Integer).Value = maxResults;

            var results = new List<(string concept, double distance, double similarity)>();
            var seenConcepts = new HashSet<string>(StringComparer.Ordinal);
            using var reader = command.ExecuteReader();
            while (reader.Read())
            {
                var concept = reader.GetString(0);


                if (reader.IsDBNull(1))
                {
                    continue;
                }

                var distance = reader.GetDouble(1);

                // T-QUAL-FSC2: RTM FR-7.4
                // Regression guard: skip invalid distance rows (NULL/NaN/Infinity) to avoid bogus similarity output.
                if (double.IsNaN(distance) || double.IsInfinity(distance))
                {
                    continue;
                }


                // T-QUAL-FSC2: RTM FR-7.4
                // Post-filter and normalize any concept names from the DB before emitting.
                var conceptNfkc = concept.Normalize(NormalizationForm.FormKC);
                if (conceptNfkc.Contains('[', StringComparison.Ordinal) || conceptNfkc.Contains(']', StringComparison.Ordinal))
                {
                    continue;
                }

                // T-QUAL-FSC2: RTM FR-7.4
                // Also drop bracket-like delimiter variants that can pollute result lists.
                if (ContainsBracketLikeDelimiter_T_QUAL_FSC2(conceptNfkc))
                {
                    continue;
                }

                // T-QUAL-FSC2: RTM FR-7.4
                // Avoid emitting concepts that normalize to the query concept.
                if (string.Equals(conceptNfkc, conceptName, StringComparison.Ordinal))
                {
                    continue;
                }

                // T-QUAL-FSC2: RTM FR-7.4
                // De-duplicate after normalization to avoid confusing repeated results.
                if (!seenConcepts.Add(conceptNfkc))
                {
                    continue;
                }


                // T-QUAL-FSC2: RTM FR-7.4
                // Guard against negative distances inflating similarity above 1.000.
                var similarity = 1.0 / (1.0 + Math.Max(0, distance));
                similarity = Math.Clamp(similarity, 0.0, 1.0);
                results.Add((conceptNfkc, distance, similarity));
            }

            if (!results.Any())
            {
                return ToolResponse.WithNextSteps(
                    $"No similar concepts found for '{conceptName}'. Run Sync to ensure concept embeddings are generated.",
                    "BuildContext", "SearchMemories"
                );
            }

            // T-QUAL-FSC2: RTM FR-7.4
            // Detect suspicious plateaus (many near-identical scores/distances) that indicate embedding/distance failure.
            if (IsSimilarityPlateau_T_QUAL_FSC2(results))
            {
                return ToolResponse.WithNextSteps(
                    $"DIAGNOSTIC: similarity plateau detected for '{originalConceptName}'. This can happen when embeddings are invalid or distance calculations collapse. Try a more specific query.",
                    "BuildContext", "SearchMemories"
                );
            }

            // T-GRAPH-DECAY-001.1: RTM FR-7.5, NFR-7.5.4 - Apply decay weighting to concept similarity scores
            // Get decay weight for each concept based on its most recent source file
            var decayedResults = ApplyDecayToConceptResults_T_GRAPH_DECAY_001_1(results);

            var sb = new StringBuilder();
            sb.AppendLineInvariant($"Similar concepts to '{conceptName}' (by semantic similarity):\n");

            foreach (var (concept, decayedSimilarity) in decayedResults)
            {
                sb.AppendLineInvariant($"  • {concept} (similarity: {decayedSimilarity:F3})");
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

    // T-QUAL-FSC2: RTM FR-7.4
    private static bool ContainsBracketLikeDelimiter_T_QUAL_FSC2(string input)
    {
        // Block common bracket-like delimiters seen in Unicode-bypass attempts.
        // Note: ASCII '[' / ']' are handled separately (including NFKC confusables).
        return input.EnumerateRunes().Where(rune => rune.Value is
            // ASCII angle brackets
            (int)'<' or (int)'>' or
            // CJK corner/white/ornamental brackets
            0x300A or 0x300B or // 《 》
            0x3008 or 0x3009 or // 〈 〉
            0x3010 or 0x3011 or // 【 】
            0x300C or 0x300D or // 「 」
            0x300E or 0x300F or // 『 』
            0x3014 or 0x3015 or // 〔 〕
            0x3016 or 0x3017 or // 〖 〗
            // Mathematical white square brackets
            0x27E6 or 0x27E7 or // ⟦ ⟧
            0x27E8 or 0x27E9 or // ⟨ ⟩
            0x27EA or 0x27EB or // ⟪ ⟫
            0x27EC or 0x27ED or // ⟬ ⟭
            0x27EE or 0x27EF or // ⟮ ⟯
            // Dingbat brackets
            0x2772 or 0x2773 or // ❲ ❳
            0x276E or 0x276F)   // ❮ ❯
            .Any();
    }

    // T-QUAL-FSC2: RTM FR-7.4
    private static bool IsLowInformationQuery_T_QUAL_FSC2(string input)
    {
        var hasLetterOrDigit = false;
        var hasAnyNonIgnored = false;
        var hasNonSpacingMark = false;
        var hasNonMarkNonIgnored = false;

        foreach (var rune in input.EnumerateRunes())
        {
            var category = Rune.GetUnicodeCategory(rune);
            if (IsIgnoredForInformation_T_QUAL_FSC2(rune, category))
                continue;

            hasAnyNonIgnored = true;

            if (Rune.IsLetterOrDigit(rune))
            {
                hasLetterOrDigit = true;
                break;
            }

            if (category == UnicodeCategory.NonSpacingMark)
                hasNonSpacingMark = true;
            else
                hasNonMarkNonIgnored = true;
        }

        if (hasLetterOrDigit)
            return false;

        // If nothing besides whitespace/control/format was present, this is low-info.
        if (!hasAnyNonIgnored)
            return true;

        // No letters/digits => low-info (includes emoji-only, punctuation-only, symbols-only, etc.).
        // Explicitly covers combining-marks-only queries (NonSpacingMark-only).
        _ = hasNonSpacingMark;
        _ = hasNonMarkNonIgnored;
        return true;
    }

    // T-QUAL-FSC2: RTM FR-7.4
    private static bool IsIgnoredForInformation_T_QUAL_FSC2(Rune rune, UnicodeCategory category)
    {
        if (Rune.IsWhiteSpace(rune))
            return true;

        return category is UnicodeCategory.Control or UnicodeCategory.Format;
    }

    // T-QUAL-FSC2: RTM FR-7.4
    private static bool IsValidEmbeddingForSearch_T_QUAL_FSC2(float[] embedding)
    {
        if (embedding.Length != VectorTools.EmbeddingLength)
            return false;

        var sumSq = 0.0;
        for (int i = 0; i < embedding.Length; i++)
        {
            var v = embedding[i];
            if (float.IsNaN(v) || float.IsInfinity(v))
                return false;
            sumSq += v * v;
        }

        // Near-zero embeddings often lead to degenerate cosine distances.
        return sumSq > 1e-12;
    }

    // T-QUAL-FSC2: RTM FR-7.4
    private static bool IsSimilarityPlateau_T_QUAL_FSC2(List<(string concept, double distance, double similarity)> results)
    {
        var topK = Math.Min(results.Count, 10);
        if (topK < 5)
            return false;

        const double NearZero = 1e-12;
        var nearZeroCount = 0;
        var min = double.MaxValue;
        var max = double.MinValue;
        for (int i = 0; i < topK; i++)
        {
            var d = results[i].distance;
            // T-QUAL-FSC2: RTM FR-7.4
            // Plateau detection: treat +/- epsilon as near-zero (negative distances can occur due to numeric drift).
            if (Math.Abs(d) <= NearZero)
                nearZeroCount++;
            if (d < min) min = d;
            if (d > max) max = d;
        }

        // Plateau if most distances are near 0, or if there's effectively no variance.
        if (nearZeroCount >= topK - 1)
            return true;

        return (max - min) <= 1e-9;
    }

    // T-GRAPH-DECAY-001.1: RTM FR-7.5, NFR-7.5.4 - Apply decay weighting to concept similarity results
    /// <summary>
    /// Applies decay weighting to concept similarity results based on the freshness of source files.
    /// Each concept's decay weight is determined by the most recent file containing that concept.
    /// </summary>
    /// <param name="results">Raw similarity results from vector search</param>
    /// <returns>Re-sorted results with decay-weighted similarity scores</returns>
    private static List<(string concept, double decayedSimilarity)> ApplyDecayToConceptResults_T_GRAPH_DECAY_001_1(
        List<(string concept, double distance, double similarity)> results)
    {
        var decayedResults = new List<(string concept, double decayedSimilarity)>();

        foreach (var (concept, _, similarity) in results)
        {
            // Get decay weight for this concept based on its source files
            var decayWeight = GetConceptDecayWeight_T_GRAPH_DECAY_001_1(concept);
            var decayedSimilarity = similarity * decayWeight;
            decayedResults.Add((concept, decayedSimilarity));
        }

        // Re-sort by decay-weighted similarity (descending)
        return decayedResults
            .OrderByDescending(r => r.decayedSimilarity)
            .ToList();
    }

    // T-GRAPH-DECAY-001.1: RTM FR-7.5 - Get decay weight for a concept
    /// <summary>
    /// Calculates decay weight for a concept based on its source files.
    /// Uses the maximum decay weight (newest file) among all files containing the concept.
    /// Queries concept_mentions table which directly maps concepts to their source files.
    /// </summary>
    /// <param name="conceptName">Normalized concept name</param>
    /// <returns>Decay weight between 0.0 and 1.0 (1.0 = no decay, recent content)</returns>
    private static double GetConceptDecayWeight_T_GRAPH_DECAY_001_1(string conceptName)
    {
        try
        {
            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenReadOnly();

            // T-GRAPH-DECAY-001.1: RTM FR-7.5 - Query concept_mentions for direct concept-to-file mapping
            // This is more reliable than concept_graph which only stores concept pairs
            var sourceFiles = new List<string>();
            using var cmd = new SqliteCommand(
                "SELECT source_file FROM concept_mentions WHERE concept_name = @concept",
                conn);
            cmd.Parameters.AddWithValue("@concept", conceptName);
            using var reader = cmd.ExecuteReader();
            while (reader.Read())
            {
                if (!reader.IsDBNull(0))
                {
                    sourceFiles.Add(reader.GetString(0));
                }
            }

            if (sourceFiles.Count == 0)
            {
                // Concept not in graph, return default weight (no decay)
                return 1.0;
            }

            // T-GRAPH-DECAY-001.1: RTM FR-7.5 - Calculate decay weight for each source file and take the maximum
            // (concept's freshness is determined by its most recent occurrence)
            var maxDecayWeight = 0.0;
            foreach (var fileUri in sourceFiles)
            {
                var decayWeight = MemorySearchTools.GetDecayWeightForFile(fileUri);
                if (decayWeight > maxDecayWeight)
                {
                    maxDecayWeight = decayWeight;
                }
            }

            return maxDecayWeight > 0 ? maxDecayWeight : 1.0;
        }
        catch
        {
            // On error, return default weight (no decay)
            return 1.0;
        }
    }
}
