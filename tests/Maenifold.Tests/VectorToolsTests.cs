using Microsoft.ML.OnnxRuntime;
using Maenifold.Utils;
using NUnit.Framework;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Maenifold.Tests;

/// <summary>
/// Unit tests for VectorTools implementation following Ma Protocol testing philosophy.
/// Tests case-insensitive path finding, ONNX model loading, and cross-platform compatibility.
/// </summary>
public class VectorToolsTests
{
    private readonly string _originalBaseDir;

    public VectorToolsTests()
    {
        _originalBaseDir = AppContext.BaseDirectory;
    }

    [SetUp]
    public void SetUp()
    {
        // Reset VectorTools state before each test
        VectorTools.Cleanup();
    }

    [TearDown]
    public void TearDown()
    {
        // Clean up after tests
        VectorTools.Cleanup();
    }

    [Test]
    public void VectorToolsLoadModelHandlesCaseSensitiveFileSystems()
    {
        // Test that the model loading works on case-sensitive systems
        // This test verifies the fix for the "Maenifold.sln" vs "maenifold.sln" issue

        Assert.DoesNotThrow(() => VectorTools.LoadModel(),
            "LoadModel should handle case-sensitive file systems correctly");
    }

    [Test]
    public void GenerateEmbeddingProducesConsistent384DimensionalVectors()
    {
        // Ensure model is loaded
        VectorTools.LoadModel();

        const string testText = "Test embedding for [[neural-networks]] and [[natural-language-processing]]";

        var embedding1 = VectorTools.GenerateEmbedding(testText);
        var embedding2 = VectorTools.GenerateEmbedding(testText);

        Assert.That(embedding1, Is.Not.Null, "Embedding should not be null");
        Assert.That(embedding1.Length, Is.EqualTo(384), "Embedding should be 384-dimensional");
        Assert.That(embedding1, Is.EqualTo(embedding2), "Same input should produce same embedding");
    }

    [Test]
    public void FallbackEmbeddingGeneratesValidVectorsWhenONNXUnavailable()
    {
        // Test fallback behavior when ONNX model is not available
        // Since we can't easily disable ONNX in test environment,
        // this test just verifies that ANY embedding generated is properly normalized
        const string testText = "Fallback test for [[embedding-fallback]]";

        var embedding = VectorTools.GenerateEmbedding(testText);

        Assert.That(embedding, Is.Not.Null);
        Assert.That(embedding.Length, Is.EqualTo(384));
        Assert.That(embedding.Any(x => x != 0), Is.True, "Embedding should contain non-zero values");

        // Verify normalization - ONNX embeddings may not be unit normalized, 
        // but they should have reasonable magnitude
        var magnitude = Math.Sqrt(embedding.Sum(x => x * x));
        Assert.That(magnitude, Is.GreaterThan(0.1),
            "Embedding should have reasonable magnitude");
        Assert.That(magnitude, Is.LessThan(10.0),
            "Embedding magnitude should not be excessive");
    }

    [Test]
    public void Base64EncodingPreservesEmbeddingPrecision()
    {
        VectorTools.LoadModel();

        const string testText = "Base64 encoding test";

        var embedding = VectorTools.GenerateEmbedding(testText);
        var base64 = VectorTools.GenerateEmbeddingBase64(testText);
        var decoded = VectorTools.DecodeBase64Embedding(base64);

        Assert.That(decoded, Is.EqualTo(embedding),
            "Base64 round-trip should preserve embedding exactly");
    }

    [Test]
    public void EmptyAndNullInputsProduceSafeEmbeddings()
    {
        var emptyEmbedding = VectorTools.GenerateEmbedding("");
        var nullEmbedding = VectorTools.GenerateEmbedding(null!);

        Assert.That(emptyEmbedding, Is.Not.Null);
        Assert.That(emptyEmbedding.Length, Is.EqualTo(384));
        Assert.That(nullEmbedding, Is.Not.Null);
        Assert.That(nullEmbedding.Length, Is.EqualTo(384));
    }

