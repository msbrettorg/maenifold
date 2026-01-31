using System.Text.Json;
using Microsoft.ML.OnnxRuntime;
using Microsoft.ML.OnnxRuntime.Tensors;
using Microsoft.ML.Tokenizers;

namespace Maenifold.Utils
{
    public static partial class VectorTools
    {
        private static readonly JsonSerializerOptions TokenizerConfigJsonOptions = new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true
        };
        private sealed class TokenizerAssets
        {
            public BertTokenizer Tokenizer { get; }
            public int PadId { get; }
            public int UnknownId { get; }
            public int ClassifyId { get; }
            public int SeparatorId { get; }

            public TokenizerAssets(BertTokenizer tokenizer, int padId, int unknownId, int classifyId, int separatorId)
            {
                Tokenizer = tokenizer;
                PadId = padId;
                UnknownId = unknownId;
                ClassifyId = classifyId;
                SeparatorId = separatorId;
            }
        }

        private sealed class BertTokenizerConfig
        {
            public string? TokenizerClass { get; set; }
            public string? Tokenizer_Class { get; set; }
            public bool? DoLowerCase { get; set; }
            public string? UnkToken { get; set; }
            public string? SepToken { get; set; }
            public string? PadToken { get; set; }
            public string? ClsToken { get; set; }
            public string? MaskToken { get; set; }
            public bool TokenizeChineseChars { get; set; }
            public string? StripAccents { get; set; }
            public bool DoBasicTokenize { get; set; }
            public int ModelMaxLength { get; set; }
        }

        private static void EnsureModelLoaded()
        {
            if (!_initialized)
                LoadModel();
        }

        private static string FindAssetPath(string fileName)
        {
            var baseDir = AppContext.BaseDirectory;
            var directPath = Path.Combine(baseDir, "assets", "models", fileName);
            if (File.Exists(directPath))
                return directPath;
            var currentDir = new DirectoryInfo(baseDir);
            while (currentDir != null && !File.Exists(Path.Combine(currentDir.FullName, "Maenifold.sln")))
                currentDir = currentDir.Parent;
            if (currentDir == null)
            {
                currentDir = new DirectoryInfo(baseDir);
                while (currentDir != null)
                {
                    var hasSln = Directory.EnumerateFiles(currentDir.FullName, "*.sln")
                        .Any(f => string.Equals(Path.GetFileName(f), "Maenifold.sln", StringComparison.OrdinalIgnoreCase));
                    if (hasSln) break;
                    currentDir = currentDir.Parent;
                }
            }
            if (currentDir == null)
                throw new DirectoryNotFoundException("Could not find repository root containing Maenifold.sln");
            return Path.Combine(currentDir.FullName, "assets", "models", fileName);
        }

        private static string FindModelPath() => FindAssetPath("all-MiniLM-L6-v2.onnx");
        private static string FindVocabPath() => FindAssetPath("vocab.txt");
        private static string FindTokenizerConfigPath() => FindAssetPath("tokenizer_config.json");

