using System.Diagnostics;
using Microsoft.ML.OnnxRuntime;
using Microsoft.ML.OnnxRuntime.Tensors;

namespace Maenifold.Utils
{
    public static partial class VectorTools
    {
        private static readonly object _lockObject = new object();
        private static InferenceSession? _session;
        private static SimpleTokenizer? _tokenizer;
        private static bool _initialized;
        private const int MaxTokens = 512;
        private const int EmbeddingDimensions = 384;
        public static int EmbeddingLength => EmbeddingDimensions;

        public static float[] GenerateEmbedding(string text)
        {
            var embeddingTimer = Stopwatch.StartNew();
            if (string.IsNullOrEmpty(text))
            {
                embeddingTimer.Stop();
                if (Config.EnableEmbeddingLogs)
                    Console.Error.WriteLine($"[EMBEDDING] Generated 1 embedding in {embeddingTimer.ElapsedMilliseconds}ms (empty text)");
                return new float[EmbeddingDimensions];
            }

            EnsureModelLoaded();
            if (_session != null && _tokenizer != null)
            {
                var encoding = _tokenizer.Encode(text);
                var tokenIds = encoding.Ids.Take(MaxTokens).Select(id => (long)id).ToArray();
                var inputIds = new long[MaxTokens];
                var attentionMask = new long[MaxTokens];
                for (int i = 0; i < MaxTokens; i++)
                {
                    if (i < tokenIds.Length)
                    {
                        inputIds[i] = tokenIds[i];
                        attentionMask[i] = 1L;
                    }
                    else
                    {
                        inputIds[i] = 0L;
                        attentionMask[i] = 0L;
                    }
                }
                var inputIdsTensor = new DenseTensor<long>(inputIds, new int[] { 1, MaxTokens });
                var attentionMaskTensor = new DenseTensor<long>(attentionMask, new int[] { 1, MaxTokens });
                var tokenTypeIds = new long[MaxTokens];
                var tokenTypeIdsTensor = new DenseTensor<long>(tokenTypeIds, new int[] { 1, MaxTokens });
                var inputs = new List<NamedOnnxValue>
                {
                    NamedOnnxValue.CreateFromTensor("input_ids", inputIdsTensor),
                    NamedOnnxValue.CreateFromTensor("attention_mask", attentionMaskTensor),
                    NamedOnnxValue.CreateFromTensor("token_type_ids", tokenTypeIdsTensor)
                };
                using var results = _session.Run(inputs);
                var chosen = SelectBestOutput(results);
                if (chosen == null)
                {
                    var fallback = GenerateFallbackEmbedding(text);
                    embeddingTimer.Stop();
                    if (Config.EnableEmbeddingLogs)
                        Console.Error.WriteLine($"[EMBEDDING] Generated 1 embedding in {embeddingTimer.ElapsedMilliseconds}ms (no output)");
                    return fallback;
                }
                var dims = chosen.Dimensions;
                if (dims.Length == 2 && dims[0] == 1)
                {
                    var vec = GetTensorAsFloatArray(chosen, out var _);
                    var result2D = PadOrTruncate(vec, EmbeddingDimensions);
                    embeddingTimer.Stop();
                    if (Config.EnableEmbeddingLogs)
                        Console.Error.WriteLine($"[EMBEDDING] Generated 1 embedding in {embeddingTimer.ElapsedMilliseconds}ms (ONNX 2D)");
                    return result2D;
                }
                if (dims.Length == 1)
                {
                    var vec = GetTensorAsFloatArray(chosen, out var _);
                    var result1D = PadOrTruncate(vec, EmbeddingDimensions);
                    embeddingTimer.Stop();
                    if (Config.EnableEmbeddingLogs)
                        Console.Error.WriteLine($"[EMBEDDING] Generated 1 embedding in {embeddingTimer.ElapsedMilliseconds}ms (ONNX 1D)");
                    return result1D;
                }
                if (dims.Length == 3 && dims[0] == 1)
                {
                    var seqLen = dims[1];
                    var hidden = dims[2];
                    var seqTensor = GetTensorAsFloatArray(chosen, out var seqDims);
                    var pooled = PoolSequenceByAttention(seqTensor, seqLen, hidden, attentionMask);
                    var result3D = PadOrTruncate(pooled, EmbeddingDimensions);
                    embeddingTimer.Stop();
                    if (Config.EnableEmbeddingLogs)
                        Console.Error.WriteLine($"[EMBEDDING] Generated 1 embedding in {embeddingTimer.ElapsedMilliseconds}ms (ONNX 3D)");
                    return result3D;
                }
                var fallbackVec = GetTensorAsFloatArray(chosen, out var fallbackDims);
                var resultFlat = PadOrTruncate(fallbackVec, EmbeddingDimensions);
                embeddingTimer.Stop();
                if (Config.EnableEmbeddingLogs)
                    Console.Error.WriteLine($"[EMBEDDING] Generated 1 embedding in {embeddingTimer.ElapsedMilliseconds}ms (ONNX)");
                return resultFlat;
            }
            var fallbackResult = GenerateFallbackEmbedding(text);
            embeddingTimer.Stop();
            if (Config.EnableEmbeddingLogs)
                Console.Error.WriteLine($"[EMBEDDING] Generated 1 embedding in {embeddingTimer.ElapsedMilliseconds}ms (fallback)");
            return fallbackResult;
        }

