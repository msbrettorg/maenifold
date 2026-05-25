use crate::error::{MaenifoldError, Result};
use ort::{
    session::Session,
    value::{Outlet, TensorElementType, ValueType},
};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tokenizers::Tokenizer;

mod discovery;
mod inference;
mod output;
mod tokenizer;

#[cfg(test)]
mod inference_tests;

pub(crate) use discovery::model_candidate_dirs;
pub use discovery::{cleanup, is_initialized, load_model};
pub use inference::{
    decode_base64_embedding, generate_embedding, generate_embedding_base64, generate_embeddings,
    to_sqlite_vector_blob,
};

pub const DEFAULT_EMBEDDING_DIMENSIONS: usize = 384;
pub const DEFAULT_MAX_TOKENS: usize = 512;
#[doc(hidden)]
pub const EMBEDDING_DIMENSIONS: usize = DEFAULT_EMBEDDING_DIMENSIONS;

pub(crate) const DEFAULT_MAX_BATCH: usize = 32;
pub(crate) const TOKENIZER_CONFIG_FILENAME: &str = "tokenizer_config.json";
pub(crate) const TOKENIZER_JSON_FILENAME: &str = "tokenizer.json";
pub(crate) const OUTPUT_NAME_PREFERENCES: [&str; 6] = [
    "pooled",
    "pooler",
    "pool",
    "sentence",
    "sentence_embedding",
    "embedding",
];

// T-9.1: RTM FR-7 — preserve the runtime-discovered [[embedding]] model state while
// splitting [[vector.rs]] into smaller modules.
pub(crate) struct ModelState {
    pub(crate) session: Session,
    pub(crate) tokenizer: TokenizerAssets,
    pub(crate) embedding_dims: usize,
    pub(crate) max_tokens: ResolvedMaxTokens,
    pub(crate) model_filename: String,
    pub(crate) model_path: PathBuf,
    pub(crate) input_metadata: Vec<ModelInputMetadata>,
}

#[derive(Debug, Clone)]
pub(crate) struct TokenizerAssets {
    pub(crate) tokenizer: Tokenizer,
    pub(crate) pad_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct BertTokenizerConfig {
    pub(crate) pad_token: Option<String>,
    #[serde(default)]
    pub(crate) model_max_length: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelInputMetadata {
    pub(crate) name: String,
    pub(crate) dtype: TensorElementType,
    pub(crate) dims: Vec<i64>,
    pub(crate) dimension_symbols: Vec<String>,
}

impl ModelInputMetadata {
    pub(crate) fn from_outlet(outlet: &Outlet) -> Result<Self> {
        match outlet.dtype() {
            ValueType::Tensor {
                ty,
                shape,
                dimension_symbols,
            } => Ok(Self {
                name: outlet.name().to_string(),
                dtype: *ty,
                dims: shape.iter().copied().collect(),
                dimension_symbols: dimension_symbols.iter().cloned().collect(),
            }),
            other => Err(MaenifoldError::Message(format!(
                "embedding model input `{}` has unsupported non-tensor contract {:?}",
                outlet.name(),
                other
            ))),
        }
    }

    pub(crate) fn describe_contract(&self) -> String {
        format!(
            "input={} dtype={} shape={:?} symbols={:?}",
            self.name, self.dtype, self.dims, self.dimension_symbols
        )
    }
}

pub(crate) fn input_metadata_by_name<'a>(
    metadata: &'a [ModelInputMetadata],
    name: &str,
) -> Option<&'a ModelInputMetadata> {
    metadata.iter().find(|input| input.name == name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedMaxTokens {
    pub(crate) value: usize,
    pub(crate) source_tag: String,
}

static MODEL_STATE: OnceLock<Mutex<Option<ModelState>>> = OnceLock::new();

/// Get the current model's embedding dimensions, or None if no model loaded
pub fn embedding_dimensions() -> Option<usize> {
    MODEL_STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| guard.as_ref().map(|state| state.embedding_dims))
}

/// Get the loaded tokenizer max token length, or None if no model loaded
pub fn max_tokens() -> Option<usize> {
    // T-10.11: RTM FR-18 — expose runtime tokenizer limits so boundary tests can verify
    // `model_max_length` loading and 512-default behavior without reaching into private state.
    MODEL_STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| guard.as_ref().map(|state| state.max_tokens.value))
}

/// Get the loaded model filename, or None if no model loaded
pub fn model_filename() -> Option<String> {
    MODEL_STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| guard.as_ref().map(|state| state.model_filename.clone()))
}

/// Get the full path to the loaded model file, or None if no model loaded
pub fn model_path() -> Option<PathBuf> {
    MODEL_STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| guard.as_ref().map(|state| state.model_path.clone()))
}

/// T-202.9: Extract the currently loaded model ID from the model path.
/// Supports two on-disk layouts:
///   - Subdir layout: `models/bge-small-en-v1.5/model.onnx` → "bge-small-en-v1.5"
///   - Flat layout:   `models/all-MiniLM-L6-v2.onnx`        → "all-MiniLM-L6-v2"
/// When the parent directory is the well-known `models` root, fall back to the
/// file stem so flat-layout installations get a meaningful, distinct ID instead
/// of the literal string "models".
pub fn model_id() -> Option<String> {
    model_path().and_then(|path| {
        let parent_name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());

        if parent_name == Some("models") {
            // Flat layout: derive ID from the file stem.
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
        } else {
            parent_name.map(str::to_string)
        }
    })
}

pub(crate) fn model_state() -> &'static Mutex<Option<ModelState>> {
    MODEL_STATE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn zero_embedding_vector() -> Vec<f32> {
    vec![0.0; embedding_dimensions().unwrap_or(DEFAULT_EMBEDDING_DIMENSIONS)]
}