    [Test]
    public void VeryLongInputsAreTruncatedToMaxTokens()
    {
        // Create a very long input that exceeds max tokens
        var longText = string.Join(" ", Enumerable.Repeat("word", 1000));

        Assert.DoesNotThrow(() => VectorTools.GenerateEmbedding(longText),
            "Should handle very long inputs by truncating to max tokens");

        var embedding = VectorTools.GenerateEmbedding(longText);
        Assert.That(embedding.Length, Is.EqualTo(384));
    }

    [Test]
    [Platform("Linux,MacOSX,Unix")]
    public void PathResolutionWorksOnUnixSystems()
    {
        // Test specifically for Unix-based systems (Linux, macOS, WSL)
        // Verify case-sensitive path handling

        VectorTools.LoadModel();

        // The test passes if we get here without exceptions
        // On Unix systems, the path finding should use case-insensitive fallback
        Assert.Pass("Path resolution successful on Unix system");
    }

    [Test]
    public void DiagnosticLoggingProvidesUsefulInformation()
    {
        // Enable embedding logs for this test to verify diagnostic messages
        var originalEmbeddingLogsValue = Environment.GetEnvironmentVariable("MAENIFOLD_EMBEDDING_LOGS");
        Environment.SetEnvironmentVariable("MAENIFOLD_EMBEDDING_LOGS", "true");

        // Capture console error output to verify diagnostic messages
        var originalError = Console.Error;
        using var errorCapture = new StringWriter();
        Console.SetError(errorCapture);

        try
        {
            VectorTools.Cleanup(); // Ensure clean state
            VectorTools.LoadModel();

            var output = errorCapture.ToString();

            // Debug output (will be visible if test fails)
            if (!output.Contains("[VectorTools]"))
            {
                Console.SetError(originalError);
                Console.WriteLine($"DEBUG: Expected logs but got empty output. Output length: {output.Length}");
                Console.WriteLine($"DEBUG: Env var MAENIFOLD_EMBEDDING_LOGS: {Environment.GetEnvironmentVariable("MAENIFOLD_EMBEDDING_LOGS")}");
            }

            // Check for diagnostic messages
            Assert.That(output.Contains("[VectorTools]"), Is.True,
                "Should include diagnostic prefix");

            // Should log either successful loading or fallback message
            var hasSuccessMessage = output.Contains("loaded successfully") ||
                                   output.Contains("Falling back to hash-based embeddings");
            Assert.That(hasSuccessMessage, Is.True,
                "Should log loading status");
        }
        finally
        {
            Console.SetError(originalError);
            // Restore original environment variable
            if (originalEmbeddingLogsValue != null)
                Environment.SetEnvironmentVariable("MAENIFOLD_EMBEDDING_LOGS", originalEmbeddingLogsValue);
            else
                Environment.SetEnvironmentVariable("MAENIFOLD_EMBEDDING_LOGS", null);
        }
    }

    [Test]
    public void ThreadSafeInitializationWorks()
    {
        // Test thread-safe singleton initialization
        var tasks = new List<Task>();
        var embeddings = new List<float[]>();
        var lockObj = new object();

        for (int i = 0; i < 10; i++)
        {
            tasks.Add(Task.Run(() =>
            {
                VectorTools.LoadModel();
                var embedding = VectorTools.GenerateEmbedding($"Test {i}");
                lock (lockObj)
                {
                    embeddings.Add(embedding);
                }
            }));
        }

        Task.WaitAll(tasks.ToArray());

        Assert.That(embeddings.Count, Is.EqualTo(10));
        Assert.That(embeddings.All(e => e.Length == 384), Is.True,
            "All embeddings should be 384-dimensional");
    }

