use crate::error::{MaenifoldError, Result};
use base64::Engine as _;
use ort::{session::Session, value::Tensor};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tokenizers::{
    AddedToken, PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy,
    models::wordpiece::WordPiece, normalizers::bert::BertNormalizer,
    pre_tokenizers::bert::BertPreTokenizer, processors::template::TemplateProcessing,
};

pub const DEFAULT_EMBEDDING_DIMENSIONS: usize = 384;
const DEFAULT_MAX_TOKENS: usize = 512;
#[doc(hidden)]
pub const EMBEDDING_DIMENSIONS: usize = DEFAULT_EMBEDDING_DIMENSIONS;

const DEFAULT_MAX_BATCH: usize = 32;
const VOCAB_FILENAME: &str = "vocab.txt";
const TOKENIZER_CONFIG_FILENAME: &str = "tokenizer_config.json";
const TOKENIZER_JSON_FILENAME: &str = "tokenizer.json";
const OUTPUT_NAME_PREFERENCES: [&str; 6] = [
    "pooled",
    "pooler",
    "pool",
    "sentence",
    "sentence_embedding",
    "embedding",
];

// T-8.2: RTM FR-33.1 / FR-33.2 / FR-33.3 — ONNX/tokenizer assets used to generate
// 384-dimensional [[embedding]] vectors with batched inference parity.
struct ModelState {
    session: Session,
    tokenizer: TokenizerAssets,
    // T-9.1: RTM FR-7 — runtime model metadata comes from the loaded ONNX/tokenizer assets
    // instead of hardcoded constants so [[dynamic model loading]] works across model swaps.
    embedding_dims: usize,
    max_tokens: usize,
    model_filename: String,
}

