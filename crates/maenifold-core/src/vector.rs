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

pub const EMBEDDING_DIMENSIONS: usize = 384;
pub const MAX_TOKENS: usize = 512;

const DEFAULT_MAX_BATCH: usize = 32;
const MODEL_FILENAME: &str = "all-MiniLM-L6-v2.onnx";
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
}

#[derive(Debug, Clone)]
struct TokenizerAssets {
    tokenizer: Tokenizer,
    pad_id: u32,
    unknown_id: u32,
    class_id: u32,
    separator_id: u32,
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

    let mut results = vec![zero_embedding_vector(); texts.len()];
    let non_empty = texts
        .iter()
        .enumerate()
        .filter_map(|(index, text)| {
            if text.trim().is_empty() {
                None
            } else {
                Some((index, *text))
            }
        })
        .collect::<Vec<_>>();

    if non_empty.is_empty() {
        return Ok(results);
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

    for chunk in non_empty.chunks(effective_batch) {
        let batch_size = chunk.len();
        debug_assert_ne!(state.tokenizer.unknown_id, state.tokenizer.pad_id);
        debug_assert_ne!(state.tokenizer.class_id, state.tokenizer.separator_id);

        let mut input_ids = vec![i64::from(state.tokenizer.pad_id); batch_size * MAX_TOKENS];
        let mut attention_mask = vec![0_i64; batch_size * MAX_TOKENS];
        let mut token_type_ids = vec![0_i64; batch_size * MAX_TOKENS];

        for (batch_index, (_, text)) in chunk.iter().enumerate() {
            let encoding = state
                .tokenizer
                .tokenizer
                .encode(*text, true)
                .map_err(|error| {
                    MaenifoldError::Message(format!("tokenizer encode failed: {error}"))
                })?;

            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let types = encoding.get_type_ids();

            let max_copy = ids.len().min(MAX_TOKENS);
            let row_offset = batch_index * MAX_TOKENS;
            for token_index in 0..max_copy {
                input_ids[row_offset + token_index] = i64::from(ids[token_index]);
                attention_mask[row_offset + token_index] = i64::from(mask[token_index]);
                token_type_ids[row_offset + token_index] = i64::from(types[token_index]);
            }
        }

        let outputs = state
            .session
            .run(ort::inputs! {
                "input_ids" => Tensor::from_array(([batch_size, MAX_TOKENS], input_ids.clone().into_boxed_slice())).map_err(map_ort_error)?,
                "attention_mask" => Tensor::from_array(([batch_size, MAX_TOKENS], attention_mask.clone().into_boxed_slice())).map_err(map_ort_error)?,
                "token_type_ids" => Tensor::from_array(([batch_size, MAX_TOKENS], token_type_ids.into_boxed_slice())).map_err(map_ort_error)?,
            })
            .map_err(map_ort_error)?;

        let chosen = select_best_output(&outputs)?;
        let (dims, flat_data) = extract_output_data(&outputs, &chosen)?;

        for (batch_index, (result_index, _)) in chunk.iter().enumerate() {
            let embedding = extract_embedding_from_output(
                &dims,
                &flat_data,
                batch_index,
                &attention_mask[(batch_index * MAX_TOKENS)..((batch_index + 1) * MAX_TOKENS)],
            );
            results[*result_index] = pad_or_truncate(&embedding, EMBEDDING_DIMENSIONS);
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
    let vocab_path = find_vocab_path()?;
    let tokenizer_config_path = find_tokenizer_config_path()?;
    let session = Session::builder()
        .map_err(map_ort_error)?
        .commit_from_file(&model_path)
        .map_err(map_ort_error)?;
    let tokenizer = load_tokenizer_assets(&vocab_path, &tokenizer_config_path)?;
    *guard = Some(ModelState { session, tokenizer });
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

fn model_state() -> &'static Mutex<Option<ModelState>> {
    MODEL_STATE.get_or_init(|| Mutex::new(None))
}

fn find_model_path() -> Result<PathBuf> {
    find_asset_path(MODEL_FILENAME)
}

fn find_vocab_path() -> Result<PathBuf> {
    find_asset_path(VOCAB_FILENAME)
}

fn find_tokenizer_config_path() -> Result<PathBuf> {
    find_asset_path(TOKENIZER_CONFIG_FILENAME)
}

fn find_asset_path(file_name: &str) -> Result<PathBuf> {
    let executable = env::current_exe()?;
    let base_dir = executable
        .parent()
        .ok_or(MaenifoldError::MissingExecutableParent)?;

    let direct_path = base_dir.join("assets").join("models").join(file_name);
    if direct_path.exists() {
        return Ok(direct_path);
    }

    for ancestor in base_dir.ancestors() {
        let workspace_manifest = ancestor.join("Cargo.toml");
        let candidate = ancestor.join("assets").join("models").join(file_name);
        if workspace_manifest.exists() && candidate.exists() {
            return Ok(candidate);
        }
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fallback = manifest_dir.join("../../assets/models").join(file_name);
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(MaenifoldError::Message(format!(
        "embedding asset not found: {file_name}"
    )))
}

fn load_tokenizer_assets(vocab_path: &Path, config_path: &Path) -> Result<TokenizerAssets> {
    let config_text = fs::read_to_string(config_path)?;
    let config: BertTokenizerConfig = serde_json::from_str(&config_text)?;
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

    let pad_token = config.pad_token.unwrap_or_else(|| "[PAD]".to_string());
    let unk_token = config.unk_token.unwrap_or_else(|| "[UNK]".to_string());
    let cls_token = config.cls_token.unwrap_or_else(|| "[CLS]".to_string());
    let sep_token = config.sep_token.unwrap_or_else(|| "[SEP]".to_string());
    let mask_token = config.mask_token.unwrap_or_else(|| "[MASK]".to_string());
    let model_dir = config_path.parent().ok_or_else(|| {
        MaenifoldError::Message("tokenizer config has no parent directory".to_string())
    })?;

    let mut tokenizer = {
        let tokenizer_json = model_dir.join(TOKENIZER_JSON_FILENAME);
        if tokenizer_json.exists() {
            Tokenizer::from_file(&tokenizer_json).map_err(|error| {
                MaenifoldError::Message(format!("tokenizer.json load failed: {error}"))
            })?
        } else {
            build_wordpiece_tokenizer(
                vocab_path,
                config.do_lower_case.unwrap_or(true),
                config.tokenize_chinese_chars,
                config.strip_accents,
                &pad_token,
                &unk_token,
                &cls_token,
                &sep_token,
            )?
        }
    };

    tokenizer.add_special_tokens(&[
        AddedToken::from(pad_token.clone(), true),
        AddedToken::from(unk_token.clone(), true),
        AddedToken::from(cls_token.clone(), true),
        AddedToken::from(sep_token.clone(), true),
        AddedToken::from(mask_token, true),
    ]);

    let pad_id = tokenizer
        .token_to_id(&pad_token)
        .ok_or_else(|| MaenifoldError::Message("missing tokenizer [PAD] token id".to_string()))?;
    let unknown_id = tokenizer
        .token_to_id(&unk_token)
        .ok_or_else(|| MaenifoldError::Message("missing tokenizer [UNK] token id".to_string()))?;
    let class_id = tokenizer
        .token_to_id(&cls_token)
        .ok_or_else(|| MaenifoldError::Message("missing tokenizer [CLS] token id".to_string()))?;
    let separator_id = tokenizer
        .token_to_id(&sep_token)
        .ok_or_else(|| MaenifoldError::Message("missing tokenizer [SEP] token id".to_string()))?;

    tokenizer
        .with_truncation(Some(TruncationParams {
            max_length: config.model_max_length.min(MAX_TOKENS),
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
            ..Default::default()
        }))
        .map_err(|error| {
            MaenifoldError::Message(format!("tokenizer truncation setup failed: {error}"))
        })?;
    tokenizer.with_padding(Some(PaddingParams {
        strategy: PaddingStrategy::Fixed(MAX_TOKENS),
        pad_id,
        pad_type_id: 0,
        pad_token: pad_token.clone(),
        ..Default::default()
    }));

    Ok(TokenizerAssets {
        tokenizer,
        pad_id,
        unknown_id,
        class_id,
        separator_id,
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
        .find(|candidate| candidate.dims.len() == 2 && candidate.dims[0] == 1)
    {
        return Ok(candidate.clone());
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.dims.len() == 1)
    {
        return Ok(candidate.clone());
    }

    Ok(candidates
        .into_iter()
        .next()
        .expect("candidate list already checked as non-empty"))
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
            let start = batch_index * hidden;
            let end = start + hidden;
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
            let start = batch_index * EMBEDDING_DIMENSIONS;
            let end = start + EMBEDDING_DIMENSIONS.min(*hidden);
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
    vec![0.0; EMBEDDING_DIMENSIONS]
}

fn map_ort_error(error: ort::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("onnx runtime error: {error}"))
}

const fn default_true() -> bool {
    true
}

const fn default_model_max_length() -> usize {
    MAX_TOKENS
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
        assert_eq!(embedding.len(), EMBEDDING_DIMENSIONS);
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
        assert_eq!(batched[0].len(), EMBEDDING_DIMENSIONS);
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
