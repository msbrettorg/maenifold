use super::{
    BertTokenizerConfig, DEFAULT_MAX_TOKENS, TOKENIZER_CONFIG_FILENAME, TOKENIZER_JSON_FILENAME,
    TokenizerAssets, VOCAB_FILENAME,
};
use crate::error::{MaenifoldError, Result};
use std::fs;
use std::path::Path;
use tokenizers::{
    AddedToken, PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy,
    models::wordpiece::WordPiece, normalizers::bert::BertNormalizer,
    pre_tokenizers::bert::BertPreTokenizer, processors::template::TemplateProcessing,
};

pub(crate) fn load_tokenizer_assets(model_dir: &Path) -> Result<(TokenizerAssets, usize)> {
    let tokenizer_config_path = model_dir.join(TOKENIZER_CONFIG_FILENAME);
    let config = load_tokenizer_config(&tokenizer_config_path)?;
    let max_tokens = config
        .as_ref()
        .map(|config| config.model_max_length)
        .filter(|max_tokens| *max_tokens > 0)
        .unwrap_or(DEFAULT_MAX_TOKENS);

    // T-9.1: RTM FR-7 — prefer HuggingFace `tokenizer.json` assets and only rebuild
    // manual BERT WordPiece tokenizers when [[dynamic model loading]] needs the fallback path.
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