#[derive(Debug, Clone)]
struct TokenizerAssets {
    tokenizer: Tokenizer,
    pad_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct BertTokenizerConfig {
    #[serde(alias = "tokenizerClass", alias = "tokenizer_Class")]
    tokenizer_class: Option<String>,
    do_lower_case: Option<bool>,
    unk_token: Option<String>,
    sep_token: Option<String>,
    pad_token: Option<String>,
    cls_token: Option<String>,
    mask_token: Option<String>,
    #[serde(default = "default_true")]
    tokenize_chinese_chars: bool,
    strip_accents: Option<bool>,
    #[serde(default = "default_true")]
    do_basic_tokenize: bool,
    #[serde(default = "default_model_max_length")]
    model_max_length: usize,
}

#[derive(Debug, Clone)]
struct OutputCandidate {
    index: usize,
    name: String,
    dims: Vec<usize>,
    element_type: OutputElementType,
}

#[derive(Debug, Clone, Copy)]
enum OutputElementType {
    Float32,
    Float64,
}

static MODEL_STATE: OnceLock<Mutex<Option<ModelState>>> = OnceLock::new();

/// Generate embedding for a single text
pub fn generate_embedding(text: &str) -> Result<Vec<f32>> {
    generate_embeddings(&[text], DEFAULT_MAX_BATCH).map(|mut embeddings| {
        embeddings
            .drain(..)
            .next()
            .unwrap_or_else(zero_embedding_vector)
    })
}

/// Generate embeddings for multiple texts in batched ONNX inference (max_batch=32 default)
pub fn generate_embeddings(texts: &[&str], max_batch: usize) -> Result<Vec<Vec<f32>>> {
    // T-8.2: RTM FR-33.1 / FR-33.2 / FR-33.3 — batch inference preserves individual
    // embedding behavior for empty inputs and chunked ONNX execution.
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let non_empty = texts
        .iter()
        .enumerate()
        .filter_map(|(index, text)| {
            if text.trim().is_empty() {
                None
            } else {
                // T-8.5a: RTM FR-33.1 / FR-33.2 / FR-33.3 — cap raw input before
                // tokenization so huge documents do not pay unnecessary tokenizer cost.
                Some((index, text.chars().take(10_000).collect::<String>()))
            }
        })
        .collect::<Vec<_>>();

    if non_empty.is_empty() {
        return Ok(vec![zero_embedding_vector(); texts.len()]);
    }

    load_model()?;

    let effective_batch = if max_batch == 0 {
        DEFAULT_MAX_BATCH
    } else {
        max_batch
    };

    let mut guard = model_state()
        .lock()
        .map_err(|_| MaenifoldError::Message("embedding model mutex poisoned".to_string()))?;
    let state = guard.as_mut().ok_or_else(|| {
        MaenifoldError::Message("embedding model session was not initialized".to_string())
    })?;
    let mut results = vec![vec![0.0; state.embedding_dims]; texts.len()];

    for chunk in non_empty.chunks(effective_batch) {
        let batch_size = chunk.len();
        let max_tokens = state.max_tokens;

        let mut input_ids = vec![i64::from(state.tokenizer.pad_id); batch_size * max_tokens];
        let mut attention_mask = vec![0_i64; batch_size * max_tokens];
        let mut token_type_ids = vec![0_i64; batch_size * max_tokens];

        for (batch_index, (_, text)) in chunk.iter().enumerate() {
            let encoding = state
                .tokenizer
                .tokenizer
                .encode(text.as_str(), true)
                .map_err(|error| {
                    MaenifoldError::Message(format!("tokenizer encode failed: {error}"))
                })?;

            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let types = encoding.get_type_ids();

            let max_copy = ids.len().min(max_tokens);
            let row_offset = batch_index * max_tokens;
            for token_index in 0..max_copy {
                input_ids[row_offset + token_index] = i64::from(ids[token_index]);
                attention_mask[row_offset + token_index] = i64::from(mask[token_index]);
                token_type_ids[row_offset + token_index] = i64::from(types[token_index]);
            }
        }

        let outputs = state
            .session
            .run(ort::inputs! {
                "input_ids" => Tensor::from_array(([batch_size, max_tokens], input_ids.clone().into_boxed_slice())).map_err(map_ort_error)?,
                "attention_mask" => Tensor::from_array(([batch_size, max_tokens], attention_mask.clone().into_boxed_slice())).map_err(map_ort_error)?,
                "token_type_ids" => Tensor::from_array(([batch_size, max_tokens], token_type_ids.into_boxed_slice())).map_err(map_ort_error)?,
            })
            .map_err(map_ort_error)?;

        let chosen = select_best_output(&outputs)?;
        let (dims, flat_data) = extract_output_data(&outputs, &chosen)?;

        for (batch_index, (result_index, _)) in chunk.iter().enumerate() {
            let embedding = extract_embedding_from_output(
                &dims,
                &flat_data,
                batch_index,
                &attention_mask[(batch_index * max_tokens)..((batch_index + 1) * max_tokens)],
            );
            results[*result_index] = pad_or_truncate(&embedding, state.embedding_dims);
        }
    }

    Ok(results)
}

/// Encode embedding as base64 (for transport)
pub fn generate_embedding_base64(text: &str) -> Result<String> {
    let embedding = generate_embedding(text)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(to_sqlite_vector_blob(&embedding)))
}

/// Decode base64 embedding back to float array
pub fn decode_base64_embedding(base64: &str) -> Vec<f32> {
    if base64.trim().is_empty() {
        return zero_embedding_vector();
    }

    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(base64) else {
        return zero_embedding_vector();
    };

    decode_le_f32_bytes(&bytes).unwrap_or_else(|_| zero_embedding_vector())
}

