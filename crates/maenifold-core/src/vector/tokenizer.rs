use super::{
    BertTokenizerConfig, TokenizerAssets, DEFAULT_MAX_TOKENS, TOKENIZER_CONFIG_FILENAME,
    TOKENIZER_JSON_FILENAME,
};
use crate::error::{MaenifoldError, Result};
use std::fs;
use std::path::Path;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};

pub(crate) fn load_tokenizer_assets(model_dir: &Path) -> Result<(TokenizerAssets, usize)> {
    let tokenizer_config_path = model_dir.join(TOKENIZER_CONFIG_FILENAME);
    let config = load_tokenizer_config(&tokenizer_config_path)?;
    let max_tokens = config
        .as_ref()
        .map(|config| config.model_max_length)
        .filter(|max_tokens| *max_tokens > 0)
        .unwrap_or(DEFAULT_MAX_TOKENS);

    let tokenizer_json = model_dir.join(TOKENIZER_JSON_FILENAME);
    if !tokenizer_json.is_file() {
        return Err(MaenifoldError::Message(format!(
            "Model at {} is missing tokenizer.json. Curated models must include tokenizer.json — re-download the model or select a curated model.",
            model_dir.display()
        )));
    }
    let mut tokenizer = Tokenizer::from_file(&tokenizer_json)
        .map_err(|error| MaenifoldError::Message(format!("tokenizer.json load failed: {error}")))?;

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
