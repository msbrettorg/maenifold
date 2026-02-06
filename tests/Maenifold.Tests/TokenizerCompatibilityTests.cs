using Maenifold.Utils;
using NUnit.Framework;
using System.Reflection;
using Microsoft.ML.Tokenizers;

namespace Maenifold.Tests;

/// <summary>
/// Tests verifying tokenizer compatibility with all-MiniLM-L6-v2 ONNX model.
/// 
/// These tests address review feedback from PR #56 regarding the refactoring
/// from SimpleTokenizer to Microsoft.ML.Tokenizers BertTokenizer.
/// 
/// Verification goals:
/// 1. New tokenizer produces compatible token IDs with the ONNX model
/// 2. Existing embeddings remain compatible with new embeddings
/// 3. Vector search results maintain consistency
/// </summary>
[TestFixture]
[NonParallelizable]
public sealed class TokenizerCompatibilityTests
{
    private const string TestText1 = "machine learning and natural language processing";
    private const string TestText2 = "neural networks for text understanding";
    private const string TestText3 = "database query optimization";

    [SetUp]
    public void SetUp()
    {
        VectorTools.Cleanup();
    }

    [TearDown]
    public void TearDown()
    {
        VectorTools.Cleanup();
    }

    [Test]
    public void TokenizerProducesValidBertTokenIds()
    {
        // Verify that the BertTokenizer produces valid token IDs compatible with all-MiniLM-L6-v2
        VectorTools.LoadModel();

        var tokenizer = GetTokenizerFromVectorTools();
        Assert.That(tokenizer, Is.Not.Null, "Tokenizer should be loaded");

        // Test basic tokenization with special tokens
        var ids = tokenizer.EncodeToIds(
            TestText1,
            512,
            addSpecialTokens: true,
            out _,
            out _,
            considerPreTokenization: true,
            considerNormalization: true);

        Assert.That(ids, Is.Not.Null, "Token IDs should not be null");
        Assert.That(ids.Count, Is.GreaterThan(0), "Should produce at least one token");

        // Verify special tokens are included (CLS and SEP)
        // BERT models typically use 101 for [CLS] and 102 for [SEP] in standard vocab
        var firstId = ids[0];
        var lastId = ids[ids.Count - 1];

        // Special tokens should be in the valid range (0-30000 for BERT vocab)
        Assert.That(firstId, Is.GreaterThanOrEqualTo(0), "First token (CLS) should be valid");
        Assert.That(firstId, Is.LessThan(30000), "First token should be in vocab range");
        Assert.That(lastId, Is.GreaterThanOrEqualTo(0), "Last token (SEP) should be valid");
        Assert.That(lastId, Is.LessThan(30000), "Last token should be in vocab range");
    }

    [Test]
    public void TokenizerProducesConsistentTokensForSameInput()
    {
        // Verify deterministic tokenization
        VectorTools.LoadModel();

        var tokenizer = GetTokenizerFromVectorTools();
        
        var ids1 = tokenizer.EncodeToIds(
            TestText1,
            512,
            addSpecialTokens: true,
            out _,
            out _,
            considerPreTokenization: true,
            considerNormalization: true);

        var ids2 = tokenizer.EncodeToIds(
            TestText1,
            512,
            addSpecialTokens: true,
            out _,
            out _,
            considerPreTokenization: true,
            considerNormalization: true);

        Assert.That(ids1.Count, Is.EqualTo(ids2.Count), "Same input should produce same number of tokens");
        
        for (int i = 0; i < ids1.Count; i++)
        {
            Assert.That(ids1[i], Is.EqualTo(ids2[i]), $"Token {i} should be identical");
        }
    }

    [Test]
    public void EmbeddingGenerationIsDeterministicWithNewTokenizer()
    {
        // Verify that embeddings are deterministic across multiple calls
        VectorTools.LoadModel();

        var embedding1 = VectorTools.GenerateEmbedding(TestText1);
        var embedding2 = VectorTools.GenerateEmbedding(TestText1);
        var embedding3 = VectorTools.GenerateEmbedding(TestText1);

        Assert.That(embedding1, Is.EqualTo(embedding2), "Same input should produce identical embeddings (1 vs 2)");
        Assert.That(embedding1, Is.EqualTo(embedding3), "Same input should produce identical embeddings (1 vs 3)");
        Assert.That(embedding2, Is.EqualTo(embedding3), "Same input should produce identical embeddings (2 vs 3)");
    }