/// Convert float array to SQLite blob format (little-endian f32 bytes)
pub fn to_sqlite_vector_blob(embedding: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(std::mem::size_of_val(embedding));
    for value in embedding {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}

/// Load/initialize the model (called lazily on first use)
pub fn load_model() -> Result<()> {
    // T-8.2: RTM FR-33.1 — model initialization is lazy, idempotent, and safe under
    // concurrent access so repeated batch/single embedding calls reuse the same session.
    let mut guard = model_state()
        .lock()
        .map_err(|_| MaenifoldError::Message("embedding model mutex poisoned".to_string()))?;
    if guard.is_some() {
        return Ok(());
    }

    let model_path = find_model_path()?;
    let session = Session::builder()
        .map_err(map_ort_error)?
        .commit_from_file(&model_path)
        .map_err(map_ort_error)?;
    let embedding_dims = discover_embedding_dimensions(&session)?;
    let model_dir = model_path.parent().ok_or_else(|| {
        MaenifoldError::Message("embedding model has no parent directory".to_string())
    })?;
    let (tokenizer, max_tokens) = load_tokenizer_assets(model_dir)?;
    let model_filename = model_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MaenifoldError::NonUtf8Path(model_path.clone()))?
        .to_string();
    *guard = Some(ModelState {
        session,
        tokenizer,
        embedding_dims,
        max_tokens,
        model_filename,
    });
    Ok(())
}

/// Cleanup/release model resources
pub fn cleanup() {
    if let Some(state) = MODEL_STATE.get()
        && let Ok(mut guard) = state.lock()
    {
        *guard = None;
    }
}

/// Check if model is initialized
pub fn is_initialized() -> bool {
    MODEL_STATE
        .get()
        .and_then(|state| state.lock().ok())
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// Get the current model's embedding dimensions, or None if no model loaded
pub fn embedding_dimensions() -> Option<usize> {
    MODEL_STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| guard.as_ref().map(|state| state.embedding_dims))
}

/// Get the loaded model filename, or None if no model loaded
pub fn model_filename() -> Option<String> {
    MODEL_STATE
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| guard.as_ref().map(|state| state.model_filename.clone()))
}

fn model_state() -> &'static Mutex<Option<ModelState>> {
    MODEL_STATE.get_or_init(|| Mutex::new(None))
}

fn find_model_path() -> Result<PathBuf> {
    let executable = env::current_exe()?;
    let base_dir = executable
        .parent()
        .ok_or(MaenifoldError::MissingExecutableParent)?;

    // T-9.1: RTM FR-7 — search every supported assets/models directory for any usable ONNX
    // file so the runtime can adopt the largest available model without filename pinning.
    let direct_models_dir = base_dir.join("assets").join("models");
    if let Some(model_path) = discover_model(&direct_models_dir) {
        return Ok(model_path);
    }

    for ancestor in base_dir.ancestors() {
        let workspace_manifest = ancestor.join("Cargo.toml");
        let models_dir = ancestor.join("assets").join("models");
        if workspace_manifest.exists()
            && let Some(model_path) = discover_model(&models_dir)
        {
            return Ok(model_path);
        }
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fallback_models_dir = manifest_dir.join("../../assets/models");
    if let Some(model_path) = discover_model(&fallback_models_dir) {
        return Ok(model_path);
    }

    Err(MaenifoldError::Message(format!(
        "embedding model not found: expected a non-empty *.onnx file under assets/models"
    )))
}

fn discover_model(models_dir: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(models_dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let extension = path.extension()?.to_str()?;
            if !extension.eq_ignore_ascii_case("onnx") {
                return None;
            }

            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() || metadata.len() == 0 {
                return None;
            }

            Some((metadata.len(), path))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates.into_iter().next().map(|(_, path)| path)
}

fn load_tokenizer_assets(model_dir: &Path) -> Result<(TokenizerAssets, usize)> {
    let tokenizer_config_path = model_dir.join(TOKENIZER_CONFIG_FILENAME);
    let config = load_tokenizer_config(&tokenizer_config_path)?;
    let max_tokens = config
        .as_ref()
        .map(|config| config.model_max_length)
        .filter(|max_tokens| *max_tokens > 0)
        .unwrap_or(DEFAULT_MAX_TOKENS);

    // T-9.1: RTM FR-7 — prefer HuggingFace tokenizer.json for cross-model compatibility and
    // only fall back to manual BERT WordPiece reconstruction when tokenizer.json is absent.
    let mut tokenizer = {
        let tokenizer_json = model_dir.join(TOKENIZER_JSON_FILENAME);
        if tokenizer_json.is_file() {
            Tokenizer::from_file(&tokenizer_json).map_err(|error| {
                MaenifoldError::Message(format!("tokenizer.json load failed: {error}"))
            })?
        } else {
            let config = config.as_ref().ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "tokenizer.json not found and tokenizer config missing: {}",
                    tokenizer_config_path.display()
                ))
            })?;
            let vocab_path = model_dir.join(VOCAB_FILENAME);
            if !vocab_path.is_file() {
                return Err(MaenifoldError::Message(format!(
                    "tokenizer.json not found and vocab missing: {}",
                    vocab_path.display()
                )));
            }
            build_manual_tokenizer(&vocab_path, config)?
        }
    };

    let pad_token = tokenizer
        .get_padding()
        .map(|padding| padding.pad_token.clone())
        .or_else(|| config.as_ref().and_then(|config| config.pad_token.clone()))
        .unwrap_or_else(|| "[PAD]".to_string());
    let pad_id = tokenizer
        .get_padding()
        .map(|padding| padding.pad_id)
        .or_else(|| tokenizer.token_to_id(&pad_token))
        .or_else(|| tokenizer.token_to_id("<pad>"))
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "missing tokenizer pad token id for tokenizer in {}",
                model_dir.display()
            ))
        })?;

    tokenizer
        .with_truncation(Some(TruncationParams {
            max_length: max_tokens,
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
            ..Default::default()
        }))
        .map_err(|error| {
            MaenifoldError::Message(format!("tokenizer truncation setup failed: {error}"))
        })?;
    tokenizer.with_padding(Some(PaddingParams {
        strategy: PaddingStrategy::Fixed(max_tokens),
        pad_id,
        pad_type_id: 0,
        pad_token,
        ..Default::default()
    }));

    Ok((TokenizerAssets { tokenizer, pad_id }, max_tokens))
}

