#[path = "dynamic_model/support.rs"]
mod dynamic_model_support;

use dynamic_model_support::{TEST_MUTEX, TestContext, packaged_model_fixture_available};
use maenifold_core::vector;
use maenifold_core::vector::DEFAULT_MAX_TOKENS;

// T-10.11: RTM FR-18 / RTM-098 — when both tokenizer.json and manual fallback assets exist,
// runtime loading must prefer tokenizer.json rather than rebuilding from vocab.txt.
#[test]
fn t10_11_tokenizer_json_is_preferred_over_vocab_txt() {
    let _guard = TEST_MUTEX.lock();
    if !packaged_model_fixture_available() {
        eprintln!("Skipping dynamic model test: packaged ONNX fixture is unavailable");
        return;
    }
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("preferred-tokenizer.onnx");
    ctx.write_vocab("this is intentionally not a valid wordpiece vocab");
    ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]","model_max_length":96}"#);
    ctx.write_tokenizer_json();

    // Primary assertion: load_model() succeeding here proves tokenizer.json was used,
    // because the deliberately-broken vocab.txt would cause WordPiece::from_file to panic.
    vector::load_model().expect("load model via tokenizer.json");

    assert_eq!(
        vector::model_filename().as_deref(),
        Some("preferred-tokenizer.onnx")
    );
    assert_eq!(vector::max_tokens(), Some(96));
    assert_eq!(vector::embedding_dimensions(), Some(384));
    assert_eq!(
        vector::generate_embedding("tokenizer preference")
            .expect("embedding from tokenizer.json")
            .len(),
        384
    );
}

// T-10.11: RTM FR-18 / RTM-099a — explicit model_max_length in tokenizer_config.json is respected.
#[test]
fn t10_11_max_tokens_reads_model_max_length_from_config() {
    let _guard = TEST_MUTEX.lock();
    if !packaged_model_fixture_available() {
        eprintln!("Skipping dynamic model test: packaged ONNX fixture is unavailable");
        return;
    }
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("configured-max-tokens.onnx");
    ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]","model_max_length":128}"#);
    ctx.write_tokenizer_json();

    vector::load_model().expect("load configured max tokens model");
    assert_eq!(vector::max_tokens(), Some(128));
}

// T-10.11: RTM FR-18 / RTM-099b — absent model_max_length falls back to DEFAULT_MAX_TOKENS.
#[test]
fn t10_11_max_tokens_defaults_when_field_absent() {
    let _guard = TEST_MUTEX.lock();
    if !packaged_model_fixture_available() {
        eprintln!("Skipping dynamic model test: packaged ONNX fixture is unavailable");
        return;
    }
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("default-max-tokens.onnx");
    ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]"}"#);
    ctx.write_tokenizer_json();

    vector::load_model().expect("load default max tokens model");
    assert_eq!(vector::max_tokens(), Some(DEFAULT_MAX_TOKENS));
}

// T-10.11: RTM FR-18 / RTM-099c — model_max_length: 0 is treated as absent and falls back
// to DEFAULT_MAX_TOKENS. The tokenizer loading logic explicitly guards against zero values
// with `.filter(|max_tokens| *max_tokens > 0)`.
#[test]
fn t10_11_max_tokens_treats_zero_as_absent() {
    let _guard = TEST_MUTEX.lock();
    if !packaged_model_fixture_available() {
        eprintln!("Skipping dynamic model test: packaged ONNX fixture is unavailable");
        return;
    }
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("zero-max-tokens.onnx");
    ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]","model_max_length":0}"#);
    ctx.write_tokenizer_json();

    vector::load_model().expect("load model with zero model_max_length");
    assert_eq!(vector::max_tokens(), Some(DEFAULT_MAX_TOKENS));
}