        public static string GenerateEmbeddingBase64(string text)
        {
            var embedding = GenerateEmbedding(text);
            var bytes = new byte[embedding.Length * sizeof(float)];
            Buffer.BlockCopy(embedding, 0, bytes, 0, bytes.Length);
            return Convert.ToBase64String(bytes);
        }

        public static float[] DecodeBase64Embedding(string base64Embedding)
        {
            if (string.IsNullOrEmpty(base64Embedding))
                return new float[EmbeddingDimensions];

            try
            {
                var bytes = Convert.FromBase64String(base64Embedding);
                var embedding = new float[bytes.Length / sizeof(float)];
                Buffer.BlockCopy(bytes, 0, embedding, 0, bytes.Length);
                return embedding;
            }
            catch
            {
                return new float[EmbeddingDimensions];
            }
        }

        public static void LoadModel()
        {
            if (_initialized)
                return;
            lock (_lockObject)
            {
                if (_initialized)
                    return;
                try
                {
                    var modelPath = FindModelPath();
                    if (File.Exists(modelPath))
                    {
                        var sessionOptions = new SessionOptions();
                        sessionOptions.LogSeverityLevel = OrtLoggingLevel.ORT_LOGGING_LEVEL_WARNING;
                        _session = new InferenceSession(modelPath, sessionOptions);
                        if (Config.EnableEmbeddingLogs)
                            Console.Error.WriteLine($"[VectorTools] ONNX model loaded successfully from: {modelPath}");
                        _tokenizer = new SimpleTokenizer();
                        var vocabPath = FindVocabPath();
                        if (File.Exists(vocabPath))
                        {
                            _tokenizer.LoadVocab(vocabPath);
                            if (Config.EnableEmbeddingLogs)
                                Console.Error.WriteLine($"[VectorTools] Vocabulary loaded successfully from: {vocabPath}");
                        }
                        else
                        {
                            if (Config.EnableEmbeddingLogs)
                            {
                                Console.Error.WriteLine($"[VectorTools] Warning: Vocabulary file not found at: {vocabPath}");
                                Console.Error.WriteLine("[VectorTools] Using basic tokenizer without vocabulary");
                            }
                        }
                    }
                    else
                    {
                        if (Config.EnableEmbeddingLogs)
                        {
                            Console.Error.WriteLine($"[VectorTools] Warning: ONNX model not found at: {modelPath}");
                            Console.Error.WriteLine("[VectorTools] Falling back to hash-based embeddings");
                        }
                    }
                    _initialized = true;
                }
                catch (Exception ex)
                {
                    if (Config.EnableEmbeddingLogs)
                    {
                        Console.Error.WriteLine($"[VectorTools] Error initializing ONNX session: {ex.Message}");
                        Console.Error.WriteLine("[VectorTools] Falling back to hash-based embeddings");
                    }
                    _session?.Dispose();
                    _session = null;
                    _tokenizer = null;
                    _initialized = true;
                }
            }
        }

        private static float[] GenerateFallbackEmbedding(string text)
        {
            var input = System.Text.Encoding.UTF8.GetBytes(text ?? "");
            var hash = System.Security.Cryptography.SHA256.HashData(input);
            var embedding = new float[EmbeddingDimensions];
            var random = new Random(BitConverter.ToInt32(hash, 0));
            for (int i = 0; i < EmbeddingDimensions; i++)
            {
                embedding[i] = (float)(random.NextDouble() * 2.0 - 1.0);
            }
            var magnitude = Math.Sqrt(embedding.Sum(x => x * x));
            if (magnitude > 0)
            {
                for (int i = 0; i < embedding.Length; i++)
                {
                    embedding[i] = (float)(embedding[i] / magnitude);
                }
            }
            return embedding;
        }

        public static void Cleanup()
        {
            lock (_lockObject)
            {
                _session?.Dispose();
                _tokenizer = null;
                _initialized = false;
            }
        }

        public static byte[] ToSqliteVectorBlob(float[] embedding)
        {
            if (embedding == null || embedding.Length == 0)
                return Array.Empty<byte>();
            var bytes = new byte[embedding.Length * sizeof(float)];
            Buffer.BlockCopy(embedding, 0, bytes, 0, bytes.Length);
            return bytes;
        }
    }
}