fn load_tokenizer_config(config_path: &Path) -> Result<Option<BertTokenizerConfig>> {
    if !config_path.is_file() {
        return Ok(None);
    }

    let config_text = fs::read_to_string(config_path)?;
    let config: BertTokenizerConfig = serde_json::from_str(&config_text)?;
    Ok(Some(config))
}

fn build_manual_tokenizer(vocab_path: &Path, config: &BertTokenizerConfig) -> Result<Tokenizer> {
    let tokenizer_class = config.tokenizer_class.as_deref().unwrap_or("BertTokenizer");
    if !tokenizer_class.eq_ignore_ascii_case("BertTokenizer") {
        return Err(MaenifoldError::Message(
            "tokenizer config does not specify a BertTokenizer".to_string(),
        ));
    }

    if matches!(config.do_lower_case, Some(false)) {
        return Err(MaenifoldError::Message(
            "tokenizer config indicates case-sensitive vocab; expected lower-cased BERT vocab"
                .to_string(),
        ));
    }
    if !config.do_basic_tokenize {
        return Err(MaenifoldError::Message(
            "tokenizer config disables basic tokenization; unsupported for BERT WordPiece parity"
                .to_string(),
        ));
    }

    let pad_token = config
        .pad_token
        .clone()
        .unwrap_or_else(|| "[PAD]".to_string());
    let unk_token = config
        .unk_token
        .clone()
        .unwrap_or_else(|| "[UNK]".to_string());
    let cls_token = config
        .cls_token
        .clone()
        .unwrap_or_else(|| "[CLS]".to_string());
    let sep_token = config
        .sep_token
        .clone()
        .unwrap_or_else(|| "[SEP]".to_string());
    let mask_token = config
        .mask_token
        .clone()
        .unwrap_or_else(|| "[MASK]".to_string());

    let mut tokenizer = build_wordpiece_tokenizer(
        vocab_path,
        config.do_lower_case.unwrap_or(true),
        config.tokenize_chinese_chars,
        config.strip_accents,
        &pad_token,
        &unk_token,
        &cls_token,
        &sep_token,
    )?;

    tokenizer.add_special_tokens(&[
        AddedToken::from(pad_token, true),
        AddedToken::from(unk_token, true),
        AddedToken::from(cls_token, true),
        AddedToken::from(sep_token, true),
        AddedToken::from(mask_token, true),
    ]);

    Ok(tokenizer)
}