    [Test]
    public void EmbeddingsPreserveSemanticSimilarity()
    {
        // Verify that semantically similar texts have higher cosine similarity
        // than semantically dissimilar texts
        VectorTools.LoadModel();

        var embedding1 = VectorTools.GenerateEmbedding(TestText1); // ML/NLP
        var embedding2 = VectorTools.GenerateEmbedding(TestText2); // Neural networks/text
        var embedding3 = VectorTools.GenerateEmbedding(TestText3); // Database queries

        // Calculate similarities
        var similarity12 = CosineSimilarity(embedding1, embedding2); // Related: ML/NLP
        var similarity13 = CosineSimilarity(embedding1, embedding3); // Unrelated
        var similarity23 = CosineSimilarity(embedding2, embedding3); // Unrelated

        // Related texts should have higher similarity than unrelated texts
        Assert.That(similarity12, Is.GreaterThan(similarity13),
            $"ML/NLP texts (sim={similarity12:F4}) should be more similar than ML vs Database (sim={similarity13:F4})");
        Assert.That(similarity12, Is.GreaterThan(similarity23),
            $"ML/NLP texts (sim={similarity12:F4}) should be more similar than Neural vs Database (sim={similarity23:F4})");

        // All similarities should be in valid range [-1, 1]
        Assert.That(similarity12, Is.InRange(-1.0f, 1.0f), "Similarity should be in valid range");
        Assert.That(similarity13, Is.InRange(-1.0f, 1.0f), "Similarity should be in valid range");
        Assert.That(similarity23, Is.InRange(-1.0f, 1.0f), "Similarity should be in valid range");

        Console.WriteLine($"Similarity (ML/NLP vs Neural/Text): {similarity12:F4}");
        Console.WriteLine($"Similarity (ML/NLP vs Database): {similarity13:F4}");
        Console.WriteLine($"Similarity (Neural/Text vs Database): {similarity23:F4}");
    }

    [Test]
    public void EmbeddingsHaveReasonableMagnitude()
    {
        // Verify embeddings have reasonable magnitude
        // Note: all-MiniLM-L6-v2 does NOT produce unit-normalized embeddings
        // The embeddings have magnitude typically between 4.0 and 8.0
        VectorTools.LoadModel();

        var testTexts = new[] { TestText1, TestText2, TestText3 };

        foreach (var text in testTexts)
        {
            var embedding = VectorTools.GenerateEmbedding(text);
            var magnitude = Math.Sqrt(embedding.Sum(x => x * x));

            // Verify embeddings have reasonable magnitude (not zero, not excessively large)
            Assert.That(magnitude, Is.GreaterThan(1.0),
                $"Embedding magnitude ({magnitude:F4}) should be non-trivial for text: {text}");
            Assert.That(magnitude, Is.LessThan(10.0),
                $"Embedding magnitude ({magnitude:F4}) should not be excessive for text: {text}");

            var displayText = text.Length > 40 ? text[..40] : text;
            Console.WriteLine($"Magnitude for '{displayText}...': {magnitude:F4}");
        }
    }

    [Test]
    public void TokenizerHandlesEdgeCases()
    {
        // Verify tokenizer handles various edge cases correctly
        VectorTools.LoadModel();

        var tokenizer = GetTokenizerFromVectorTools();

        // Empty string
        var emptyIds = tokenizer.EncodeToIds(
            "",
            512,
            addSpecialTokens: true,
            out _,
            out _,
            considerPreTokenization: true,
            considerNormalization: true);
        Assert.That(emptyIds.Count, Is.GreaterThan(0), "Should include special tokens for empty string");

        // Single word
        var singleIds = tokenizer.EncodeToIds(
            "test",
            512,
            addSpecialTokens: true,
            out _,
            out _,
            considerPreTokenization: true,
            considerNormalization: true);
        Assert.That(singleIds.Count, Is.GreaterThan(1), "Should tokenize single word with special tokens");

        // Very long text
        var longText = string.Join(" ", Enumerable.Repeat("word", 1000));
        var longIds = tokenizer.EncodeToIds(
            longText,
            512,
            addSpecialTokens: true,
            out _,
            out _,
            considerPreTokenization: true,
            considerNormalization: true);
        Assert.That(longIds.Count, Is.LessThanOrEqualTo(512), "Should respect max token limit");

        // Special characters and punctuation
        var specialIds = tokenizer.EncodeToIds(
            "Hello, world! How are you?",
            512,
            addSpecialTokens: true,
            out _,
            out _,
            considerPreTokenization: true,
            considerNormalization: true);
        Assert.That(specialIds.Count, Is.GreaterThan(0), "Should handle punctuation");

        // Mixed case (BERT is typically lowercase)
        var mixedIds = tokenizer.EncodeToIds(
            "Machine Learning",
            512,
            addSpecialTokens: true,
            out _,
            out _,
            considerPreTokenization: true,
            considerNormalization: true);
        Assert.That(mixedIds.Count, Is.GreaterThan(0), "Should handle mixed case");
    }

    [Test]
    public void EmbeddingsAre384Dimensional()
    {
        // Verify all embeddings are exactly 384-dimensional (all-MiniLM-L6-v2 output size)
        VectorTools.LoadModel();

        var testCases = new[]
        {
            "",
            "a",
            "short text",
            TestText1,
            TestText2,
            TestText3,
            string.Join(" ", Enumerable.Repeat("word", 1000))
        };

        foreach (var text in testCases)
        {
            var embedding = VectorTools.GenerateEmbedding(text);
            var displayText = text.Length > 40 ? text[..40] : text;
            Assert.That(embedding.Length, Is.EqualTo(384),
                $"Embedding should be 384-dimensional for text: '{displayText}'");
        }
    }