        private static TokenizerAssets LoadTokenizerAssets(string vocabPath, string configPath)
        {
            if (!File.Exists(vocabPath))
                throw new FileNotFoundException("Tokenizer vocab file not found.", vocabPath);
            if (!File.Exists(configPath))
                throw new FileNotFoundException("Tokenizer config file not found.", configPath);

            var configText = File.ReadAllText(configPath);
            var config = JsonSerializer.Deserialize<BertTokenizerConfig>(configText, TokenizerConfigJsonOptions);

            if (config == null)
                throw new InvalidOperationException("Tokenizer config could not be parsed.");

            var tokenizerClass = config.TokenizerClass ?? config.Tokenizer_Class;
            if (!string.Equals(tokenizerClass, "BertTokenizer", StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException("Tokenizer config does not specify a BertTokenizer.");

            if (config.DoLowerCase == false)
                throw new InvalidOperationException("Tokenizer config indicates case-sensitive vocab; expected lower-cased BERT vocab.");

            var options = new BertOptions
            {
                ApplyBasicTokenization = config.DoBasicTokenize,
                LowerCaseBeforeTokenization = config.DoLowerCase ?? true,
                IndividuallyTokenizeCjk = config.TokenizeChineseChars
            };

            if (!string.IsNullOrWhiteSpace(config.UnkToken))
                options.UnknownToken = config.UnkToken;
            if (!string.IsNullOrWhiteSpace(config.PadToken))
                options.PaddingToken = config.PadToken;
            if (!string.IsNullOrWhiteSpace(config.ClsToken))
                options.ClassificationToken = config.ClsToken;
            if (!string.IsNullOrWhiteSpace(config.SepToken))
                options.SeparatorToken = config.SepToken;
            if (!string.IsNullOrWhiteSpace(config.MaskToken))
                options.MaskingToken = config.MaskToken;

            var tokenizer = BertTokenizer.Create(vocabPath, options);

            var ids = tokenizer.EncodeToIds(string.Join(" ", new[]
            {
                options.PaddingToken,
                options.UnknownToken,
                options.ClassificationToken,
                options.SeparatorToken
            }.Where(token => !string.IsNullOrWhiteSpace(token))));

            if (ids.Count < 4)
                throw new InvalidOperationException("Tokenizer output did not include expected special tokens.");

            var padId = ids[0];
            var unkId = ids[1];
            var clsId = ids[2];
            var sepId = ids[3];

            if (padId < 0 || unkId < 0 || clsId < 0 || sepId < 0)
                throw new InvalidOperationException("Tokenizer vocab did not resolve expected special tokens to valid ids.");

            return new TokenizerAssets(tokenizer, padId, unkId, clsId, sepId);
        }

        private static TensorInfo? SelectBestOutput(IDisposableReadOnlyCollection<DisposableNamedOnnxValue> outputs)
        {
            var candidates = new List<TensorInfo>();
            foreach (var o in outputs)
            {
                try
                {
                    var t = o.AsTensor<float>();
                    candidates.Add(new TensorInfo { Name = o.Name, Tensor = t });
                    continue;
                }
                catch
                {
                }
                try
                {
                    var t2 = o.AsTensor<double>();
                    candidates.Add(new TensorInfo { Name = o.Name, TensorDouble = t2 });
                    continue;
                }
                catch
                {
                }
            }
            if (!candidates.Any())
                return null;
            var namePref = new[] { "pooled", "pooler", "pool", "sentence", "sentence_embedding", "embedding" };
            foreach (var n in namePref)
            {
                var match = candidates.FirstOrDefault(c => c.Name != null && c.Name.Contains(n, StringComparison.OrdinalIgnoreCase));
                if (match != null)
                    return match;
            }
            var twoD = candidates.FirstOrDefault(c => c.Dimensions.Length == 2 && c.Dimensions[0] == 1);
            if (twoD != null) return twoD;
            var oneD = candidates.FirstOrDefault(c => c.Dimensions.Length == 1);
            if (oneD != null) return oneD;
            return candidates.First();
        }

        private sealed class TensorInfo
        {
            public string? Name { get; set; }
            public Tensor<float>? Tensor { get; set; }
            public Tensor<double>? TensorDouble { get; set; }
            public int[] Dimensions
            {
                get
                {
                    if (Tensor != null) return Tensor.Dimensions.ToArray();
                    if (TensorDouble != null) return TensorDouble.Dimensions.ToArray();
                    return Array.Empty<int>();
                }
            }
        }

        private static float[] GetTensorAsFloatArray(TensorInfo info, out int[] dims)
        {
            if (info.Tensor != null)
            {
                dims = info.Tensor.Dimensions.ToArray();
                try
                {
                    return info.Tensor.ToArray();
                }
                catch
                {
                    var arr = info.Tensor.AsEnumerable().ToArray();
                    dims = info.Tensor.Dimensions.ToArray();
                    return arr;
                }
            }
            if (info.TensorDouble != null)
            {
                dims = info.TensorDouble.Dimensions.ToArray();
                var d = info.TensorDouble.ToArray();
                var f = new float[d.Length];
                for (int i = 0; i < d.Length; i++) f[i] = (float)d[i];
                return f;
            }
            dims = Array.Empty<int>();
            return Array.Empty<float>();
        }

        private static float[] PoolSequenceByAttention(float[] flattened, int seqLen, int hidden, long[] attentionMask)
        {
            var pooled = new float[hidden];
            long count = 0;
            for (int i = 0; i < seqLen; i++)
            {
                var use = 0L;
                if (i < attentionMask.Length) use = attentionMask[i];
                if (use == 0) continue;
                count++;
                var baseIndex = i * hidden;
                for (int h = 0; h < hidden; h++)
                {
                    pooled[h] += flattened[baseIndex + h];
                }
            }
            if (count == 0)
                return pooled;
            var inv = 1.0f / (float)count;
            for (int h = 0; h < hidden; h++) pooled[h] *= inv;
            return pooled;
        }

        private static float[] PadOrTruncate(float[] src, int desired)
        {
            var dst = new float[desired];
            if (src == null || src.Length == 0) return dst;
            var len = Math.Min(src.Length, desired);
            Array.Copy(src, dst, len);
            if (len < desired)
            {
                for (int i = len; i < desired; i++) dst[i] = 0f;
            }
            return dst;
        }
    }
}