fn discover_embedding_dimensions(session: &Session) -> Result<usize> {
    let chosen = select_best_session_output(session.outputs())?;
    chosen
        .dims
        .last()
        .copied()
        .filter(|dimension| *dimension > 0)
        .and_then(|dimension| usize::try_from(dimension).ok())
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "embedding model output `{}` does not expose a concrete embedding dimension",
                chosen.name
            ))
        })
}

#[allow(clippy::too_many_arguments)]
fn build_wordpiece_tokenizer(
    vocab_path: &Path,
    do_lower_case: bool,
    tokenize_chinese_chars: bool,
    strip_accents: Option<bool>,
    pad_token: &str,
    unk_token: &str,
    cls_token: &str,
    sep_token: &str,
) -> Result<Tokenizer> {
    let vocab_path_str = vocab_path
        .to_str()
        .ok_or_else(|| MaenifoldError::NonUtf8Path(vocab_path.to_path_buf()))?;
    let word_piece = WordPiece::from_file(vocab_path_str)
        .unk_token(unk_token.to_string())
        .build()
        .map_err(|error| {
            MaenifoldError::Message(format!("wordpiece vocab load failed: {error}"))
        })?;

    let mut tokenizer = Tokenizer::new(word_piece);
    tokenizer.with_normalizer(Some(BertNormalizer::new(
        true,
        tokenize_chinese_chars,
        strip_accents,
        do_lower_case,
    )));
    tokenizer.with_pre_tokenizer(Some(BertPreTokenizer));
    tokenizer.with_post_processor(Some(
        TemplateProcessing::builder()
            .try_single(vec![cls_token, "$0", sep_token])
            .map_err(|error| {
                MaenifoldError::Message(format!(
                    "bert template single-sequence setup failed: {error}"
                ))
            })?
            .try_pair(format!(
                "{cls_token}:0 $A:0 {sep_token}:0 $B:1 {sep_token}:1"
            ))
            .map_err(|error| {
                MaenifoldError::Message(format!(
                    "bert template pair-sequence setup failed: {error}"
                ))
            })?
            .special_tokens(vec![(cls_token, 101), (sep_token, 102)])
            .build()
            .map_err(|error| {
                MaenifoldError::Message(format!("bert template processor build failed: {error}"))
            })?,
    ));
    tokenizer.add_special_tokens(&[
        AddedToken::from(pad_token.to_string(), true),
        AddedToken::from(unk_token.to_string(), true),
        AddedToken::from(cls_token.to_string(), true),
        AddedToken::from(sep_token.to_string(), true),
    ]);
    Ok(tokenizer)
}

fn select_best_output(outputs: &ort::session::SessionOutputs<'_>) -> Result<OutputCandidate> {
    let mut candidates = Vec::new();
    for (index, (name, output)) in outputs.iter().enumerate() {
        if let Ok((shape, _)) = output.try_extract_tensor::<f32>() {
            candidates.push(OutputCandidate {
                index,
                name: name.to_string(),
                dims: shape_to_usize_vec(shape)?,
                element_type: OutputElementType::Float32,
            });
            continue;
        }

        if let Ok((shape, _)) = output.try_extract_tensor::<f64>() {
            candidates.push(OutputCandidate {
                index,
                name: name.to_string(),
                dims: shape_to_usize_vec(shape)?,
                element_type: OutputElementType::Float64,
            });
        }
    }

    if candidates.is_empty() {
        return Err(MaenifoldError::Message(
            "embedding model produced no usable outputs".to_string(),
        ));
    }

    for preferred_name in OUTPUT_NAME_PREFERENCES {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.name.to_ascii_lowercase().contains(preferred_name))
        {
            return Ok(candidate.clone());
        }
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.dims.len() == 2)
    {
        return Ok(candidate.clone());
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.dims.len() == 1)
    {
        return Ok(candidate.clone());
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.dims.len() == 3)
    {
        return Ok(candidate.clone());
    }

    Ok(candidates
        .into_iter()
        .next()
        .expect("candidate list already checked as non-empty"))
}