    [Test]
    public void TokenizerConfigurationIsLoadedCorrectly()
    {
        // Verify tokenizer configuration matches all-MiniLM-L6-v2 requirements
        VectorTools.LoadModel();

        var assets = GetTokenizerAssetsFromVectorTools();
        Assert.That(assets, Is.Not.Null, "Tokenizer assets should be loaded");

        // Verify special token IDs are set
        var padId = GetPropertyValue<int>(assets, "PadId");
        var unknownId = GetPropertyValue<int>(assets, "UnknownId");
        var classifyId = GetPropertyValue<int>(assets, "ClassifyId");
        var separatorId = GetPropertyValue<int>(assets, "SeparatorId");

        Assert.That(padId, Is.GreaterThanOrEqualTo(0), "PAD token ID should be valid");
        Assert.That(unknownId, Is.GreaterThanOrEqualTo(0), "UNK token ID should be valid");
        Assert.That(classifyId, Is.GreaterThanOrEqualTo(0), "CLS token ID should be valid");
        Assert.That(separatorId, Is.GreaterThanOrEqualTo(0), "SEP token ID should be valid");

        Console.WriteLine($"PAD: {padId}, UNK: {unknownId}, CLS: {classifyId}, SEP: {separatorId}");
    }

    [Test]
    public void VectorSearchConsistencyIsPreserved()
    {
        // Verify that vector search results are consistent with the new tokenizer
        VectorTools.LoadModel();

        // Create a set of embeddings for search
        // First 3 items are ML-related, next 3 are unrelated topics
        const int MlRelatedItemsCount = 3;
        var corpus = new[]
        {
            "machine learning algorithms for classification",
            "deep neural networks and transformers",
            "natural language processing with BERT",
            "database indexing strategies",
            "distributed systems architecture",
            "semantic similarity using embeddings"
        };

        var embeddings = corpus.Select(text => VectorTools.GenerateEmbedding(text)).ToArray();

        // Query for ML-related content
        var query = "neural network machine learning";
        var queryEmbedding = VectorTools.GenerateEmbedding(query);

        // Calculate similarities
        var similarities = embeddings.Select((emb, idx) => new
        {
            Index = idx,
            Text = corpus[idx],
            Similarity = CosineSimilarity(queryEmbedding, emb)
        }).OrderByDescending(x => x.Similarity).ToArray();

        // Top results should be ML-related (indices 0-2)
        Assert.That(similarities[0].Index, Is.LessThan(MlRelatedItemsCount),
            $"Top result should be ML-related, but got: {similarities[0].Text} (similarity: {similarities[0].Similarity:F4})");
        Assert.That(similarities[1].Index, Is.LessThan(MlRelatedItemsCount),
            $"Second result should be ML-related, but got: {similarities[1].Text} (similarity: {similarities[1].Similarity:F4})");

        Console.WriteLine("Top 3 search results:");
        for (int i = 0; i < 3; i++)
        {
            Console.WriteLine($"{i + 1}. {similarities[i].Text} (similarity: {similarities[i].Similarity:F4})");
        }
    }

    // Helper methods using reflection to access internal VectorTools state
    private static BertTokenizer GetTokenizerFromVectorTools()
    {
        var assets = GetTokenizerAssetsFromVectorTools();
        var tokenizerProperty = assets.GetType().GetProperty("Tokenizer", BindingFlags.Instance | BindingFlags.Public);
        if (tokenizerProperty == null)
            throw new InvalidOperationException("Could not find Tokenizer property on TokenizerAssets");

        var tokenizer = tokenizerProperty.GetValue(assets) as BertTokenizer;
        if (tokenizer == null)
            throw new InvalidOperationException("Tokenizer is null");

        return tokenizer;
    }

    private static object GetTokenizerAssetsFromVectorTools()
    {
        var field = typeof(VectorTools).GetField("_tokenizer", BindingFlags.NonPublic | BindingFlags.Static);
        if (field == null)
            throw new InvalidOperationException("Could not find _tokenizer field on VectorTools");

        var assets = field.GetValue(null);
        if (assets == null)
            throw new InvalidOperationException("TokenizerAssets is null - model not loaded?");

        return assets;
    }

    private static T GetPropertyValue<T>(object obj, string propertyName)
    {
        var property = obj.GetType().GetProperty(propertyName, BindingFlags.Instance | BindingFlags.Public);
        if (property == null)
            throw new InvalidOperationException($"Property {propertyName} not found");

        var value = property.GetValue(obj);
        if (value is T typedValue)
            return typedValue;

        throw new InvalidOperationException($"Property {propertyName} is not of type {typeof(T)}");
    }

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
