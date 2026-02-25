"""
Vector embedding generation using ONNX model with fallback to hash-based embeddings.

This module provides thread-safe embedding generation using the all-MiniLM-L6-v2 model
(384 dimensions). Falls back to deterministic SHA256-based pseudo-random embeddings
when the ONNX model is unavailable.
"""

import base64
import hashlib
import os
import re
import struct
import threading
from pathlib import Path
from typing import Dict, List, Optional, Tuple

try:
    import numpy as np
    import onnxruntime as ort
    HAS_ONNX = True
except ImportError:
    HAS_ONNX = False
    np = None  # type: ignore
    ort = None  # type: ignore


# Constants
MAX_TOKENS = 512
EMBEDDING_DIMENSIONS = 384


class SimpleTokenizer:
    """Basic word tokenizer with vocabulary for BERT-style models."""

    def __init__(self):
        self._vocab: Dict[str, int] = {
            "[PAD]": 0,
            "[UNK]": 1,
            "[CLS]": 2,
            "[SEP]": 3,
        }
        self._id_to_token: Dict[int, str] = {
            0: "[PAD]",
            1: "[UNK]",
            2: "[CLS]",
            3: "[SEP]",
        }

    def load_vocab(self, vocab_path: str) -> None:
        """Load vocabulary from file."""
        if not os.path.exists(vocab_path):
            return

        with open(vocab_path, 'r', encoding='utf-8') as f:
            for line in f:
                token = line.strip()
                if token and token not in self._vocab:
                    token_id = len(self._vocab)
                    self._vocab[token] = token_id
                    self._id_to_token[token_id] = token

    def encode(self, text: str, add_special_tokens: bool = True) -> Tuple[List[int], List[str]]:
        """
        Encode text to token IDs.

        Returns:
            Tuple of (ids, tokens)
        """
        # Simple word-based tokenization
        words = [w for w in re.split(r'\W+', text.lower()) if w]

        ids = []
        tokens = []

        if add_special_tokens:
            ids.append(self._vocab["[CLS]"])
            tokens.append("[CLS]")

        for word in words:
            token_id = self._vocab.get(word, self._vocab["[UNK]"])
            ids.append(token_id)
            tokens.append(self._id_to_token.get(token_id, "[UNK]"))

        if add_special_tokens:
            ids.append(self._vocab["[SEP]"])
            tokens.append("[SEP]")

        return ids, tokens