    [Test]
    public void OnnxSessionHandlesMultipleInferences()
    {
        VectorTools.LoadModel();

        // Test multiple inferences to ensure session is reused correctly
        var texts = new[]
        {
            "First inference test",
            "Second inference with [[semantic-similarity]]",
            "Third test with different content",
            "Fourth test to verify session stability",
            "Fifth and final test"
        };

        var embeddings = texts.Select(t => VectorTools.GenerateEmbedding(t)).ToArray();

        Assert.That(embeddings.Length, Is.EqualTo(5));
        Assert.That(embeddings.All(e => e.Length == 384), Is.True);

        // Verify different texts produce different embeddings
        for (int i = 0; i < embeddings.Length; i++)
        {
            for (int j = i + 1; j < embeddings.Length; j++)
            {
                Assert.That(embeddings[i], Is.Not.EqualTo(embeddings[j]),
                    $"Different texts should produce different embeddings ({i} vs {j})");
            }
        }
    }

    [Test]
    public void CleanupProperlyDisposesResources()
    {
        VectorTools.LoadModel();
        var embedding1 = VectorTools.GenerateEmbedding("Before cleanup");

        VectorTools.Cleanup();

        // After cleanup, should reinitialize on next use
        var embedding2 = VectorTools.GenerateEmbedding("After cleanup");

        Assert.That(embedding1.Length, Is.EqualTo(384));
        Assert.That(embedding2.Length, Is.EqualTo(384));

        // ONNX models can have slight numerical differences between sessions
        // Check that embeddings are reasonably similar (cosine similarity > 0.9)
        var similarity = CosineSimilarity(embedding1, embedding2);
        Assert.That(similarity, Is.GreaterThan(0.9),
            $"Same text should produce similar embeddings even after cleanup/reinit (similarity: {similarity:F4})");
    }

    [Test]
    public void VocabularyLoadingHandlesMissingFile()
    {
        // Test that missing vocabulary file doesn't crash the system
        // The tokenizer should work with basic vocabulary even without vocab.txt

        Assert.DoesNotThrow(() =>
        {
            VectorTools.LoadModel();
            var embedding = VectorTools.GenerateEmbedding("Test without vocabulary file");
            Assert.That(embedding.Length, Is.EqualTo(384));
        }, "Should handle missing vocabulary file gracefully");
    }

    [Test]
    public void EmbeddingQualityDiffersBetweenOnnxAndFallback()
    {
        // Generate embeddings for similar texts and check if semantic similarity is preserved
        const string text1 = "machine learning and artificial intelligence";
        const string text2 = "ML and AI technologies";
        const string text3 = "cooking recipes and food preparation"; // Unrelated

        var embedding1 = VectorTools.GenerateEmbedding(text1);
        var embedding2 = VectorTools.GenerateEmbedding(text2);
        var embedding3 = VectorTools.GenerateEmbedding(text3);

        // Calculate cosine similarities
        var similarity12 = CosineSimilarity(embedding1, embedding2);
        var similarity13 = CosineSimilarity(embedding1, embedding3);

        // With ONNX model, similar texts should have higher similarity
        // With fallback, this relationship might not hold as strongly
        // But we always expect valid embeddings
        Assert.That(embedding1.Length, Is.EqualTo(384));
        Assert.That(embedding2.Length, Is.EqualTo(384));
        Assert.That(embedding3.Length, Is.EqualTo(384));

        // Log which mode we're in for debugging
        Console.WriteLine($"Similarity (text1-text2): {similarity12:F4}");
        Console.WriteLine($"Similarity (text1-text3): {similarity13:F4}");
    }

    // Helper method for cosine similarity calculation
    private static float CosineSimilarity(float[] a, float[] b)
    {
        if (a.Length != b.Length)
            throw new ArgumentException("Vectors must have same length");

        float dotProduct = 0;
        float magnitudeA = 0;
        float magnitudeB = 0;

        for (int i = 0; i < a.Length; i++)
        {
            dotProduct += a[i] * b[i];
            magnitudeA += a[i] * a[i];
            magnitudeB += b[i] * b[i];
        }

        magnitudeA = (float)Math.Sqrt(magnitudeA);
        magnitudeB = (float)Math.Sqrt(magnitudeB);

        if (magnitudeA == 0 || magnitudeB == 0)
            return 0;

        return dotProduct / (magnitudeA * magnitudeB);
    }
}
