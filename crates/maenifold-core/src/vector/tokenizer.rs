use super::{
    BertTokenizerConfig, ResolvedMaxTokens, TOKENIZER_CONFIG_FILENAME, TOKENIZER_JSON_FILENAME,
    TokenizerAssets,
};
use crate::error::{MaenifoldError, Result};
use crate::tools::embedding_models::CatalogModel;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tokenizers::{Tokenizer, TruncationParams, TruncationStrategy};

const MODEL_CONFIG_FILENAME: &str = "config.json";

#[derive(Debug, Clone, Deserialize)]
struct LocalModelConfig {
    #[serde(default)]
    max_position_embeddings: Option<usize>,
    #[serde(default)]
    n_positions: Option<usize>,
    #[serde(default)]
    max_seq_len: Option<usize>,
    #[serde(default)]
    max_sequence_length: Option<usize>,
    #[serde(default)]
    seq_length: Option<usize>,
    #[serde(default)]
    context_length: Option<usize>,
}

pub(crate) fn load_tokenizer_assets(
    model_dir: &Path,
    model_id: &str,
    catalog_model: &CatalogModel,
) -> Result<(TokenizerAssets, ResolvedMaxTokens)> {
    let tokenizer_config_path = model_dir.join(TOKENIZER_CONFIG_FILENAME);
    let tokenizer_config = load_tokenizer_config(&tokenizer_config_path)?;
    let max_tokens = resolve_max_tokens(
        model_dir,
        model_id,
        catalog_model.max_tokens,
        tokenizer_config
            .as_ref()
            .and_then(|config| config.model_max_length)
            .filter(|value| *value > 0),
    )?;

    let tokenizer_json = model_dir.join(TOKENIZER_JSON_FILENAME);
    if !tokenizer_json.is_file() {
        return Err(MaenifoldError::Message(
            "Active embedding model is missing tokenizer.json. Re-download the model or select a curated model.".to_string()
        ));
    }
    let mut tokenizer = Tokenizer::from_file(&tokenizer_json)
        .map_err(|error| MaenifoldError::Message(format!("tokenizer.json load failed: {error}")))?;

    let pad_token = tokenizer
        .get_padding()
        .map(|padding| padding.pad_token.clone())
        .or_else(|| {
            tokenizer_config
                .as_ref()
                .and_then(|config| config.pad_token.clone())
        })
        .unwrap_or_else(|| "[PAD]".to_string());
    let pad_id = tokenizer
        .get_padding()
        .map(|padding| padding.pad_id)
        .or_else(|| tokenizer.token_to_id(&pad_token))
        .or_else(|| tokenizer.token_to_id("<pad>"))
        .ok_or_else(|| {
            MaenifoldError::Message(
                "Active embedding model tokenizer has no pad token id. Re-download the model or select a curated model.".to_string()
            )
        })?;

    tokenizer
        .with_truncation(Some(TruncationParams {
            max_length: max_tokens.value,
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
            ..Default::default()
        }))
        .map_err(|error| {
            MaenifoldError::Message(format!("tokenizer truncation setup failed: {error}"))
        })?;
    tokenizer.with_padding(None);

    Ok((TokenizerAssets { tokenizer, pad_id }, max_tokens))
}

pub(crate) fn resolve_max_tokens(
    model_dir: &Path,
    model_id: &str,
    catalog_max_tokens: usize,
    tokenizer_hint: Option<usize>,
) -> Result<ResolvedMaxTokens> {
    let config_path = model_dir.join(MODEL_CONFIG_FILENAME);
    let trusted = load_local_model_limit(&config_path)?
        .map(|(field, value)| (format!("config.json:{field}"), value))
        .or_else(|| {
            (catalog_max_tokens > 0).then(|| ("catalog:maxTokens".to_string(), catalog_max_tokens))
        })
        .ok_or_else(|| {
            let tokenizer_source = tokenizer_hint
                .map(|value| format!("tokenizer_config.json:model_max_length={value}"))
                .unwrap_or_else(|| "tokenizer_config.json:model_max_length=missing".to_string());
            MaenifoldError::Message(format!(
                "embedding model '{model_id}' has no trusted max token source; {tokenizer_source}"
            ))
        })?;

    let (trusted_source, trusted_value) = trusted;
    let (value, source_tag) = match tokenizer_hint {
        Some(hint) if hint > 0 && hint < trusted_value => (
            hint,
            format!("{trusted_source}+tokenizer_config.json:model_max_length"),
        ),
        _ => (trusted_value, trusted_source),
    };

    Ok(ResolvedMaxTokens { value, source_tag })
}

fn load_local_model_limit(config_path: &Path) -> Result<Option<(&'static str, usize)>> {
    if !config_path.is_file() {
        return Ok(None);
    }

    let config_text = fs::read_to_string(config_path)?;
    let config: LocalModelConfig = serde_json::from_str(&config_text)?;
    Ok([
        ("max_position_embeddings", config.max_position_embeddings),
        ("n_positions", config.n_positions),
        ("max_seq_len", config.max_seq_len),
        ("max_sequence_length", config.max_sequence_length),
        ("seq_length", config.seq_length),
        ("context_length", config.context_length),
    ]
    .into_iter()
    .find_map(|(field, value)| value.filter(|limit| *limit > 0).map(|limit| (field, limit))))
}

fn load_tokenizer_config(config_path: &Path) -> Result<Option<BertTokenizerConfig>> {
    if !config_path.is_file() {
        return Ok(None);
    }

    let config_text = fs::read_to_string(config_path)?;
    let config: BertTokenizerConfig = serde_json::from_str(&config_text)?;
    Ok(Some(config))
}