class VectorTools:
    """Thread-safe vector embedding generation."""

    _lock = threading.Lock()
    _session: Optional['ort.InferenceSession'] = None
    _tokenizer: Optional[SimpleTokenizer] = None
    _initialized = False

    @classmethod
    def generate_embedding(cls, text: str) -> List[float]:
        """
        Generate 384-dimensional embedding for text.

        Falls back to hash-based embedding if ONNX model unavailable.

        Args:
            text: Input text to embed

        Returns:
            384-dimensional embedding as list of floats
        """
        if not text:
            return [0.0] * EMBEDDING_DIMENSIONS

        cls._ensure_model_loaded()

        if cls._session is not None and cls._tokenizer is not None:
            try:
                return cls._generate_onnx_embedding(text)
            except Exception:
                # Fall back on any ONNX error
                pass

        return cls._generate_fallback_embedding(text)

    @classmethod
    def _generate_onnx_embedding(cls, text: str) -> List[float]:
        """Generate embedding using ONNX model."""
        if cls._session is None or cls._tokenizer is None:
            raise RuntimeError("Model not loaded")

        # Tokenize
        token_ids, _ = cls._tokenizer.encode(text)

        # Truncate to MAX_TOKENS
        token_ids = token_ids[:MAX_TOKENS]

        # Create input tensors
        input_ids = np.zeros((1, MAX_TOKENS), dtype=np.int64)
        attention_mask = np.zeros((1, MAX_TOKENS), dtype=np.int64)
        token_type_ids = np.zeros((1, MAX_TOKENS), dtype=np.int64)

        for i, token_id in enumerate(token_ids):
            input_ids[0, i] = token_id
            attention_mask[0, i] = 1

        # Run inference
        outputs = cls._session.run(
            None,
            {
                "input_ids": input_ids,
                "attention_mask": attention_mask,
                "token_type_ids": token_type_ids,
            }
        )

        # Select best output
        chosen = cls._select_best_output(outputs, cls._session.get_outputs())
        if chosen is None:
            return cls._generate_fallback_embedding(text)

        # Handle different tensor shapes
        if len(chosen.shape) == 2 and chosen.shape[0] == 1:
            # [1, embedding_dim] - take first row
            embedding = chosen[0].astype(np.float32)
        elif len(chosen.shape) == 1:
            # [embedding_dim] - use directly
            embedding = chosen.astype(np.float32)
        elif len(chosen.shape) == 3 and chosen.shape[0] == 1:
            # [1, seq_len, hidden_dim] - pool over sequence
            seq_len = chosen.shape[1]
            hidden_dim = chosen.shape[2]
            embedding = cls._pool_sequence_by_attention(
                chosen[0], attention_mask[0], seq_len, hidden_dim
            )
        else:
            # Flatten and hope for the best
            embedding = chosen.flatten().astype(np.float32)

        # Pad or truncate to EMBEDDING_DIMENSIONS
        return cls._pad_or_truncate(embedding.tolist(), EMBEDDING_DIMENSIONS)

    @classmethod
    def _select_best_output(
        cls,
        outputs: List,
        output_names: List
    ) -> Optional:
        """Select the most likely embedding tensor from ONNX outputs."""
        if not outputs:
            return None

        # Prefer outputs with these name patterns
        name_preferences = ["pooled", "pooler", "pool", "sentence", "sentence_embedding", "embedding"]

        for pattern in name_preferences:
            for i, output_meta in enumerate(output_names):
                if pattern in output_meta.name.lower():
                    return outputs[i]

        # Fall back to shape-based selection
        for i, output in enumerate(outputs):
            if len(output.shape) == 2 and output.shape[0] == 1:
                return output

        for output in outputs:
            if len(output.shape) == 1:
                return output

        return outputs[0]

    @classmethod
    def _pool_sequence_by_attention(
        cls,
        sequence,
        attention_mask,
        seq_len: int,
        hidden_dim: int
    ):
        """Average pool sequence embeddings using attention mask."""
        pooled = np.zeros(hidden_dim, dtype=np.float32)
        count = 0

        for i in range(seq_len):
            if i < len(attention_mask) and attention_mask[i] == 0:
                continue
            count += 1
            pooled += sequence[i]

        if count > 0:
            pooled /= count

        return pooled

    @classmethod
    def _pad_or_truncate(cls, embedding: List[float], desired: int) -> List[float]:
        """Pad with zeros or truncate to desired length."""
        if len(embedding) >= desired:
            return embedding[:desired]
        return embedding + [0.0] * (desired - len(embedding))

    @classmethod
    def _generate_fallback_embedding(cls, text: str) -> List[float]:
        """
        Generate deterministic pseudo-random embedding using SHA256 hash.

        This provides a consistent embedding when ONNX model is unavailable.
        The embedding is normalized to unit length.
        """
        # Hash the text
        text_bytes = text.encode('utf-8')
        hash_digest = hashlib.sha256(text_bytes).digest()

        # Use first 4 bytes as random seed
        seed = struct.unpack('i', hash_digest[:4])[0]

        # Generate pseudo-random embedding
        import random
        rng = random.Random(seed)
        embedding = [rng.uniform(-1.0, 1.0) for _ in range(EMBEDDING_DIMENSIONS)]

        # Normalize to unit length
        magnitude = sum(x * x for x in embedding) ** 0.5
        if magnitude > 0:
            embedding = [x / magnitude for x in embedding]

        return embedding

    @classmethod
    def generate_embedding_base64(cls, text: str) -> str:
        """
        Generate embedding and encode as base64 string.

        Args:
            text: Input text to embed

        Returns:
            Base64-encoded embedding
        """
        embedding = cls.generate_embedding(text)
        bytes_data = struct.pack(f'{len(embedding)}f', *embedding)
        return base64.b64encode(bytes_data).decode('ascii')

    @classmethod
    def decode_base64_embedding(cls, base64_str: str) -> List[float]:
        """
        Decode base64 string to embedding.

        Args:
            base64_str: Base64-encoded embedding

        Returns:
            Embedding as list of floats
        """
        if not base64_str:
            return [0.0] * EMBEDDING_DIMENSIONS

        try:
            bytes_data = base64.b64decode(base64_str)
            count = len(bytes_data) // 4  # 4 bytes per float
            embedding = struct.unpack(f'{count}f', bytes_data)
            return list(embedding)
        except Exception:
            return [0.0] * EMBEDDING_DIMENSIONS

    @classmethod
    def to_sqlite_vector_blob(cls, embedding: List[float]) -> bytes:
        """
        Convert embedding to bytes for sqlite-vec storage.

        Args:
            embedding: Embedding as list of floats

        Returns:
            Raw bytes (float32 array)
        """
        if not embedding:
            return b''
        return struct.pack(f'{len(embedding)}f', *embedding)

    @classmethod
    def _ensure_model_loaded(cls) -> None:
        """Ensure ONNX model is loaded (thread-safe)."""
        if cls._initialized:
            return

        with cls._lock:
            if cls._initialized:
                return

            cls._load_model()
            cls._initialized = True

    @classmethod
    def _load_model(cls) -> None:
        """Load ONNX model and tokenizer."""
        if not HAS_ONNX:
            return

        try:
            model_path = cls._find_asset_path("all-MiniLM-L6-v2.onnx")

            if os.path.exists(model_path):
                # Create session with minimal logging
                sess_options = ort.SessionOptions()
                sess_options.log_severity_level = 3  # WARNING
                cls._session = ort.InferenceSession(model_path, sess_options)

                # Load tokenizer
                cls._tokenizer = SimpleTokenizer()
                vocab_path = cls._find_asset_path("vocab.txt")
                if os.path.exists(vocab_path):
                    cls._tokenizer.load_vocab(vocab_path)
        except Exception:
            # Silently fall back to hash-based embeddings
            cls._session = None
            cls._tokenizer = None

    @classmethod
    def _find_asset_path(cls, filename: str) -> str:
        """
        Find asset file in project structure.

        Searches from current file location up to project root.
        """
        # Start from this file's directory
        current = Path(__file__).parent

        # Try direct path from utils/
        direct = current.parent.parent / "assets" / "models" / filename
        if direct.exists():
            return str(direct)

        # Traverse up to find project root (contains pyproject.toml or setup.py)
        while current != current.parent:
            # Check for project markers
            if (current / "pyproject.toml").exists() or (current / "setup.py").exists():
                asset_path = current / "assets" / "models" / filename
                if asset_path.exists():
                    return str(asset_path)
            current = current.parent

        # Fall back to relative path from package
        return str(Path(__file__).parent.parent.parent / "assets" / "models" / filename)

    @classmethod
    def cleanup(cls) -> None:
        """Clean up resources (for testing)."""
        with cls._lock:
            cls._session = None
            cls._tokenizer = None
            cls._initialized = False

    @classmethod
    @property
    def embedding_length(cls) -> int:
        """Get embedding dimension."""
        return EMBEDDING_DIMENSIONS
