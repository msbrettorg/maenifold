using System.Reflection;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

[TestFixture]
[NonParallelizable]
public sealed class TQualFsc2TokenizerDiagnosticsTests
{
    [Test]
    public void PrintTokenizationDiagnostics()
    {
        // T-QUAL-FSC2.1: RTM FR-7.4
        // This test is intentionally disabled: tokenizer diagnostics are now validated via
        // LoadTokenizerAssets and GenerateEmbedding integration tests using Microsoft.ML.Tokenizers.
        Assert.Ignore("Tokenizer diagnostics are covered by integration tests using Microsoft.ML.Tokenizers.");
    }

    private static string ResolveVocabPath()
    {
        var findVocab = typeof(VectorTools).GetMethod("FindVocabPath", BindingFlags.NonPublic | BindingFlags.Static);
        if (findVocab == null)
            throw new InvalidOperationException("Could not locate VectorTools.FindVocabPath via reflection");

        var path = findVocab.Invoke(null, Array.Empty<object?>()) as string;
        if (string.IsNullOrWhiteSpace(path))
            throw new InvalidOperationException("VectorTools.FindVocabPath returned null or empty");

        return path;
    }

    private static string ResolveTokenizerConfigPath()
    {
        var findConfig = typeof(VectorTools).GetMethod("FindTokenizerConfigPath", BindingFlags.NonPublic | BindingFlags.Static);
        if (findConfig == null)
            throw new InvalidOperationException("Could not locate VectorTools.FindTokenizerConfigPath via reflection");

        var path = findConfig.Invoke(null, Array.Empty<object?>()) as string;
        if (string.IsNullOrWhiteSpace(path))
            throw new InvalidOperationException("VectorTools.FindTokenizerConfigPath returned null or empty");

        return path;
    }

    private static object InvokeLoadTokenizerAssets(string vocabPath, string configPath)
    {
        var load = typeof(VectorTools).GetMethod("LoadTokenizerAssets", BindingFlags.NonPublic | BindingFlags.Static);
        if (load == null)
            throw new InvalidOperationException("Could not locate VectorTools.LoadTokenizerAssets via reflection");

        var assets = load.Invoke(null, new object[] { vocabPath, configPath });
        if (assets == null)
            throw new InvalidOperationException("VectorTools.LoadTokenizerAssets returned null");

        return assets;
    }

    private static object GetTokenizerFromAssets(object assets)
    {
        var property = assets.GetType().GetProperty("Tokenizer", BindingFlags.Instance | BindingFlags.Public);
        if (property == null)
            throw new InvalidOperationException("Tokenizer assets did not expose a Tokenizer property");

        var tokenizer = property.GetValue(assets);
        if (tokenizer == null)
            throw new InvalidOperationException("Tokenizer assets returned a null tokenizer instance");

        return tokenizer;
    }

    // NOTE: Diagnostics use reflection helpers elsewhere; no direct tokenization APIs needed here.
}