#[derive(Debug, Clone)]
struct SessionOutputCandidate {
    name: String,
    dims: Vec<i64>,
}

fn select_best_session_output(outputs: &[ort::value::Outlet]) -> Result<SessionOutputCandidate> {
    let mut candidates = outputs
        .iter()
        .filter_map(|output| {
            let dims = output
                .dtype()
                .tensor_shape()?
                .iter()
                .copied()
                .collect::<Vec<_>>();
            Some(SessionOutputCandidate {
                name: output.name().to_string(),
                dims,
            })
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err(MaenifoldError::Message(
            "embedding model metadata exposes no tensor outputs".to_string(),
        ));
    }

    for preferred_name in OUTPUT_NAME_PREFERENCES {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.name.to_ascii_lowercase().contains(preferred_name))
        {
            return Ok(candidate.clone());
        }
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.dims.len() == 2)
    {
        return Ok(candidate.clone());
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.dims.len() == 1)
    {
        return Ok(candidate.clone());
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.dims.len() == 3)
    {
        return Ok(candidate.clone());
    }

    Ok(candidates.remove(0))
}

fn extract_output_data(
    outputs: &ort::session::SessionOutputs<'_>,
    chosen: &OutputCandidate,
) -> Result<(Vec<usize>, Vec<f32>)> {
    let output = &outputs[chosen.index];
    match chosen.element_type {
        OutputElementType::Float32 => {
            let (shape, values) = output.try_extract_tensor::<f32>().map_err(map_ort_error)?;
            Ok((shape_to_usize_vec(shape)?, values.to_vec()))
        }
        OutputElementType::Float64 => {
            let (shape, values) = output.try_extract_tensor::<f64>().map_err(map_ort_error)?;
            Ok((
                shape_to_usize_vec(shape)?,
                values.iter().map(|value| *value as f32).collect(),
            ))
        }
    }
}

fn extract_embedding_from_output(
    dims: &[usize],
    flat_data: &[f32],
    batch_index: usize,
    attention_mask: &[i64],
) -> Vec<f32> {
    match dims {
        [batch, hidden] if batch_index < *batch => {
            let start = batch_index * *hidden;
            let end = start + *hidden;
            flat_data
                .get(start..end)
                .map(|slice| slice.to_vec())
                .unwrap_or_else(zero_embedding_vector)
        }
        [batch, seq_len, hidden] if batch_index < *batch => {
            let item_len = seq_len * hidden;
            let start = batch_index * item_len;
            let end = start + item_len;
            flat_data
                .get(start..end)
                .map(|slice| pool_sequence_by_attention(slice, *seq_len, *hidden, attention_mask))
                .unwrap_or_else(zero_embedding_vector)
        }
        [hidden] => {
            let mut vec = vec![0.0; *hidden];
            let start = batch_index * *hidden;
            let end = (start + *hidden).min(flat_data.len());
            if let Some(slice) = flat_data.get(start..end) {
                vec[..slice.len()].copy_from_slice(slice);
            }
            vec
        }
        _ => flat_data.to_vec(),
    }
}

fn pool_sequence_by_attention(
    flattened: &[f32],
    seq_len: usize,
    hidden: usize,
    attention_mask: &[i64],
) -> Vec<f32> {
    let mut pooled = vec![0.0; hidden];
    let mut count = 0usize;

    for token_index in 0..seq_len {
        let use_token = attention_mask.get(token_index).copied().unwrap_or_default();
        if use_token == 0 {
            continue;
        }

        count += 1;
        let base_index = token_index * hidden;
        for hidden_index in 0..hidden {
            pooled[hidden_index] += flattened[base_index + hidden_index];
        }
    }

    if count == 0 {
        return pooled;
    }

    let inverse = 1.0f32 / count as f32;
    for value in &mut pooled {
        *value *= inverse;
    }
    pooled
}

