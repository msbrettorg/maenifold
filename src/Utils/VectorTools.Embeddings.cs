using System.Diagnostics;
using Microsoft.ML.OnnxRuntime;
using Microsoft.ML.OnnxRuntime.Tensors;
using Microsoft.ML.Tokenizers;

namespace Maenifold.Utils
{
    public static partial class VectorTools
    {
        private static readonly object _lockObject = new object();
        private static InferenceSession? _session;
        private static TokenizerAssets? _tokenizer;
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

            try
            {
                EnsureModelLoaded();
            }
            catch (Exception ex)
            {
                embeddingTimer.Stop();
                if (Config.EnableEmbeddingLogs)
                    Console.Error.WriteLine($"[EMBEDDING] Failed to initialize model/tokenizer in {embeddingTimer.ElapsedMilliseconds}ms: {ex.Message}");
                throw;
            }

            // T-QUAL-FSC2.1: RTM FR-7.4
            if (_session == null)
                throw new InvalidOperationException("Embedding model session was not initialized.");
            if (_tokenizer == null)
                throw new InvalidOperationException("Tokenizer assets were not initialized.");

            var ids = _tokenizer.Tokenizer.EncodeToIds(
                text,
                MaxTokens,
                addSpecialTokens: true,
                out _,
                out _,
                considerPreTokenization: true,
                considerNormalization: true);
            var inputIds = new long[MaxTokens];
            var attentionMask = new long[MaxTokens];
            var maxCopy = Math.Min(ids.Count, MaxTokens);
            for (int i = 0; i < maxCopy; i++)
            {
                inputIds[i] = ids[i];
                attentionMask[i] = 1L;
            }
            for (int i = maxCopy; i < MaxTokens; i++)
            {
                inputIds[i] = _tokenizer.PadId;
                attentionMask[i] = 0L;
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
                throw new InvalidOperationException("Embedding model produced no usable outputs.");
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
            var fallbackVec = GetTensorAsFloatArray(chosen, out var _);
            var resultFlat = PadOrTruncate(fallbackVec, EmbeddingDimensions);
            embeddingTimer.Stop();
            if (Config.EnableEmbeddingLogs)
                Console.Error.WriteLine($"[EMBEDDING] Generated 1 embedding in {embeddingTimer.ElapsedMilliseconds}ms (ONNX)");
            return resultFlat;
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
                var modelPath = FindModelPath();
                if (!File.Exists(modelPath))
                    throw new FileNotFoundException("Embedding model not found.", modelPath);

                var sessionOptions = new SessionOptions();
                sessionOptions.LogSeverityLevel = OrtLoggingLevel.ORT_LOGGING_LEVEL_WARNING;
                _session = new InferenceSession(modelPath, sessionOptions);
                if (Config.EnableEmbeddingLogs)
                    Console.Error.WriteLine($"[VectorTools] ONNX model loaded successfully from: {modelPath}");

                // T-QUAL-FSC2.1: RTM FR-7.4
                var vocabPath = FindVocabPath();
                var configPath = FindTokenizerConfigPath();
                _tokenizer = LoadTokenizerAssets(vocabPath, configPath);
                if (Config.EnableEmbeddingLogs)
                    Console.Error.WriteLine($"[VectorTools] Tokenizer assets loaded from: {vocabPath}");

                _initialized = true;
            }
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
