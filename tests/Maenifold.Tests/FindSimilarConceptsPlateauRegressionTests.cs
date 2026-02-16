#pragma warning disable CA1861
using Maenifold.Tools;
using NUnit.Framework;
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Maenifold.Utils;

namespace Maenifold.Tests;

[TestFixture]
[NonParallelizable]
public class FindSimilarConceptsPlateauRegressionTests
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
        _testRoot = Path.Combine(repoRoot, "test-outputs", "fsc2-plateau-regression", $"run-{Guid.NewGuid():N}");
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
                    f.Attributes = FileAttributes.Normal;
                Directory.Delete(_testRoot, recursive: true);
            }
        }
        catch { }

        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT",
            string.IsNullOrEmpty(_previousMaenifoldRootEnv) ? null : _previousMaenifoldRootEnv);
        Environment.SetEnvironmentVariable("MAENIFOLD_DATABASE_PATH",
            string.IsNullOrEmpty(_previousDatabasePathEnv) ? null : _previousDatabasePathEnv);

        if (string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv)
            && string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
        {
            Config.ResetOverrides();
        }
        else
        {
            if (!string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv))
                Config.OverrideRoot(_previousMaenifoldRootEnv);
            if (!string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
                Config.SetDatabasePath(_previousDatabasePathEnv);
        }
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
    }

    [Test]
    public void FindSimilarConcepts_WhenQueryIsPunctuationOnly_DoesNotEmitConfidentResults()
    {
        // T-QUAL-FSC2: RTM FR-7.4
        // Regression guard: garbage inputs like "----" should not produce a similarity list.
        var result = VectorSearchTools.FindSimilarConcepts("----");

        Assert.That(result, Does.Contain("DIAGNOSTIC:"));
        Assert.That(result, Does.Contain("too low-information"));
        Assert.That(result, Does.Not.Contain("Similar concepts to"));
        Assert.That(result, Does.Contain("Next: Consider using BuildContext, SearchMemories"));
    }

    [Test]
    public void FindSimilarConcepts_WhenQueryContainsUnicodeBracketConfusables_RejectsWithHelpfulDiagnostic()
    {
        // T-QUAL-FSC2: RTM FR-7.4
        // Fullwidth brackets normalize (NFKC) to ASCII brackets and are rejected.
        var result = VectorSearchTools.FindSimilarConcepts("［［tool］］");

        Assert.That(result, Does.Contain("DIAGNOSTIC:"));
        Assert.That(result, Does.Contain("contains bracket characters"));
        Assert.That(result, Does.Not.Contain("Similar concepts to"));
    }

    [Test]
    public void FindSimilarConcepts_WhenQueryContainsBracketLikeDelimiters_RejectsWithHelpfulDiagnostic()
    {
        // T-QUAL-FSC2: RTM FR-7.4
        // Regression guard: bracket-like delimiters must not bypass bracket checks.
        var result = VectorSearchTools.FindSimilarConcepts("⟦tool⟧");

        Assert.That(result, Does.Contain("DIAGNOSTIC:"));
        Assert.That(result, Does.Contain("bracket-like delimiter"));
        Assert.That(result, Does.Not.Contain("Similar concepts to"));
    }

    [Test]
    public void FindSimilarConcepts_WhenConceptNameTooLong_ReturnsValidationError()
    {
        // T-QUAL-FSC2: RTM FR-7.4
        var tooLong = new string('a', 257);
        var result = VectorSearchTools.FindSimilarConcepts(tooLong);

        Assert.That(result, Is.EqualTo("ERROR: conceptName must be 256 characters or fewer"));
    }

    [Test]
    public void FindSimilarConcepts_WhenMaxResultsOutOfRange_ReturnsValidationError()
    {
        // T-QUAL-FSC2: RTM FR-7.4
        var resultLow = VectorSearchTools.FindSimilarConcepts("test", maxResults: 0);
        var resultHigh = VectorSearchTools.FindSimilarConcepts("test", maxResults: 51);

        Assert.That(resultLow, Is.EqualTo("ERROR: maxResults must be between 1 and 50"));
        Assert.That(resultHigh, Is.EqualTo("ERROR: maxResults must be between 1 and 50"));
    }

    [Test]
    public void FindSimilarConcepts_WhenItReturnsResults_NeverEmitsSimilarityGreaterThanOne()
    {
        // T-QUAL-FSC2: RTM FR-7.4
        // Regression guard: even if vec_distance_cosine returns negative distances (numeric drift),
        // similarity must never exceed 1.000.
        //
        // NOTE: We do not attempt to seed negative distance rows directly because sqlite-vec's distance
        // function behavior may not allow constructing such a case. Instead, we seed the vector table and
        // enforce the output invariant.

        using (var connection = new Microsoft.Data.Sqlite.SqliteConnection(Maenifold.Utils.Config.DatabaseConnectionString))
        {
            connection.Open();
            connection.LoadVectorExtension();

            // Seed at least a couple concept embeddings to guarantee a similarity list is produced.
            SeedVecConcept(connection, "alpha");
            SeedVecConcept(connection, "beta");
            SeedVecConcept(connection, "gamma");
        }

        var result = VectorSearchTools.FindSimilarConcepts("test", maxResults: 10);
        Assert.That(result, Does.Contain("Similar concepts to"));

        var similarityValues = ParseSimilarityValues(result);
        Assert.That(similarityValues, Is.Not.Empty, "Expected similarity values to be emitted when results are returned.");

        foreach (var similarity in similarityValues)
        {
            Assert.That(similarity, Is.LessThanOrEqualTo(1.0), $"Similarity must be <= 1.000, got {similarity.ToString("F6", CultureInfo.InvariantCulture)}. Output:\n{result}");
        }
    }

    private static void SeedVecConcept(Microsoft.Data.Sqlite.SqliteConnection connection, string concept)
    {
        // T-QUAL-FSC2: RTM FR-7.4
        // vec0 virtual tables do not support INSERT OR REPLACE; use DELETE-then-INSERT.
        var embedding = Maenifold.Utils.VectorTools.GenerateEmbedding(concept);
        var blob = Maenifold.Utils.VectorTools.ToSqliteVectorBlob(embedding);

        using var delCmd = connection.CreateCommand();
        delCmd.CommandText = "DELETE FROM vec_concepts WHERE concept_name = @concept";
        delCmd.Parameters.AddWithValue("@concept", concept);
        delCmd.ExecuteNonQuery();

        using var insCmd = connection.CreateCommand();
        insCmd.CommandText = "INSERT INTO vec_concepts (concept_name, embedding) VALUES (@concept, @embedding)";
        insCmd.Parameters.AddWithValue("@concept", concept);
        insCmd.Parameters.Add("@embedding", Microsoft.Data.Sqlite.SqliteType.Blob).Value = blob;
        insCmd.ExecuteNonQuery();
    }

    private static List<double> ParseSimilarityValues(string output)
    {
        // Expected line format:
        //   • some-concept (similarity: 0.123)

        var values = new List<double>();
        using var reader = new StringReader(output);
        string? line;
        while ((line = reader.ReadLine()) != null)
        {
            var idx = line.IndexOf("similarity:", StringComparison.Ordinal);
            if (idx < 0)
                continue;

            var start = idx + "similarity:".Length;
            // Trim and parse until we hit a closing paren or whitespace.
            var span = line.AsSpan(start).TrimStart();
            var end = span.IndexOf(')');
            if (end < 0)
                continue;

            var number = span.Slice(0, end).Trim().ToString();
            if (double.TryParse(number, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed))
            {
                values.Add(parsed);
            }
        }

        return values;
    }
}

#pragma warning restore CA1861