fn pad_or_truncate(src: &[f32], desired: usize) -> Vec<f32> {
    let mut dst = vec![0.0; desired];
    let copy_len = src.len().min(desired);
    dst[..copy_len].copy_from_slice(&src[..copy_len]);
    dst
}

fn decode_le_f32_bytes(bytes: &[u8]) -> std::result::Result<Vec<f32>, String> {
    let chunks = bytes.chunks_exact(std::mem::size_of::<f32>());
    if !chunks.remainder().is_empty() {
        return Err("embedding bytes length is not divisible by 4".to_string());
    }

    Ok(chunks
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn shape_to_usize_vec(shape: &ort::value::Shape) -> Result<Vec<usize>> {
    shape
        .iter()
        .map(|dimension| {
            usize::try_from(*dimension).map_err(|_| {
                MaenifoldError::Message(format!(
                    "negative or unsupported tensor dimension encountered: {dimension}"
                ))
            })
        })
        .collect()
}

fn zero_embedding_vector() -> Vec<f32> {
    vec![0.0; embedding_dimensions().unwrap_or(DEFAULT_EMBEDDING_DIMENSIONS)]
}

fn map_ort_error(error: ort::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("onnx runtime error: {error}"))
}

const fn default_true() -> bool {
    true
}

const fn default_model_max_length() -> usize {
    DEFAULT_MAX_TOKENS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
        let dot = left
            .iter()
            .zip(right.iter())
            .map(|(left, right)| left * right)
            .sum::<f32>();
        let left_mag = left.iter().map(|value| value * value).sum::<f32>().sqrt();
        let right_mag = right.iter().map(|value| value * value).sum::<f32>().sqrt();
        if left_mag == 0.0 || right_mag == 0.0 {
            return 0.0;
        }
        dot / (left_mag * right_mag)
    }

    #[test]
    fn vector_generate_embedding_hello_world() {
        cleanup();
        load_model().expect("model loads");
        let embedding = generate_embedding("hello world").expect("embedding generated");
        assert_eq!(
            embedding.len(),
            embedding_dimensions().unwrap_or(DEFAULT_EMBEDDING_DIMENSIONS)
        );
        assert!(
            embedding.iter().any(|value| *value != 0.0),
            "hello world embedding should not be all zeros"
        );
        let self_similarity = cosine_similarity(&embedding, &embedding);
        assert!(
            (self_similarity - 1.0).abs() < 1e-5,
            "self cosine similarity should be ~1.0, got {self_similarity}"
        );
    }

    #[test]
    fn vector_batch_matches_individual() {
        cleanup();
        let text = "hello world";
        let individual = generate_embedding(text).expect("individual embedding");
        let batched = generate_embeddings(&[text], DEFAULT_MAX_BATCH).expect("batch embedding");
        assert_eq!(batched.len(), 1);
        assert_eq!(
            batched[0].len(),
            embedding_dimensions().unwrap_or(DEFAULT_EMBEDDING_DIMENSIONS)
        );
        for (index, (left, right)) in individual.iter().zip(batched[0].iter()).enumerate() {
            assert!(
                (left - right).abs() <= 1e-6,
                "batch result diverged at dimension {index}: {left} vs {right}"
            );
        }
    }

    #[test]
    fn vector_base64_round_trip_preserves_embedding() {
        cleanup();
        let embedding = generate_embedding("hello world").expect("embedding generated");
        let encoded = generate_embedding_base64("hello world").expect("base64 generated");
        let decoded = decode_base64_embedding(&encoded);
        assert_eq!(decoded.len(), embedding.len());
        for (index, (left, right)) in embedding.iter().zip(decoded.iter()).enumerate() {
            assert!(
                (left - right).abs() <= f32::EPSILON,
                "base64 round-trip diverged at dimension {index}: {left} vs {right}"
            );
        }
    }
}
